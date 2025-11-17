use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::get,
    Router,
};
use serde::{Deserialize, Serialize};
use std::env;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tracing::{error, info, warn};

#[derive(Clone)]
struct AppState {
    http_client: reqwest::Client,
}

#[derive(Debug, Deserialize)]
struct NextToken {
    next: Option<String>,
}

#[derive(Debug, Serialize)]
struct VideoResult {
    id: String,
    title: String,
    #[serde(rename = "thumbDefault")]
    thumb_default: String,
    #[serde(rename = "thumbMedium")]
    thumb_medium: String,
    #[serde(rename = "thumbHigh")]
    thumb_high: String,
    #[serde(rename = "channelTitle")]
    channel_title: String,
    #[serde(rename = "channelId")]
    channel_id: String,
    #[serde(rename = "publishedAt")]
    published_at: String,
    duration: String,
    #[serde(rename = "viewCount")]
    view_count: u64,
}

#[derive(Debug, Serialize)]
struct SearchResponse {
    videos: Vec<VideoResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    continuation: Option<String>,
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: String,
    message: String,
    version: String,
    build: String,
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    error_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    http_status_code: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    youtube_response: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    raw_response: Option<String>,
}

#[derive(Debug)]
struct YouTubeError {
    error_type: String,
    message: String,
    http_status_code: Option<u16>,
    youtube_response: Option<serde_json::Value>,
    raw_response: Option<String>,
}

impl std::fmt::Display for YouTubeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.error_type, self.message)
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct YouTubeSearchRequest {
    context: Context,
    #[serde(skip_serializing_if = "Option::is_none")]
    query: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    continuation: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Context {
    client: Client,
}

#[derive(Debug, Serialize, Deserialize)]
struct Client {
    hl: String,
    gl: String,
    #[serde(rename = "userAgent")]
    user_agent: String,
    #[serde(rename = "clientName")]
    client_name: String,
    #[serde(rename = "clientVersion")]
    client_version: String,
    #[serde(rename = "originalUrl")]
    original_url: String,
}

fn parse_youtube_response(data: &serde_json::Value) -> SearchResponse {
    let mut results = Vec::new();
    let mut continuation_token: Option<String> = None;

    if let Some(contents) = data.get("contents") {
        if let Some(renderer) = contents.get("twoColumnSearchResultsRenderer") {
            if let Some(primary) = renderer.get("primaryContents") {
                if let Some(section_list) = primary.get("sectionListRenderer") {
                    if let Some(section_contents) =
                        section_list.get("contents").and_then(|v| v.as_array())
                    {
                        for section in section_contents {
                            if let Some(item_section) = section.get("itemSectionRenderer") {
                                if let Some(items) =
                                    item_section.get("contents").and_then(|v| v.as_array())
                                {
                                    for item in items {
                                        if let Some(video) = item.get("videoRenderer") {
                                            if let Some(video_result) = parse_video_renderer(video)
                                            {
                                                results.push(video_result);
                                            }
                                        }
                                    }
                                }
                            } else if let Some(continuation_item) =
                                section.get("continuationItemRenderer")
                            {
                                if let Some(token) = continuation_item
                                    .get("continuationEndpoint")
                                    .and_then(|ep| ep.get("continuationCommand"))
                                    .and_then(|cmd| cmd.get("token"))
                                    .and_then(|t| t.as_str())
                                {
                                    continuation_token = Some(token.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Handle continuation responses (when using continuation token)
    if let Some(on_response_received_commands) = data.get("onResponseReceivedCommands") {
        if let Some(commands) = on_response_received_commands.as_array() {
            for command in commands {
                if let Some(append_continuation) = command.get("appendContinuationItemsAction") {
                    if let Some(items) = append_continuation
                        .get("continuationItems")
                        .and_then(|v| v.as_array())
                    {
                        for item in items {
                            // Check for itemSectionRenderer first (common in continuation responses)
                            if let Some(item_section) = item.get("itemSectionRenderer") {
                                if let Some(section_contents) =
                                    item_section.get("contents").and_then(|v| v.as_array())
                                {
                                    for section_item in section_contents {
                                        if let Some(video) = section_item.get("videoRenderer") {
                                            if let Some(video_result) = parse_video_renderer(video)
                                            {
                                                results.push(video_result);
                                            }
                                        }
                                    }
                                }
                            }
                            // Direct videoRenderer (fallback)
                            else if let Some(video) = item.get("videoRenderer") {
                                if let Some(video_result) = parse_video_renderer(video) {
                                    results.push(video_result);
                                }
                            }
                            // Check for continuation token
                            else if let Some(continuation_item) =
                                item.get("continuationItemRenderer")
                            {
                                if let Some(token) = continuation_item
                                    .get("continuationEndpoint")
                                    .and_then(|ep| ep.get("continuationCommand"))
                                    .and_then(|cmd| cmd.get("token"))
                                    .and_then(|t| t.as_str())
                                {
                                    continuation_token = Some(token.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    SearchResponse {
        videos: results,
        continuation: continuation_token,
    }
}

fn parse_video_renderer(video: &serde_json::Value) -> Option<VideoResult> {
    let id = video.get("videoId")?.as_str()?.to_string();

    let title = video
        .get("title")?
        .get("runs")?
        .get(0)?
        .get("text")?
        .as_str()?
        .to_string();

    let thumbnails = video.get("thumbnail")?.get("thumbnails")?.as_array()?;
    let thumb_default = thumbnails.first()?.get("url")?.as_str()?.to_string();
    let thumb_medium = thumbnails
        .get(1)
        .and_then(|t| t.get("url"))
        .and_then(|u| u.as_str())
        .unwrap_or(&thumb_default)
        .to_string();
    let thumb_high = thumbnails
        .get(2)
        .and_then(|t| t.get("url"))
        .and_then(|u| u.as_str())
        .unwrap_or(&thumb_medium)
        .to_string();

    let channel_title = video
        .get("ownerText")?
        .get("runs")?
        .get(0)?
        .get("text")?
        .as_str()?
        .to_string();

    let channel_id = video
        .get("ownerText")?
        .get("runs")?
        .get(0)?
        .get("navigationEndpoint")?
        .get("browseEndpoint")?
        .get("browseId")?
        .as_str()?
        .to_string();

    let published_at = video
        .get("publishedTimeText")?
        .get("simpleText")?
        .as_str()?
        .to_string();

    let duration = video
        .get("lengthText")?
        .get("simpleText")?
        .as_str()?
        .to_string();

    let view_count_text = video.get("viewCountText")?.get("simpleText")?.as_str()?;

    let view_count = parse_view_count(view_count_text);

    Some(VideoResult {
        id,
        title,
        thumb_default,
        thumb_medium,
        thumb_high,
        channel_title,
        channel_id,
        published_at,
        duration,
        view_count,
    })
}

fn parse_view_count(text: &str) -> u64 {
    let cleaned = text
        .split_whitespace()
        .next()
        .unwrap_or("0")
        .replace(",", "");

    cleaned.parse::<u64>().unwrap_or(0)
}

async fn search_youtube(
    client: &reqwest::Client,
    query: Option<&str>,
    continuation: Option<String>,
) -> Result<serde_json::Value, YouTubeError> {
    // Get the search query for originalUrl - use query if available, otherwise extract from continuation context
    let url_query = query.unwrap_or("search");

    let context = Context {
        client: Client {
            hl: "en".to_string(),
            gl: "US".to_string(),
            user_agent: "Mozilla/5.0 (Macintosh; Intel Mac OS X 15_7_2) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/26.0 Safari/605.1.15".to_string(),
            client_name: "WEB".to_string(),
            client_version: "2.20251117.01.00".to_string(),
            original_url: format!("https://www.youtube.com/results?search_query={}", url_query),
        },
    };

    let payload = if let Some(cont_token) = continuation {
        YouTubeSearchRequest {
            context,
            query: None,
            continuation: Some(cont_token),
        }
    } else if let Some(search_query) = query {
        YouTubeSearchRequest {
            context,
            query: Some(search_query.to_string()),
            continuation: None,
        }
    } else {
        return Err(YouTubeError {
            error_type: "invalid_request".to_string(),
            message: "Either query or continuation must be provided".to_string(),
            http_status_code: None,
            youtube_response: None,
            raw_response: None,
        });
    };

    // Send request
    let response = client
        .post("https://www.youtube.com/youtubei/v1/search")
        .json(&payload)
        .send()
        .await
        .map_err(|e| {
            let error_type = if e.is_timeout() {
                "timeout"
            } else if e.is_connect() {
                "connection_error"
            } else if e.is_request() {
                "request_error"
            } else {
                "network_error"
            };

            YouTubeError {
                error_type: error_type.to_string(),
                message: format!("Failed to send request to YouTube: {}", e),
                http_status_code: None,
                youtube_response: None,
                raw_response: None,
            }
        })?;

    let status = response.status();
    let status_code = status.as_u16();

    // Check for non-200 status codes
    if !status.is_success() {
        let raw_body = response.text().await.ok();
        return Err(YouTubeError {
            error_type: "http_error".to_string(),
            message: format!(
                "YouTube API returned HTTP {}: {}",
                status_code,
                status.canonical_reason().unwrap_or("Unknown")
            ),
            http_status_code: Some(status_code),
            youtube_response: None,
            raw_response: raw_body,
        });
    }

    // Try to parse JSON
    match response.json::<serde_json::Value>().await {
        Ok(result) => Ok(result),
        Err(e) => {
            // JSON parse failed - we might have received HTML or invalid JSON
            Err(YouTubeError {
                error_type: "json_parse_error".to_string(),
                message: format!("Failed to parse YouTube response as JSON: {}", e),
                http_status_code: Some(status_code),
                youtube_response: None,
                raw_response: None, // Already consumed by json() attempt
            })
        }
    }
}

async fn handle_search(
    State(state): State<Arc<AppState>>,
    Path(query): Path<String>,
    Query(params): Query<NextToken>,
) -> Result<Json<SearchResponse>, (StatusCode, Json<ErrorResponse>)> {
    let search_query = if params.next.is_some() {
        info!("Searching with continuation token");
        None
    } else {
        info!("Searching for: {}", query);
        Some(query.as_str())
    };

    match search_youtube(&state.http_client, search_query, params.next).await {
        Ok(result) => {
            // Check if the response has the expected structure
            let has_videos = result.get("contents").is_some()
                || result.get("onResponseReceivedCommands").is_some();

            if !has_videos {
                warn!("YouTube returned unexpected response structure");
                return Err((
                    StatusCode::BAD_GATEWAY,
                    Json(ErrorResponse {
                        error: "unexpected_youtube_response".to_string(),
                        message: "YouTube returned an unexpected response format".to_string(),
                        error_type: Some("unexpected_response".to_string()),
                        http_status_code: Some(200),
                        youtube_response: Some(result),
                        raw_response: None,
                    }),
                ));
            }

            let parsed_results = parse_youtube_response(&result);

            if parsed_results.videos.is_empty() && parsed_results.continuation.is_none() {
                warn!("YouTube response parsed but no videos found");
                return Err((
                    StatusCode::BAD_GATEWAY,
                    Json(ErrorResponse {
                        error: "no_results".to_string(),
                        message: "No videos found in YouTube response".to_string(),
                        error_type: Some("no_results".to_string()),
                        http_status_code: Some(200),
                        youtube_response: Some(result),
                        raw_response: None,
                    }),
                ));
            }

            info!("Found {} videos", parsed_results.videos.len());
            Ok(Json(parsed_results))
        }
        Err(e) => {
            error!("YouTube error - {}: {}", e.error_type, e.message);

            // Determine HTTP status code based on error type
            let status_code = match e.error_type.as_str() {
                "timeout" => StatusCode::GATEWAY_TIMEOUT,
                "connection_error" | "network_error" => StatusCode::BAD_GATEWAY,
                "http_error" => {
                    // Use the actual HTTP status code from YouTube if available
                    if let Some(code) = e.http_status_code {
                        StatusCode::from_u16(code).unwrap_or(StatusCode::BAD_GATEWAY)
                    } else {
                        StatusCode::BAD_GATEWAY
                    }
                }
                "json_parse_error" => StatusCode::BAD_GATEWAY,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };

            Err((
                status_code,
                Json(ErrorResponse {
                    error: "youtube_request_failed".to_string(),
                    message: e.message,
                    error_type: Some(e.error_type),
                    http_status_code: e.http_status_code,
                    youtube_response: e.youtube_response,
                    raw_response: e.raw_response,
                }),
            ))
        }
    }
}

async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "OK".to_string(),
        message: "RustyTube - YouTube Search Server".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        build: env!("GIT_HASH").to_string(),
    })
}

fn get_port() -> u16 {
    // Check for --port CLI argument
    let args: Vec<String> = env::args().collect();
    for i in 0..args.len() {
        if args[i] == "--port" && i + 1 < args.len() {
            if let Ok(port) = args[i + 1].parse::<u16>() {
                return port;
            }
        }
    }

    // Check for PORT environment variable
    if let Ok(port_str) = env::var("PORT") {
        if let Ok(port) = port_str.parse::<u16>() {
            return port;
        }
    }

    // Default port
    3000
}

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "rustytube=info,tower_http=info".into()),
        )
        .init();

    info!("Starting RustyTube v{}", env!("CARGO_PKG_VERSION"));

    // Create shared HTTP client with connection pooling
    let http_client = reqwest::Client::builder()
        .pool_max_idle_per_host(10)
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .expect("Failed to create HTTP client");

    let state = Arc::new(AppState { http_client });

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/", get(health_check))
        .route("/api/search/:query", get(handle_search))
        .layer(cors)
        .with_state(state);

    let port = get_port();
    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    info!("RustyTube listening on http://{}", addr);
    info!("Try: http://localhost:{}/api/search/sia", port);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind to port");

    axum::serve(listener, app).await.unwrap();
}
