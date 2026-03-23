use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{oneshot, mpsc, RwLock};
use serde::Serialize;

pub const PROXY_PORT: u16 = 18080;

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct ProxyEvent {
    pub domain: String,
    pub action: String,
    pub allowed: bool,
}

pub struct ProxyHandle {
    shutdown_tx: oneshot::Sender<()>,
}

impl ProxyHandle {
    pub fn stop(self) {
        let _ = self.shutdown_tx.send(());
    }
}

// ── Domain matching ───────────────────────────────────────────────────────────

/// Checks domain (or subdomain) against the whitelist.
/// "wikipedia.org" also matches "en.wikipedia.org"
fn is_allowed(host: &str, whitelist: &[String]) -> bool {
    let host = host.to_lowercase();
    // Strip port if included (e.g. "example.com:443" → "example.com")
    let host = host.split(':').next().unwrap_or(&host);

    whitelist.iter().any(|entry| {
        let entry = entry.to_lowercase();
        let entry = entry.trim_start_matches("http://").trim_start_matches("https://");
        let entry = entry.split('/').next().unwrap_or(entry);
        host == entry || host.ends_with(&format!(".{}", entry))
    })
}

// ── HTTP parsing helpers ──────────────────────────────────────────────────────

fn parse_connect_host(first_line: &str) -> Option<String> {
    // "CONNECT example.com:443 HTTP/1.1"
    let parts: Vec<&str> = first_line.split_whitespace().collect();
    if parts.len() >= 2 && parts[0].eq_ignore_ascii_case("CONNECT") {
        Some(parts[1].to_string())
    } else {
        None
    }
}

fn parse_host_header(raw: &str) -> Option<String> {
    for line in raw.lines() {
        if line.to_lowercase().starts_with("host:") {
            let host = line[5..].trim();
            // Strip port
            return Some(host.split(':').next().unwrap_or(host).to_string());
        }
    }
    None
}

fn parse_request_url(first_line: &str) -> Option<String> {
    // "GET http://example.com/path HTTP/1.1"
    first_line.split_whitespace().nth(1).map(|s| s.to_string())
}

fn parse_method(first_line: &str) -> &str {
    first_line.split_whitespace().next().unwrap_or("GET")
}

// ── Blocked response ──────────────────────────────────────────────────────────

fn blocked_response(domain: &str) -> Vec<u8> {
    let body = format!(
        "CrabCage: '{}' ist nicht in deiner Erlaubnisliste.\n\
         Füge diese Domain unter Berechtigungen → Websites hinzu, um den Zugriff zu erlauben.",
        domain
    );
    format!(
        "HTTP/1.1 403 Forbidden\r\n\
         Content-Type: text/plain; charset=utf-8\r\n\
         Content-Length: {}\r\n\
         X-CrabCage: blocked\r\n\
         \r\n{}",
        body.len(),
        body
    )
    .into_bytes()
}

// ── Connection handler ────────────────────────────────────────────────────────

async fn handle_connection(
    mut stream: TcpStream,
    whitelist: Arc<RwLock<Vec<String>>>,
    tx: mpsc::Sender<ProxyEvent>,
) {
    // Read initial request (headers only – up to 8 KB)
    let mut buf = vec![0u8; 8192];
    let n = match stream.read(&mut buf).await {
        Ok(0) | Err(_) => return,
        Ok(n) => n,
    };
    let raw = &buf[..n];
    let text = String::from_utf8_lossy(raw);
    let first_line = text.lines().next().unwrap_or("").to_string();

    // ── HTTPS tunnel (CONNECT method) ────────────────────────────────────────
    if first_line.to_uppercase().starts_with("CONNECT") {
        let host_port = match parse_connect_host(&first_line) {
            Some(h) => h,
            None => return,
        };
        let host = host_port.split(':').next().unwrap_or(&host_port).to_string();

        let allowed = {
            let wl = whitelist.read().await;
            is_allowed(&host, &wl)
        };

        let _ = tx.send(ProxyEvent {
            domain: host.clone(),
            action: "HTTPS".to_string(),
            allowed,
        }).await;

        if !allowed {
            let _ = stream.write_all(&blocked_response(&host)).await;
            return;
        }

        // Establish TCP connection to target, then splice streams
        match TcpStream::connect(&host_port).await {
            Ok(target) => {
                let _ = stream.write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n").await;
                let (mut ci, mut co) = stream.into_split();
                let (mut ti, mut to) = target.into_split();
                // Bidirectional pipe – runs until either side closes
                let _ = tokio::join!(
                    tokio::io::copy(&mut ci, &mut to),
                    tokio::io::copy(&mut ti, &mut co),
                );
            }
            Err(_) => {
                let _ = stream.write_all(b"HTTP/1.1 502 Bad Gateway\r\n\r\n").await;
            }
        }
        return;
    }

    // ── Plain HTTP request ───────────────────────────────────────────────────
    let host = parse_host_header(&text).unwrap_or_default();
    let method = parse_method(&first_line).to_string();

    let allowed = {
        let wl = whitelist.read().await;
        is_allowed(&host, &wl)
    };

    let _ = tx.send(ProxyEvent {
        domain: host.clone(),
        action: format!("HTTP {}", method),
        allowed,
    }).await;

    if !allowed {
        let _ = stream.write_all(&blocked_response(&host)).await;
        return;
    }

    // Forward allowed HTTP request via reqwest
    let url = match parse_request_url(&first_line) {
        Some(u) if u.starts_with("http") => u,
        _ => {
            let _ = stream.write_all(b"HTTP/1.1 400 Bad Request\r\n\r\n").await;
            return;
        }
    };

    match forward_http(&url, &method, &text).await {
        Ok(response_bytes) => {
            let _ = stream.write_all(&response_bytes).await;
        }
        Err(_) => {
            let _ = stream.write_all(b"HTTP/1.1 502 Bad Gateway\r\n\r\n").await;
        }
    }
}

// ── HTTP forwarding via reqwest ───────────────────────────────────────────────

async fn forward_http(url: &str, method: &str, raw_headers: &str) -> Result<Vec<u8>, String> {
    let client = reqwest::Client::builder()
        .no_proxy() // we don't want to go through ourselves again
        .build()
        .map_err(|e| e.to_string())?;

    let req = match method.to_uppercase().as_str() {
        "POST"   => client.post(url),
        "PUT"    => client.put(url),
        "DELETE" => client.delete(url),
        "PATCH"  => client.patch(url),
        "HEAD"   => client.head(url),
        _        => client.get(url),
    };

    // Forward original headers, skip proxy-specific ones
    let skip = ["host", "proxy-connection", "proxy-authorization",
                "transfer-encoding", "te", "trailers", "upgrade", "connection"];
    let req = raw_headers.lines().skip(1).fold(req, |req, line| {
        if let Some((name, value)) = line.split_once(':') {
            if !skip.contains(&name.trim().to_lowercase().as_str()) {
                return req.header(name.trim(), value.trim());
            }
        }
        req
    });

    let resp = req.send().await.map_err(|e| e.to_string())?;
    let status = resp.status();
    let headers = resp.headers().clone();
    let body = resp.bytes().await.map_err(|e| e.to_string())?;

    let mut out = format!(
        "HTTP/1.1 {} {}\r\n",
        status.as_u16(),
        status.canonical_reason().unwrap_or("")
    );
    for (name, value) in &headers {
        if let Ok(v) = value.to_str() {
            out.push_str(&format!("{}: {}\r\n", name.as_str(), v));
        }
    }
    out.push_str(&format!("Content-Length: {}\r\n\r\n", body.len()));

    let mut result = out.into_bytes();
    result.extend_from_slice(&body);
    Ok(result)
}

// ── Public API ────────────────────────────────────────────────────────────────

pub async fn start_proxy(
    whitelist: Arc<RwLock<Vec<String>>>,
    tx: mpsc::Sender<ProxyEvent>,
) -> Result<ProxyHandle, String> {
    let (shutdown_tx, mut shutdown_rx) = oneshot::channel();

    let listener = TcpListener::bind(format!("127.0.0.1:{}", PROXY_PORT))
        .await
        .map_err(|e| format!("Proxy-Port {} nicht verfügbar: {}", PROXY_PORT, e))?;

    tauri::async_runtime::spawn(async move {
        loop {
            tokio::select! {
                result = listener.accept() => {
                    match result {
                        Ok((stream, _)) => {
                            let wl = whitelist.clone();
                            let sender = tx.clone();
                            tauri::async_runtime::spawn(async move {
                                handle_connection(stream, wl, sender).await;
                            });
                        }
                        Err(_) => break,
                    }
                }
                _ = &mut shutdown_rx => break,
            }
        }
    });

    Ok(ProxyHandle { shutdown_tx })
}
