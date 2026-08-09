//! 模型列表获取命令
//!
//! 提供 Tauri 命令，供前端在供应商表单中获取可用模型列表。

use crate::services::model_fetch::{self, FetchedModel, OpenCodeModelRef};

/// 获取 OpenCode 当前运行时可用的模型。
///
/// 复用工具更新页的 CLI 定位逻辑执行 `opencode models`，因此会包含 OpenCode
/// 已加载的 OAuth 模型与 Zen 免费模型，而不是只读取 opencode.json。
#[tauri::command]
pub async fn get_opencode_models() -> Result<Vec<OpenCodeModelRef>, String> {
    model_fetch::get_opencode_models().await
}

/// 获取供应商的可用模型列表
///
/// 使用 OpenAI 兼容的 GET /v1/models 端点。优先使用 `models_url` 精确覆写；
/// 否则对 baseURL 生成候选列表（含「剥离 Anthropic 兼容子路径」兜底），按序尝试。
#[tauri::command(rename_all = "camelCase")]
pub async fn fetch_models_for_config(
    base_url: String,
    api_key: String,
    is_full_url: Option<bool>,
    models_url: Option<String>,
) -> Result<Vec<FetchedModel>, String> {
    model_fetch::fetch_models(
        &base_url,
        &api_key,
        is_full_url.unwrap_or(false),
        models_url.as_deref(),
    )
    .await
}
