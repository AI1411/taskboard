use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;

use axum::extract::State;
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use serde_json::json;
use taskboard_application::App;
use thiserror::Error;
use tokio::net::TcpListener;

use crate::session::{generate_session, SessionToken};

const SESSION_COOKIE: &str = "taskboard_session";
pub(crate) const SESSION_HEADER: &str = "X-Taskboard-Session";
const HTML_SHELL: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <title>Taskboard</title>
</head>
<body>
  <div id="root"></div>
</body>
</html>
"#;

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("server must bind 127.0.0.1")]
    NotLocalhost,
    #[error("failed to bind {addr}: {source}")]
    Bind {
        addr: SocketAddr,
        #[source]
        source: std::io::Error,
    },
}

pub(crate) struct AppState {
    pub(crate) app: App,
    pub(crate) session: SessionToken,
    pub(crate) port: u16,
}

pub async fn serve(app: App, addr: SocketAddr, open: bool) -> Result<SocketAddr, ApiError> {
    if addr.ip() != Ipv4Addr::LOCALHOST {
        return Err(ApiError::NotLocalhost);
    }

    let listener = TcpListener::bind(addr)
        .await
        .map_err(|source| ApiError::Bind { addr, source })?;
    let bound = listener
        .local_addr()
        .map_err(|source| ApiError::Bind { addr, source })?;

    let state = Arc::new(AppState {
        app,
        session: generate_session(),
        port: bound.port(),
    });

    let _ = open;
    tokio::spawn(async move {
        let _ = axum::serve(listener, router(state)).await;
    });
    Ok(bound)
}

fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", get(html_shell))
        .merge(crate::routes::api_router())
        .with_state(state)
}

async fn html_shell(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let cookie = format!(
        "{SESSION_COOKIE}={}; Path=/; HttpOnly; SameSite=Strict",
        state.session.0
    );
    let mut headers = HeaderMap::new();
    headers.insert(
        header::SET_COOKIE,
        HeaderValue::from_str(&cookie).expect("session token is hex"),
    );
    (headers, Html(HTML_SHELL))
}

pub(crate) fn session_from_header(headers: &HeaderMap, expected: &str) -> bool {
    headers
        .get(SESSION_HEADER)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|token| token == expected)
}

pub(crate) fn session_from_header_or_cookie(headers: &HeaderMap, expected: &str) -> bool {
    if session_from_header(headers, expected) {
        return true;
    }
    let prefix = format!("{SESSION_COOKIE}=");
    headers
        .get(header::COOKIE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|cookie| {
            cookie.split(';').any(|part| {
                part.trim()
                    .strip_prefix(&prefix)
                    .is_some_and(|value| value == expected)
            })
        })
}

pub(crate) fn unauthorized() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(json!({ "error": { "code": "unauthorized" } })),
    )
        .into_response()
}

pub(crate) fn forbidden_origin() -> Response {
    (
        StatusCode::FORBIDDEN,
        Json(json!({ "error": { "code": "forbidden_origin" } })),
    )
        .into_response()
}
