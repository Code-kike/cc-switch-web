//! 使用统计实时刷新事件模块（双运行时适配版）
//!
//! 当 `proxy_request_logs` 表写入新数据时（代理日志、会话同步、归档等），
//! 通过本模块向前端 emit `usage-log-recorded` 事件，让 UsageDashboard
//! 立刻 invalidate 查询缓存而无需等待轮询周期。
//!
//! 设计要点（与上游的差异）：
//! - **上游**直接持有 `tauri::AppHandle` 并 `AppHandle::emit`。本 Web fork 不能
//!   硬编码 Tauri emit，否则 web-server 模式拿不到实时刷新。
//! - 因此本模块持有的是运行时中立的 `Arc<dyn UiEventSink>`（来自 `crate::runtime`）：
//!     - 桌面端：`lib.rs` 注入 `TauriEventSink` → Tauri 事件总线。
//!     - Web 端：`examples/server.rs` 注入 `ChannelEventSink` → broadcast →
//!       `GET /api/events` SSE。
//! - 全局单例 sink：写日志路径上不持有 sink/AppHandle，用 `OnceLock` 共享，
//!   调用方无需任何签名改动即可通知。
//! - 200ms 防抖合并：流式响应、批量会话同步等场景在短时间内可能写入多条日志，
//!   合并成一次事件可避免前端连续 invalidate（一次 Codex 同步导入 3000+ 条只会
//!   触发约 2 次 emit，而不是 3000 次）。
//! - 不阻塞写入：通知失败仅记录 warn 日志，不向上传播错误。

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use crate::runtime::UiEventSink;

/// 前端监听的事件名
pub const EVENT_USAGE_LOG_RECORDED: &str = "usage-log-recorded";

/// 防抖窗口：合并 200ms 内的多次通知。
const DEBOUNCE_WINDOW: Duration = Duration::from_millis(200);

/// 运行时中立的事件 sink。桌面端为 `TauriEventSink`，Web 端为 `ChannelEventSink`。
static EVENT_SINK: OnceLock<Arc<dyn UiEventSink>> = OnceLock::new();

/// 防抖标记：true 表示已有调度任务在等待 emit，后续通知合并到该任务。
static EMIT_SCHEDULED: AtomicBool = AtomicBool::new(false);

/// 在应用启动阶段调用一次，注入运行时对应的事件 sink。
///
/// 重复调用是无害的（`OnceLock` 仅首次写入生效）。两种运行时各自调用一次：
/// - 桌面端：`lib.rs::run` 的 setup 阶段传入 `TauriEventSink`。
/// - Web 端：`examples/server.rs` 传入与 `ApiState` 共享的 `ChannelEventSink`。
pub fn init(sink: Arc<dyn UiEventSink>) {
    if EVENT_SINK.set(sink).is_err() {
        log::debug!("usage_events::init 重复调用，已忽略");
    } else {
        log::info!("[usage-event] 事件 sink 已注入，实时推送启用");
    }
}

/// 通知前端有新的使用日志写入。
///
/// 调用方**不**需要持有 sink/AppHandle，可以从任意线程/任意写入路径调用
/// （代理日志、Claude/Codex/Gemini 会话同步、启动归档）。
/// 内部 200ms 防抖合并，绝不阻塞调用线程。
pub fn notify_log_recorded() {
    #[cfg(test)]
    TEST_NOTIFY_COUNT.with(|count| count.set(count.get().saturating_add(1)));

    // sink 未注入（典型出现在单元测试或 init 之前）：直接放弃。
    let Some(sink) = EVENT_SINK.get() else {
        return;
    };

    // 已有调度任务：本次通知被合并到既有任务里，无需再起线程。
    if EMIT_SCHEDULED.swap(true, Ordering::AcqRel) {
        return;
    }

    let sink = Arc::clone(sink);
    std::thread::spawn(move || {
        std::thread::sleep(DEBOUNCE_WINDOW);
        // 必须先清标志再 emit：万一 emit 期间又有新通知进来，
        // 下一轮防抖窗口会重新调度，不会丢失。
        EMIT_SCHEDULED.store(false, Ordering::Release);

        // 通过运行时中立的 sink 推送。桌面端走 Tauri 事件总线，
        // Web 端走 broadcast -> /api/events SSE。payload 为空（前端只需
        // invalidate，无需具体数据）。
        sink.emit_json(EVENT_USAGE_LOG_RECORDED, serde_json::Value::Null);
    });
}

#[cfg(test)]
thread_local! {
    static TEST_NOTIFY_COUNT: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
pub(crate) fn take_test_notify_count() -> u32 {
    TEST_NOTIFY_COUNT.with(|count| count.replace(0))
}
