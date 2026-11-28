# RustyTube 🦀

[![CI Test](https://github.com/rustynail1984/rustytube-search/actions/workflows/ci.yml/badge.svg)](https://github.com/rustynail1984/rustytube-search/actions/workflows/ci.yml) [![Release](https://github.com/rustynail1984/rustytube-search/actions/workflows/release.yml/badge.svg)](https://github.com/rustynail1984/rustytube-search/actions/workflows/release.yml)

A blazingly fast YouTube search server written in Rust using YouTube's InnerTube API.


## Features

- ⚡ **Fast & Lightweight** - Built with Rust and async runtime (Tokio)
- 🔍 **YouTube Search** - Search videos using YouTube's InnerTube API
- 📄 **Pagination Support** - Continue browsing results with continuation tokens
- 🛡️ **Production Ready** - HTTP client pooling, structured logging, optimized builds
- 🔧 **Configurable** - Port configuration via CLI args, environment variables
- 📊 **Enhanced Error Handling** - Detailed error responses with error types and HTTP status codes
- 💾 **Low Memory** - ~6.5MB RAM usage in release mode

## Quick Start

### Download Pre-built Binaries

Download the latest release for your platform from [GitHub Releases](https://github.com/rustynail1984/rustytube-search/releases):

**Linux x86_64:**
- `rustytube-linux-x86_64` - Standard Linux (glibc)
- `rustytube-linux-x86_64-musl` - Static binary (musl)

**Linux ARM64 (Raspberry Pi, AWS Graviton, etc.):**
- `rustytube-linux-aarch64` - Standard ARM64 Linux (glibc)
- `rustytube-linux-aarch64-musl` - Static ARM64 binary (musl)

```bash
# Download and run (Linux x86_64 example)
wget https://github.com/rustynail1984/rustytube-search/releases/latest/download/rustytube-linux-x86_64
chmod +x rustytube-linux-x86_64
./rustytube-linux-x86_64

# For ARM64 (Raspberry Pi)
wget https://github.com/rustynail1984/rustytube-search/releases/latest/download/rustytube-linux-aarch64
chmod +x rustytube-linux-aarch64
./rustytube-linux-aarch64
```

### Build from Source

```bash
# Clone the repository
git clone https://github.com/rustynail1984/rustytube-search
cd rustytube

# Build release version
cargo build --release

# Run the server
./target/release/rustytube
```

The server will start on `http://localhost:3000` by default.

### Command Line Options

```bash
# Show version (all work the same)
./target/release/rustytube --version
./target/release/rustytube -v
./target/release/rustytube -V

# Show help
./target/release/rustytube --help
./target/release/rustytube -h

# Run with custom port
./target/release/rustytube --port 3030
./target/release/rustytube -p 3030

# Bind to localhost only (more secure)
./target/release/rustytube --host 127.0.0.1

# Set log level to debug
./target/release/rustytube --log-level debug

# Set custom timeout (in seconds)
./target/release/rustytube --timeout 60

# Combine multiple options
./target/release/rustytube --port 8080 --host 127.0.0.1 --log-level warn --timeout 45
```

**Available Options:**
- `-p, --port <PORT>` - Port to listen on (default: 3000)
- `--host <HOST>` - Host address to bind to (default: 0.0.0.0)
- `--log-level <LEVEL>` - Log level: error, warn, info, debug, trace (default: info)
- `--timeout <TIMEOUT>` - HTTP request timeout in seconds (default: 30)
- `--proxy <URL>` - Proxy for requests to YouTube (http, https, socks5, socks5h; env: `RUSTYTUBE_PROXY`)
- `-v, -V, --version` - Print version information
- `-h, --help` - Print help

**Example output:**
```bash
$ ./target/release/rustytube -v
RustyTube 1.0.0
```

## API Documentation

### Health Check

**GET /** - Returns server status and version information

```bash
curl http://localhost:3000
```

Response:
```json
{
  "status": "OK",
  "message": "RustyTube - YouTube Search Server",
  "version": "1.0.0",
  "build": "d03ff7c"
}
```

### Search Videos

**GET /api/search/:query** - Search for YouTube videos

```bash
curl http://localhost:3000/api/search/sia
```

Response:
```json
{
  "videos": [
    {
      "id": "video_id",
      "title": "Video Title",
      "thumbDefault": "https://...",
      "thumbMedium": "https://...",
      "thumbHigh": "https://...",
      "channelTitle": "Channel Name",
      "channelId": "channel_id",
      "publishedAt": "2 years ago",
      "duration": "10:23",
      "viewCount": 1234567
    }
  ],
  "continuation": "continuation_token_here"
}
```

### Pagination

**GET /api/search/:query?next=:token** - Get next page of results

```bash
curl "http://localhost:3000/api/search/sia?next=CONTINUATION_TOKEN"
```

Response format is the same as the search endpoint.

### Error Responses

All errors return a JSON response with detailed information:

```json
{
  "error": "youtube_request_failed",
  "message": "Failed to search YouTube: timeout",
  "error_type": "timeout",
  "http_status_code": 504,
  "youtube_response": null,
  "raw_response": null
}
```

**Error Types:**
- `timeout` - Request to YouTube timed out
- `connection_error` - Failed to connect to YouTube
- `http_error` - YouTube returned non-200 status code
- `json_parse_error` - Failed to parse YouTube response
- `no_results` - YouTube returned empty results
- `unexpected_response` - YouTube returned unexpected data structure

## Configuration

### Port Configuration

The server port can be configured in three ways (in order of priority):

1. **Environment Variable** (highest priority)
   ```bash
   PORT=3030 ./target/release/rustytube
   ```

2. **CLI Argument**
   ```bash
   ./target/release/rustytube --port 3030
   # or
   ./target/release/rustytube -p 3030
   ```

3. **Default** - Port 3000 if not specified

### Logging

Set log level via the `--log-level` CLI argument or `RUST_LOG` environment variable (env var takes priority):

```bash
# Using CLI argument (recommended)
./target/release/rustytube --log-level info    # Default - Shows startup messages only
./target/release/rustytube --log-level debug   # Shows all search requests and results
./target/release/rustytube --log-level warn    # Shows only warnings and errors
./target/release/rustytube --log-level error   # Shows only errors

# Using environment variable (overrides CLI argument)
RUST_LOG=debug ./target/release/rustytube
RUST_LOG=warn ./target/release/rustytube
```

**Recommended for production**: `--log-level info` or `--log-level warn`

### Host Binding

Configure which network interface to bind to:

```bash
# Bind to all interfaces (default) - accessible from anywhere
./target/release/rustytube --host 0.0.0.0

# Bind to localhost only - more secure, only local access
./target/release/rustytube --host 127.0.0.1
```

### Request Timeout

Configure the HTTP request timeout for YouTube API calls:

```bash
# Default timeout (30 seconds)
./target/release/rustytube --timeout 30

# Longer timeout for slow connections
./target/release/rustytube --timeout 60
```

### Proxy

Route all requests to YouTube through a proxy via the `--proxy` CLI argument or the `RUSTYTUBE_PROXY` environment variable (the CLI argument takes priority). Supported schemes: `http://`, `https://`, `socks5://` and `socks5h://` (DNS resolved by the proxy).

```bash
# HTTP proxy
./target/release/rustytube --proxy http://127.0.0.1:8080

# SOCKS5 proxy with authentication
./target/release/rustytube --proxy socks5://user:password@proxy.example.com:1080

# Via environment variable
RUSTYTUBE_PROXY=socks5h://127.0.0.1:9050 ./target/release/rustytube
```

Credentials are masked in the log output. An invalid proxy URL stops the server at startup. If no proxy is set, the standard `HTTP_PROXY` / `HTTPS_PROXY` environment variables are respected.

### Tokio Workers

Configure the number of async worker threads:

```bash
TOKIO_WORKER_THREADS=8 ./target/release/rustytube
```

## Development

### Prerequisites

- Rust 1.70 or higher
- Git

### Build

```bash
# Development build
cargo build

# Run in development mode
cargo run

# Run with custom port
cargo run -- --port 3030
```

### Testing

```bash
# Run tests
cargo test

# Run with output
cargo test -- --nocapture
```

## Production Deployment

### Build for Production

```bash
cargo build --release
```

The optimized binary will be at `target/release/rustytube` (~3.6MB).

### Performance Optimizations

The release build includes:
- **Link-Time Optimization (LTO)** - Better performance
- **Optimization Level 3** - Maximum compiler optimizations
- **Symbol Stripping** - Smaller binary size
- **Connection Pooling** - Reuses HTTP connections (max 10 per host)
- **30s Request Timeout** - Prevents hanging requests

### Memory Usage

- Base: ~6.5MB
- Per Request: Minimal overhead due to async/await and connection pooling

### Linux Service Example

Create a systemd service file at `/etc/systemd/system/rustytube.service`:

```ini
[Unit]
Description=RustyTube YouTube Search Server
After=network.target

[Service]
Type=simple
User=www-data
WorkingDirectory=/opt/rustytube
ExecStart=/opt/rustytube/rustytube-linux-x86_64 --port 3030
Restart=always
RestartSec=10
Environment="RUST_LOG=info"

[Install]
WantedBy=multi-user.target
```

Enable and start the service:
```bash
sudo systemctl enable rustytube
sudo systemctl start rustytube
```

### Reverse Proxy (nginx)

```nginx
location /api/ {
    proxy_pass http://localhost:3030/api/;
    proxy_http_version 1.1;
    proxy_set_header Host $host;
    proxy_set_header X-Real-IP $remote_addr;
    proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
}
```

## Technology Stack

- **[Axum](https://github.com/tokio-rs/axum)** - Web framework
- **[Tokio](https://tokio.rs/)** - Async runtime
- **[Reqwest](https://github.com/seanmonstar/reqwest)** - HTTP client with rustls
- **[Rustls](https://github.com/rustls/rustls)** - Pure Rust TLS implementation (no OpenSSL dependency)
- **[Serde](https://serde.rs/)** - JSON serialization
- **[Clap](https://github.com/clap-rs/clap)** - Command-line argument parser
- **[Tower HTTP](https://github.com/tower-rs/tower-http)** - CORS middleware
- **[Tracing](https://github.com/tokio-rs/tracing)** - Structured logging

## Architecture

RustyTube uses YouTube's **InnerTube API**, an internal API used by YouTube's web interface. The server:

1. Accepts search requests via HTTP
2. Formats requests for YouTube's InnerTube endpoint
3. Parses YouTube's complex JSON responses
4. Returns clean, normalized JSON data

**Key Features:**
- Connection pooling for efficient HTTP requests
- Handles both initial search and pagination responses
- Comprehensive error handling with detailed error information
- Production-ready with optimized builds

## License

MIT License - see LICENSE file for details

## Contributing

Contributions are welcome! Please open an issue or submit a pull request.

---

Made with 🦀 Rust
