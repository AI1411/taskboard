use axum::http::HeaderValue;

pub fn origin_allowed(origin: Option<&HeaderValue>, port: u16) -> bool {
    let Some(value) = header_str(origin) else {
        return false;
    };
    value == format!("http://127.0.0.1:{port}") || value == format!("http://localhost:{port}")
}

pub fn host_allowed(host: Option<&HeaderValue>, port: u16) -> bool {
    let Some(value) = header_str(host) else {
        return false;
    };
    let value = value.to_ascii_lowercase();
    value == format!("127.0.0.1:{port}") || value == format!("localhost:{port}")
}

fn header_str(value: Option<&HeaderValue>) -> Option<&str> {
    value.and_then(|value| value.to_str().ok())
}
