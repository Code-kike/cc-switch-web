use axum::{
    extract::State,
    routing::{get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::net::{Ipv4Addr, SocketAddrV4, TcpStream};
use std::time::{Duration, Instant};

use super::super::ApiState;
use super::common::{json_ok, ApiError, ApiResult};

#[derive(Deserialize)]
struct UrlQuery {
    url: String,
}

#[derive(Deserialize)]
struct SetUrlRequest {
    url: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProxyTestResult {
    success: bool,
    latency_ms: u64,
    error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct UpstreamProxyStatus {
    enabled: bool,
    proxy_url: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DetectedProxy {
    url: String,
    proxy_type: String,
    port: u16,
}

const PROXY_PORTS: &[(u16, &str, bool)] = &[
    (7890, "http", true),
    (7891, "socks5", false),
    (1080, "socks5", false),
    (8080, "http", false),
    (8888, "http", false),
    (3128, "http", false),
    (10808, "socks5", false),
    (10809, "http", false),
];

pub fn router(state: ApiState) -> Router {
    Router::new()
        .route(
            "/global-proxy/get-global-proxy-url",
            get(get_global_proxy_url),
        )
        .route(
            "/global-proxy/set-global-proxy-url",
            put(set_global_proxy_url),
        )
        .route("/proxy/test-proxy-url", post(test_proxy_url))
        .route(
            "/proxy/get-upstream-proxy-status",
            get(get_upstream_proxy_status),
        )
        .route("/system/scan_local_proxies", post(scan_local_proxies))
        .with_state(state)
}

async fn get_global_proxy_url(State(state): State<ApiState>) -> ApiResult<Option<String>> {
    let url = state
        .app_state
        .db
        .get_global_proxy_url()
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(url))
}

async fn set_global_proxy_url(
    State(state): State<ApiState>,
    Json(request): Json<SetUrlRequest>,
) -> ApiResult<()> {
    let url = request.url.trim();
    let url_opt = if url.is_empty() { None } else { Some(url) };
    crate::proxy::http_client::validate_proxy(url_opt).map_err(ApiError::from_service_message)?;
    state
        .app_state
        .db
        .set_global_proxy_url(url_opt)
        .map_err(ApiError::from_anyhow)?;
    crate::proxy::http_client::apply_proxy(url_opt).map_err(ApiError::from_service_message)?;
    Ok(json_ok(()))
}

async fn test_proxy_url(Json(request): Json<UrlQuery>) -> ApiResult<ProxyTestResult> {
    if request.url.trim().is_empty() {
        return Err(ApiError::bad_request("Proxy URL is empty"));
    }

    let start = Instant::now();
    let proxy = reqwest::Proxy::all(&request.url)
        .map_err(|err| ApiError::bad_request(format!("Invalid proxy URL: {err}")))?;
    let client = reqwest::Client::builder()
        .proxy(proxy)
        .timeout(Duration::from_secs(10))
        .connect_timeout(Duration::from_secs(10))
        .build()
        .map_err(ApiError::from_anyhow)?;

    let mut last_error = None;
    for test_url in [
        "https://httpbin.org/get",
        "https://www.google.com",
        "https://api.anthropic.com",
    ] {
        match client.head(test_url).send().await {
            Ok(_) => {
                return Ok(json_ok(ProxyTestResult {
                    success: true,
                    latency_ms: start.elapsed().as_millis() as u64,
                    error: None,
                }))
            }
            Err(err) => last_error = Some(err.to_string()),
        }
    }

    Ok(json_ok(ProxyTestResult {
        success: false,
        latency_ms: start.elapsed().as_millis() as u64,
        error: last_error,
    }))
}

async fn get_upstream_proxy_status() -> ApiResult<UpstreamProxyStatus> {
    let url = crate::proxy::http_client::get_current_proxy_url();
    Ok(json_ok(UpstreamProxyStatus {
        enabled: url.is_some(),
        proxy_url: url,
    }))
}

async fn scan_local_proxies() -> ApiResult<Vec<DetectedProxy>> {
    let found = tokio::task::spawn_blocking(|| {
        let mut found = Vec::new();
        for &(port, primary_type, is_mixed) in PROXY_PORTS {
            let addr = SocketAddrV4::new(Ipv4Addr::LOCALHOST, port);
            if TcpStream::connect_timeout(&addr.into(), Duration::from_millis(100)).is_ok() {
                found.push(DetectedProxy {
                    url: format!("{primary_type}://127.0.0.1:{port}"),
                    proxy_type: primary_type.to_string(),
                    port,
                });
                if is_mixed {
                    let alt_type = if primary_type == "http" {
                        "socks5"
                    } else {
                        "http"
                    };
                    found.push(DetectedProxy {
                        url: format!("{alt_type}://127.0.0.1:{port}"),
                        proxy_type: alt_type.to_string(),
                        port,
                    });
                }
            }
        }
        found
    })
    .await
    .map_err(ApiError::from_anyhow)?;

    Ok(json_ok(found))
}
