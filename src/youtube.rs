use crate::models::*;

pub fn parse_youtube_response(data: &serde_json::Value) -> SearchResponse {
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

pub async fn search_youtube(
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
