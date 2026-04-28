use crate::config::get_user_agent;
use crate::AppState;
use axum::body::Body;
use axum::extract::{Query, State};
use axum::http::{header, HeaderMap, HeaderValue, Response, StatusCode};
use axum::routing::get;
use axum::Router;
use futures::StreamExt;
use serde::Deserialize;
use std::net::SocketAddr;
use std::path::PathBuf;
use tokio::net::TcpListener;
use tower_http::services::ServeDir;
use url::Url;

pub const MEDIA_PROXY_PORT: u16 = 39143;
const INITIAL_VIDEO_RANGE: &str = "bytes=0-1048575";
const MAX_RETRIES: usize = 3;

#[derive(Debug, Deserialize)]
struct MediaProxyQuery {
    url: String,
    media_type: Option<String>,
}

fn host_matches(host: &str, allowed_domain: &str) -> bool {
    host == allowed_domain || host.ends_with(&format!(".{}", allowed_domain))
}

fn is_allowed_media_url(url: &Url) -> bool {
    if url.scheme() != "https" && url.scheme() != "http" {
        return false;
    }

    let Some(host) = url.host_str().map(|host| host.to_ascii_lowercase()) else {
        return false;
    };

    const ALLOWED_MEDIA_DOMAINS: &[&str] = &[
        "douyin.com",
        "douyinvod.com",
        "douyinpic.com",
        "douyinstatic.com",
        "byteimg.com",
        "ixigua.com",
        "amemv.com",
        "snssdk.com",
        "pstatp.com",
    ];

    ALLOWED_MEDIA_DOMAINS
        .iter()
        .any(|domain| host_matches(&host, domain))
}

fn should_send_cookie(url: &Url) -> bool {
    let Some(host) = url.host_str().map(|host| host.to_ascii_lowercase()) else {
        return false;
    };

    const COOKIE_DOMAINS: &[&str] = &["douyin.com", "amemv.com", "snssdk.com"];

    COOKIE_DOMAINS
        .iter()
        .any(|domain| host_matches(&host, domain))
}

fn allowed_request_origin(request_headers: &HeaderMap) -> Option<Option<HeaderValue>> {
    let Some(origin) = request_headers.get(header::ORIGIN) else {
        return Some(None);
    };

    let origin_str = origin.to_str().ok()?;
    let parsed = Url::parse(origin_str).ok()?;
    let scheme = parsed.scheme();
    let host = parsed.host_str()?.to_ascii_lowercase();
    let port = parsed.port_or_known_default();

    let allowed = (scheme == "http"
        && (host == "127.0.0.1" || host == "localhost")
        && port == Some(MEDIA_PROXY_PORT))
        || (scheme == "http" && host == "tauri.localhost")
        || (scheme == "tauri" && host == "localhost");

    if allowed {
        Some(Some(origin.clone()))
    } else {
        None
    }
}

fn build_error_response(status: StatusCode, message: &str) -> Response<Body> {
    Response::builder()
        .status(status)
        .body(Body::from(message.to_string()))
        .unwrap_or_else(|_| Response::new(Body::from(message.to_string())))
}

fn frontend_dist_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../dist")
}

fn resolve_redirect_target(current_url: &Url, location: &str) -> Option<String> {
    if let Ok(url) = Url::parse(location) {
        return Some(url.to_string());
    }
    current_url.join(location).ok().map(|url| url.to_string())
}

fn guess_content_type(
    url: &str,
    upstream_content_type: &str,
    requested_media_type: &str,
) -> Option<&'static str> {
    let normalized = upstream_content_type
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_lowercase();

    if requested_media_type == "audio" {
        if normalized.starts_with("audio/") {
            return Some("audio/mpeg");
        }
        if url.ends_with(".m4a") {
            return Some("audio/mp4");
        }
        return Some("audio/mpeg");
    }

    if !normalized.is_empty() && normalized != "application/octet-stream" {
        return None;
    }

    if url.contains(".mp4") || url.contains("/play/") || requested_media_type == "video" {
        return Some("video/mp4");
    }
    if url.contains(".jpg") || url.contains(".jpeg") {
        return Some("image/jpeg");
    }
    if url.contains(".png") {
        return Some("image/png");
    }
    if url.contains(".webp") {
        return Some("image/webp");
    }

    None
}

async fn media_proxy(
    State(state): State<AppState>,
    Query(query): Query<MediaProxyQuery>,
    request_headers: HeaderMap,
) -> Response<Body> {
    let requested_media_type = query
        .media_type
        .as_deref()
        .unwrap_or_default()
        .trim()
        .to_lowercase();
    let request_range = request_headers.get(header::RANGE).cloned();
    let request_range_str = request_range
        .as_ref()
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_string();
    let allow_origin = match allowed_request_origin(&request_headers) {
        Some(origin) => origin,
        None => return build_error_response(StatusCode::FORBIDDEN, "Forbidden"),
    };

    let parsed_url = match Url::parse(&query.url) {
        Ok(url) => url,
        Err(_) => return build_error_response(StatusCode::BAD_REQUEST, "Invalid URL"),
    };

    if query.url.is_empty() || !is_allowed_media_url(&parsed_url) {
        return build_error_response(StatusCode::BAD_REQUEST, "Invalid URL");
    }

    let config = state.config.lock().await.clone();
    let should_seed_video_range = request_range.is_none()
        && (requested_media_type == "video" || query.url.contains("/play/"));
    let upstream_range_value = if let Some(range) = &request_range {
        range.to_str().ok().map(|value| value.to_string())
    } else if should_seed_video_range {
        Some(INITIAL_VIDEO_RANGE.to_string())
    } else {
        None
    };
    let cache_key = if query.url.contains("/aweme/v1/play/") {
        Some(query.url.clone())
    } else {
        None
    };
    let cached_url = if let Some(key) = &cache_key {
        state.media_redirect_cache.lock().await.get(key).cloned()
    } else {
        None
    };
    let mut upstream_url = cached_url.clone().unwrap_or_else(|| query.url.clone());

    let start = std::time::Instant::now();
    let mut redirect_hops = 0usize;
    let mut retry_count = 0usize;
    let upstream_response = loop {
        let parsed_upstream_url = match Url::parse(&upstream_url) {
            Ok(url) if is_allowed_media_url(&url) => url,
            _ => {
                if let Some(key) = &cache_key {
                    state.media_redirect_cache.lock().await.remove(key);
                }
                return build_error_response(StatusCode::BAD_REQUEST, "Invalid URL");
            }
        };

        let mut upstream = state
            .media_http_client
            .get(&upstream_url)
            .header("User-Agent", get_user_agent())
            .header("Referer", "https://www.douyin.com/")
            .header("Accept", "*/*")
            .header("Accept-Encoding", "identity;q=1, *;q=0");

        if !config.cookie.is_empty() && should_send_cookie(&parsed_upstream_url) {
            upstream = upstream.header("Cookie", &config.cookie);
        }

        if let Some(range_value) = &upstream_range_value {
            upstream = upstream.header("Range", range_value);
        }

        match upstream.send().await {
            Ok(response) => {
                let status = response.status();

                // 处理重定向
                if status.is_redirection() {
                    let location = response
                        .headers()
                        .get(header::LOCATION)
                        .and_then(|value| value.to_str().ok())
                        .unwrap_or("");

                    if location.is_empty() || redirect_hops >= 4 {
                        break response;
                    }

                    if let Some(next_url) = resolve_redirect_target(response.url(), location) {
                        let next_parsed = match Url::parse(&next_url) {
                            Ok(url) if is_allowed_media_url(&url) => url,
                            _ => {
                                if let Some(key) = &cache_key {
                                    state.media_redirect_cache.lock().await.remove(key);
                                }
                                return build_error_response(
                                    StatusCode::BAD_REQUEST,
                                    "Invalid redirect URL",
                                );
                            }
                        };
                        redirect_hops += 1;
                        upstream_url = next_parsed.to_string();
                        continue;
                    }
                }

                // 处理服务器错误 (5xx)，尝试重试
                if status.is_server_error() && retry_count < MAX_RETRIES {
                    retry_count += 1;
                    log::warn!(
                        "media proxy upstream server error: status={} retry={}/{} url={}",
                        status,
                        retry_count,
                        MAX_RETRIES,
                        upstream_url.chars().take(80).collect::<String>()
                    );
                    tokio::time::sleep(tokio::time::Duration::from_millis(
                        500 * retry_count as u64,
                    ))
                    .await;
                    continue;
                }

                if let Some(key) = &cache_key {
                    if upstream_url != *key {
                        state
                            .media_redirect_cache
                            .lock()
                            .await
                            .insert(key.clone(), upstream_url.clone());
                    }
                }

                break response;
            }
            Err(error) => {
                // 网络错误，尝试重试
                if retry_count < MAX_RETRIES {
                    retry_count += 1;
                    log::warn!(
                        "media proxy network error, retrying: {:?} retry={}/{} url={}",
                        error,
                        retry_count,
                        MAX_RETRIES,
                        upstream_url.chars().take(80).collect::<String>()
                    );
                    tokio::time::sleep(tokio::time::Duration::from_millis(
                        500 * retry_count as u64,
                    ))
                    .await;
                    continue;
                }

                if let Some(key) = &cache_key {
                    state.media_redirect_cache.lock().await.remove(key);
                }
                log::error!(
                    "media proxy upstream request failed: {:?} elapsed={}ms seeded_range={} range=\"{}\" url={}",
                    error,
                    start.elapsed().as_millis(),
                    should_seed_video_range,
                    request_range_str,
                    upstream_url.chars().take(120).collect::<String>()
                );
                return build_error_response(StatusCode::BAD_GATEWAY, "Proxy error");
            }
        }
    };

    let status = upstream_response.status();
    let mut response_builder = Response::builder().status(status);
    let response_headers = match response_builder.headers_mut() {
        Some(h) => h,
        None => return build_error_response(StatusCode::BAD_GATEWAY, "Failed to build response"),
    };

    let copy_headers: [(axum::http::header::HeaderName, &str); 3] = [
        (header::CONTENT_TYPE, "content-type"),
        (header::CONTENT_RANGE, "content-range"),
        (header::ACCEPT_RANGES, "accept-ranges"),
    ];
    for (header_ref, header_name) in copy_headers {
        if let Some(value) = upstream_response.headers().get(header_name) {
            response_headers.insert(header_ref, value.clone());
        }
    }

    let upstream_content_type = upstream_response
        .headers()
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_string();

    let is_media = requested_media_type == "audio"
        || requested_media_type == "video"
        || upstream_content_type.to_lowercase().contains("video");

    if let Some(content_length) = upstream_response.headers().get("content-length") {
        let content_length_str = content_length.to_str().unwrap_or_default();
        if !content_length_str.is_empty() {
            match content_length_str.parse::<u64>() {
                Ok(length) => {
                    if length < 2 * 1024 * 1024 || !is_media {
                        response_headers.insert(header::CONTENT_LENGTH, content_length.clone());
                    }
                }
                Err(_) => {
                    response_headers.insert(header::CONTENT_LENGTH, content_length.clone());
                }
            }
        }
    }

    if let Some(content_type) =
        guess_content_type(&query.url, &upstream_content_type, &requested_media_type)
    {
        response_headers.insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
    }

    if (requested_media_type == "audio" || requested_media_type == "video")
        && !response_headers.contains_key(header::ACCEPT_RANGES)
    {
        response_headers.insert(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"));
    }

    response_headers.insert(
        header::ACCESS_CONTROL_ALLOW_ORIGIN,
        allow_origin.unwrap_or_else(|| HeaderValue::from_static("*")),
    );
    response_headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=3600"),
    );

    let stream = upstream_response
        .bytes_stream()
        .map(|result| result.map_err(std::io::Error::other));

    response_builder
        .body(Body::from_stream(stream))
        .unwrap_or_else(|_| build_error_response(StatusCode::BAD_GATEWAY, "Proxy error"))
}

pub async fn spawn_media_proxy(state: AppState) -> anyhow::Result<()> {
    let addr = SocketAddr::from(([127, 0, 0, 1], MEDIA_PROXY_PORT));
    let listener = TcpListener::bind(addr).await?;
    let dist_dir = frontend_dist_dir();

    log::info!(
        "local web server listening on http://{} (dist={})",
        addr,
        dist_dir.display()
    );

    tokio::spawn(async move {
        let app = Router::new()
            .route("/api/media/proxy", get(media_proxy))
            .fallback_service(ServeDir::new(dist_dir).append_index_html_on_directories(true))
            .with_state(state);

        if let Err(error) = axum::serve(listener, app).await {
            log::error!("local web server failed: {}", error);
        }
    });

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_media_url_by_host() {
        let allowed =
            Url::parse("https://v3-dy-o-abtest.zjcdn.com.douyinvod.com/video.mp4").unwrap();
        assert!(is_allowed_media_url(&allowed));

        let malicious = Url::parse("https://evil.example/?next=douyin.com/video.mp4").unwrap();
        assert!(!is_allowed_media_url(&malicious));

        let lookalike = Url::parse("https://douyin.com.evil.example/video.mp4").unwrap();
        assert!(!is_allowed_media_url(&lookalike));
    }

    #[test]
    fn only_sends_cookie_to_login_related_hosts() {
        let douyin = Url::parse("https://www.douyin.com/aweme/v1/play/").unwrap();
        assert!(should_send_cookie(&douyin));

        let cdn = Url::parse("https://example.douyinvod.com/video.mp4").unwrap();
        assert!(!should_send_cookie(&cdn));
    }

    #[test]
    fn validates_request_origin() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::ORIGIN,
            HeaderValue::from_static("http://127.0.0.1:39143"),
        );
        assert!(allowed_request_origin(&headers).is_some());

        headers.insert(
            header::ORIGIN,
            HeaderValue::from_static("https://evil.example"),
        );
        assert!(allowed_request_origin(&headers).is_none());
    }
}
