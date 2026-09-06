use axum::http::HeaderValue;

pub fn origin_allowed(origin: Option<&HeaderValue>, port: u16) -> bool {
    origin
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value == format!("http://127.0.0.1:{port}"))
}
