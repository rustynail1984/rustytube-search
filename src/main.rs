use axum::{
    extract::{Path, Query},
    http::StatusCode,
    response::Json,
    routing::get,
    Router,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::env;
use tower_http::cors::{Any, CorsLayer};

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
                    if let Some(section_contents) = section_list.get("contents").and_then(|v| v.as_array()) {
                        for section in section_contents {
                            if let Some(item_section) = section.get("itemSectionRenderer") {
                                if let Some(items) = item_section.get("contents").and_then(|v| v.as_array()) {
                                    for item in items {
                                        if let Some(video) = item.get("videoRenderer") {
                                            if let Some(video_result) = parse_video_renderer(video) {
                                                results.push(video_result);
                                            }
                                        }
                                    }
                                }
                            } else if let Some(continuation_item) = section.get("continuationItemRenderer") {
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
                    if let Some(items) = append_continuation.get("continuationItems").and_then(|v| v.as_array()) {
                        for item in items {
                            // Check for itemSectionRenderer first (common in continuation responses)
                            if let Some(item_section) = item.get("itemSectionRenderer") {
                                if let Some(section_contents) = item_section.get("contents").and_then(|v| v.as_array()) {
                                    for section_item in section_contents {
                                        if let Some(video) = section_item.get("videoRenderer") {
                                            if let Some(video_result) = parse_video_renderer(video) {
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
                            else if let Some(continuation_item) = item.get("continuationItemRenderer") {
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

    let title = video.get("title")
        ?.get("runs")?
        .get(0)?
        .get("text")?
        .as_str()?
        .to_string();

    let thumbnails = video.get("thumbnail")?.get("thumbnails")?.as_array()?;
    let thumb_default = thumbnails.get(0)?.get("url")?.as_str()?.to_string();
    let thumb_medium = thumbnails.get(1)
        .and_then(|t| t.get("url"))
        .and_then(|u| u.as_str())
        .unwrap_or(&thumb_default)
        .to_string();
    let thumb_high = thumbnails.get(2)
        .and_then(|t| t.get("url"))
        .and_then(|u| u.as_str())
        .unwrap_or(&thumb_medium)
        .to_string();

    let channel_title = video.get("ownerText")
        ?.get("runs")?
        .get(0)?
        .get("text")?
        .as_str()?
        .to_string();

    let channel_id = video.get("ownerText")
        ?.get("runs")?
        .get(0)?
        .get("navigationEndpoint")?
        .get("browseEndpoint")?
        .get("browseId")?
        .as_str()?
        .to_string();

    let published_at = video.get("publishedTimeText")
        ?.get("simpleText")?
        .as_str()?
        .to_string();

    let duration = video.get("lengthText")
        ?.get("simpleText")?
        .as_str()?
        .to_string();

    let view_count_text = video.get("viewCountText")
        ?.get("simpleText")?
        .as_str()?;

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

async fn search_youtube(query: Option<&str>, continuation: Option<String>) -> Result<serde_json::Value, String> {
    let client = reqwest::Client::new();

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
        return Err("Either query or continuation must be provided".to_string());
    };

    let response = client
        .post("https://www.youtube.com/youtubei/v1/search")
        .json(&payload)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let result = response.json::<serde_json::Value>().await.map_err(|e| e.to_string())?;
    Ok(result)
}

async fn handle_search(
    Path(query): Path<String>,
    Query(params): Query<NextToken>,
) -> Result<Json<SearchResponse>, (StatusCode, String)> {
    let search_query = if params.next.is_some() {
        None
    } else {
        Some(query.as_str())
    };

    match search_youtube(search_query, params.next).await {
        Ok(result) => {
            let parsed_results = parse_youtube_response(&result);
            Ok(Json(parsed_results))
        },
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to search YouTube: {}", e),
        )),
    }
}

async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "OK".to_string(),
        message: "RUST YouTube Search Server".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
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
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/", get(health_check))
        .route("/api/search/:query", get(handle_search))
        .layer(cors);

    let port = get_port();
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    println!("YouTube Search Server listening on http://{}", addr);
    println!("Try: http://localhost:{}/api/search/sia", port);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
