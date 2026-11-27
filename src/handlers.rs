use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use std::sync::Arc;
use tracing::{debug, error, warn};

use crate::models::*;
use crate::youtube::{parse_youtube_response, search_youtube};

pub async fn handle_search(
    State(state): State<Arc<AppState>>,
    Path(query): Path<String>,
    Query(params): Query<NextToken>,
) -> Result<Json<SearchResponse>, (StatusCode, Json<Box<ErrorResponse>>)> {
    let search_query = if params.next.is_some() {
        debug!("Searching with continuation token");
        None
    } else {
        debug!("Searching for: {}", query);
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
                    Json(Box::new(ErrorResponse {
                        error: "unexpected_youtube_response".to_string(),
                        message: "YouTube returned an unexpected response format".to_string(),
                        error_type: Some("unexpected_response".to_string()),
                        http_status_code: Some(200),
                        youtube_response: Some(result),
                        raw_response: None,
                    })),
                ));
            }

            let parsed_results = parse_youtube_response(&result);

            if parsed_results.videos.is_empty() && parsed_results.continuation.is_none() {
                warn!("YouTube response parsed but no videos found");
                return Err((
                    StatusCode::BAD_GATEWAY,
                    Json(Box::new(ErrorResponse {
                        error: "no_results".to_string(),
                        message: "No videos found in YouTube response".to_string(),
                        error_type: Some("no_results".to_string()),
                        http_status_code: Some(200),
                        youtube_response: Some(result),
                        raw_response: None,
                    })),
                ));
            }

            debug!("Found {} videos", parsed_results.videos.len());
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
                Json(Box::new(ErrorResponse {
                    error: "youtube_request_failed".to_string(),
                    message: e.message,
                    error_type: Some(e.error_type),
                    http_status_code: e.http_status_code,
                    youtube_response: e.youtube_response,
                    raw_response: e.raw_response,
                })),
            ))
        }
    }
}

pub async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "OK".to_string(),
        message: "RustyTube - YouTube Search Server".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        build: env!("GIT_HASH").to_string(),
    })
}
