use clap::{Parser, ValueEnum};
use serde::{Deserialize, Serialize};

/// RustyTube - A blazingly fast YouTube search server
#[derive(Parser, Debug)]
#[command(name = "RustyTube")]
#[command(version)]
#[command(about = "YouTube search server using InnerTube API", long_about = None)]
#[command(disable_version_flag = true)]
pub struct Cli {
    /// Port to listen on (can also be set via PORT environment variable)
    #[arg(short, long, default_value_t = 3000)]
    pub port: u16,

    /// Host address to bind to
    #[arg(long, default_value = "0.0.0.0")]
    pub host: String,

    /// Log level (error, warn, info, debug, trace)
    #[arg(long, default_value = "info")]
    pub log_level: LogLevel,

    /// HTTP request timeout in seconds for YouTube API
    #[arg(long, default_value_t = 30)]
    pub timeout: u64,

    /// Print version information
    #[arg(short = 'v', short_alias = 'V', long = "version", action = clap::ArgAction::Version)]
    _version: Option<bool>,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl LogLevel {
    pub fn as_str(&self) -> &str {
        match self {
            LogLevel::Error => "error",
            LogLevel::Warn => "warn",
            LogLevel::Info => "info",
            LogLevel::Debug => "debug",
            LogLevel::Trace => "trace",
        }
    }
}

#[derive(Clone)]
pub struct AppState {
    pub http_client: reqwest::Client,
}

#[derive(Debug, Deserialize)]
pub struct NextToken {
    pub next: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct VideoResult {
    pub id: String,
    pub title: String,
    #[serde(rename = "thumbDefault")]
    pub thumb_default: String,
    #[serde(rename = "thumbMedium")]
    pub thumb_medium: String,
    #[serde(rename = "thumbHigh")]
    pub thumb_high: String,
    #[serde(rename = "channelTitle")]
    pub channel_title: String,
    #[serde(rename = "channelId")]
    pub channel_id: String,
    #[serde(rename = "publishedAt")]
    pub published_at: String,
    pub duration: String,
    #[serde(rename = "viewCount")]
    pub view_count: u64,
}

#[derive(Debug, Serialize)]
pub struct SearchResponse {
    pub videos: Vec<VideoResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub continuation: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub message: String,
    pub version: String,
    pub build: String,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http_status_code: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub youtube_response: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_response: Option<String>,
}

#[derive(Debug)]
pub struct YouTubeError {
    pub error_type: String,
    pub message: String,
    pub http_status_code: Option<u16>,
    pub youtube_response: Option<serde_json::Value>,
    pub raw_response: Option<String>,
}

impl std::fmt::Display for YouTubeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.error_type, self.message)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct YouTubeSearchRequest {
    pub context: Context,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub continuation: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Context {
    pub client: Client,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Client {
    pub hl: String,
    pub gl: String,
    #[serde(rename = "userAgent")]
    pub user_agent: String,
    #[serde(rename = "clientName")]
    pub client_name: String,
    #[serde(rename = "clientVersion")]
    pub client_version: String,
    #[serde(rename = "originalUrl")]
    pub original_url: String,
}
