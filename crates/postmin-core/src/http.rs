use std::io::Read;
use std::time::{Duration, Instant};

pub const MAX_BODY_BYTES: usize = 10 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct RequestSpec {
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<Vec<u8>>,
    pub timeout_secs: u64,
    pub follow_redirects: bool,
}

#[derive(Debug, Clone)]
pub struct ResponseData {
    pub status: u16,
    pub status_text: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    pub final_url: String,
    pub duration_ms: u128,
    pub truncated: bool,
}

pub fn normalize_url(input: &str) -> Result<String, String> {
    let url = input.trim();
    if url.is_empty() {
        return Err("请求 URL 不能为空".to_string());
    }
    if url.len() > 8192 {
        return Err("请求 URL 过长".to_string());
    }
    let with_scheme = if url.contains("://") {
        url.to_string()
    } else {
        format!("http://{url}")
    };
    let lower = with_scheme.to_ascii_lowercase();
    if !(lower.starts_with("http://") || lower.starts_with("https://")) {
        return Err("仅支持 http:// 或 https:// 协议".to_string());
    }
    Ok(with_scheme)
}

fn is_tchar(byte: u8) -> bool {
    byte.is_ascii_alphanumeric()
        || matches!(
            byte,
            b'!' | b'#'
                | b'$'
                | b'%'
                | b'&'
                | b'\''
                | b'*'
                | b'+'
                | b'-'
                | b'.'
                | b'^'
                | b'_'
                | b'`'
                | b'|'
                | b'~'
        )
}

pub fn is_valid_header_name(name: &str) -> bool {
    !name.is_empty() && name.bytes().all(is_tchar)
}

pub fn is_valid_header_value(value: &str) -> bool {
    value
        .bytes()
        .all(|byte| byte == b'\t' || (0x20..=0x7e).contains(&byte))
}

pub fn validate_headers(headers: &[(String, String)]) -> Result<(), String> {
    for (name, value) in headers {
        if !is_valid_header_name(name) {
            return Err(format!("Header 名称不合法：{name:?}"));
        }
        if !is_valid_header_value(value) {
            return Err(format!("Header 值包含非法字符：{name}"));
        }
    }
    Ok(())
}

fn validate_method(method: &str) -> Result<String, String> {
    let method = method.trim().to_ascii_uppercase();
    if method.is_empty() {
        return Err("请求 Method 不能为空".to_string());
    }
    if !method.bytes().all(is_tchar) {
        return Err("请求 Method 只能包含 HTTP Token 字符（字母、数字及 -_. 等）".to_string());
    }
    if method.len() > 32 {
        return Err("请求 Method 过长".to_string());
    }
    Ok(method)
}

fn friendly_transport_error(error: &ureq::Transport, timeout_secs: u64) -> String {
    let message = error.to_string();
    let lower = message.to_ascii_lowercase();
    let hint = if lower.contains("timed out") || lower.contains("timeout") {
        format!("请求超时（{timeout_secs} 秒），可在设置中调整超时时间")
    } else if lower.contains("dns")
        || lower.contains("resolve")
        || lower.contains("name or service")
    {
        "域名解析失败，请检查 URL 与网络连接".to_string()
    } else if lower.contains("certificate")
        || lower.contains("tls")
        || lower.contains("handshake")
        || lower.contains("ssl")
    {
        "TLS/证书校验失败，请确认目标地址证书有效，或改用 http://".to_string()
    } else if lower.contains("refused") || lower.contains("10061") {
        "连接被拒绝，目标服务可能未启动或端口不可达".to_string()
    } else if lower.contains("reset") {
        "连接被重置，请稍后重试".to_string()
    } else if lower.contains("proxy") {
        "代理连接失败，请检查系统代理设置".to_string()
    } else {
        "网络请求失败，请检查地址、网络或代理设置".to_string()
    };
    format!("{hint}\n原始错误：{message}")
}

pub fn execute(spec: &RequestSpec) -> Result<ResponseData, String> {
    let method = validate_method(&spec.method)?;
    let url = normalize_url(&spec.url)?;
    validate_headers(&spec.headers)?;

    let timeout_secs = spec.timeout_secs.clamp(1, 300);
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(timeout_secs))
        .redirects(if spec.follow_redirects { 10 } else { 0 })
        .build();

    let started = Instant::now();
    let mut request = agent.request(&method, &url);
    for (name, value) in &spec.headers {
        request = request.set(name, value);
    }
    let result = match spec.body.as_deref() {
        Some(body) => request.send_bytes(body),
        None => request.call(),
    };

    let (status, status_text, headers, final_url, reader) = match result {
        Ok(response) => collect_response(response)?,
        Err(ureq::Error::Status(code, response)) => {
            let (_, status_text, headers, final_url, reader) = collect_response(response)?;
            (code, status_text, headers, final_url, reader)
        }
        Err(ureq::Error::Transport(error)) => {
            return Err(friendly_transport_error(&error, timeout_secs));
        }
    };

    let mut body = Vec::new();
    let mut limited = reader.take((MAX_BODY_BYTES + 1) as u64);
    limited
        .read_to_end(&mut body)
        .map_err(|error| format!("读取响应失败：{error}"))?;
    let truncated = body.len() > MAX_BODY_BYTES;
    if truncated {
        body.truncate(MAX_BODY_BYTES);
    }
    let duration_ms = started.elapsed().as_millis();

    Ok(ResponseData {
        status,
        status_text,
        headers,
        body,
        final_url,
        duration_ms,
        truncated,
    })
}

type ResponseParts = (
    u16,
    String,
    Vec<(String, String)>,
    String,
    Box<dyn Read + Send + Sync + 'static>,
);

fn collect_response(response: ureq::Response) -> Result<ResponseParts, String> {
    let status = response.status();
    let status_text = response.status_text().to_string();
    let final_url = response.get_url().to_string();
    let mut headers = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for name in response.headers_names() {
        if !seen.insert(name.clone()) {
            continue;
        }
        for value in response.all(&name) {
            headers.push((name.clone(), value.to_string()));
        }
    }
    let reader = response.into_reader();
    Ok((status, status_text, headers, final_url, reader))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;
    use std::net::TcpListener;

    #[test]
    fn normalizes_urls() {
        assert_eq!(
            normalize_url("example.com/api").unwrap(),
            "http://example.com/api"
        );
        assert_eq!(
            normalize_url(" https://example.com ").unwrap(),
            "https://example.com"
        );
        assert!(normalize_url("").is_err());
        assert!(normalize_url("ftp://example.com").is_err());
    }

    #[test]
    fn validates_header_hygiene() {
        assert!(is_valid_header_name("X-Trace-Id"));
        assert!(!is_valid_header_name("Bad Header"));
        assert!(!is_valid_header_name(""));
        assert!(is_valid_header_value("hello world"));
        assert!(!is_valid_header_value("bad\r\nInjected: 1"));
        assert!(validate_headers(&[("Accept".into(), "application/json".into())]).is_ok());
        assert!(validate_headers(&[("Bad Header".into(), "x".into())]).is_err());
    }

    fn spawn_server() -> (String, std::thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let address = listener.local_addr().expect("address");
        let handle = std::thread::spawn(move || {
            for _ in 0..2 {
                let (mut stream, _) = listener.accept().expect("accept");
                let mut buffer = [0u8; 8192];
                let read = stream.read(&mut buffer).unwrap_or(0);
                let request = String::from_utf8_lossy(&buffer[..read]);
                let path = request
                    .lines()
                    .next()
                    .and_then(|line| line.split_whitespace().nth(1))
                    .unwrap_or("/");
                let body = if path == "/ok" {
                    r#"{"ok":true}"#
                } else {
                    r#"{"error":"not found"}"#
                };
                let status = if path == "/ok" {
                    "200 OK"
                } else {
                    "404 Not Found"
                };
                let response = format!(
                    "HTTP/1.1 {status}\r\nContent-Type: application/json; charset=utf-8\r\nX-Test: postmin\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(response.as_bytes());
                let _ = stream.flush();
            }
        });
        (format!("http://{address}"), handle)
    }

    #[test]
    fn executes_http_and_keeps_error_responses() {
        let (base, handle) = spawn_server();
        let ok = execute(&RequestSpec {
            method: "GET".to_string(),
            url: format!("{base}/ok"),
            headers: vec![("Accept".to_string(), "application/json".to_string())],
            body: None,
            timeout_secs: 5,
            follow_redirects: true,
        })
        .expect("request succeeds");
        assert_eq!(ok.status, 200);
        assert_eq!(ok.status_text, "OK");
        assert!(String::from_utf8_lossy(&ok.body).contains("\"ok\":true"));
        assert!(ok
            .headers
            .iter()
            .any(|(key, value)| key == "x-test" && value == "postmin"));

        let not_found = execute(&RequestSpec {
            method: "GET".to_string(),
            url: format!("{base}/missing"),
            headers: vec![],
            body: None,
            timeout_secs: 5,
            follow_redirects: true,
        })
        .expect("404 is a valid response");
        assert_eq!(not_found.status, 404);
        assert!(String::from_utf8_lossy(&not_found.body).contains("not found"));

        handle.join().expect("server thread");
    }

    #[test]
    fn rejects_bad_methods_and_headers() {
        let spec = RequestSpec {
            method: "GET\nX".to_string(),
            url: "http://example.com".to_string(),
            headers: vec![],
            body: None,
            timeout_secs: 3,
            follow_redirects: true,
        };
        assert!(execute(&spec).is_err());

        let spec = RequestSpec {
            method: "GET".to_string(),
            url: "http://example.com".to_string(),
            headers: vec![("Bad Header".to_string(), "x".to_string())],
            body: None,
            timeout_secs: 3,
            follow_redirects: true,
        };
        assert!(execute(&spec).is_err());
    }
}
