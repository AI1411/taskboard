use std::net::{Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::extract::{Request, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use include_dir::{include_dir, Dir};
use serde_json::json;
use taskboard_application::App;
use thiserror::Error;
use tokio::net::TcpListener;
use tower::ServiceExt;
use tower_http::services::ServeDir;

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

static EMBEDDED_WEB: Dir<'_> = include_dir!("$OUT_DIR/web-dist");

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

enum WebRoot {
    Dir(PathBuf),
    Embedded,
}

pub(crate) struct AppState {
    pub(crate) app: App,
    pub(crate) session: SessionToken,
    pub(crate) port: u16,
    web_root: WebRoot,
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
        web_root: resolve_web_root(),
    });

    let _ = open;
    tokio::spawn(async move {
        let _ = axum::serve(listener, router(state)).await;
    });
    Ok(bound)
}

fn resolve_web_root() -> WebRoot {
    if let Ok(dir) = std::env::var("TASKBOARD_WEB_DIST") {
        let path = PathBuf::from(dir);
        if path.is_dir() {
            return WebRoot::Dir(path);
        }
    }
    WebRoot::Embedded
}

fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", get(html_shell))
        .merge(crate::routes::api_router())
        .fallback(static_or_spa)
        .with_state(state)
}

fn set_session_cookie(headers: &mut HeaderMap, token: &str) {
    let cookie = format!("{SESSION_COOKIE}={token}; Path=/; HttpOnly; SameSite=Strict");
    headers.insert(
        header::SET_COOKIE,
        HeaderValue::from_str(&cookie).expect("session token is hex"),
    );
}

fn is_api_path(path: &str) -> bool {
    path == "/api" || path.starts_with("/api/")
}

fn read_index(root: &Path) -> Option<String> {
    std::fs::read_to_string(root.join("index.html")).ok()
}

fn embedded_index_html() -> String {
    EMBEDDED_WEB
        .get_file("index.html")
        .map(|file| String::from_utf8_lossy(file.contents()).into_owned())
        .unwrap_or_else(|| HTML_SHELL.to_string())
}

fn embedded_file(path: &str) -> Option<Response> {
    let rel = path.trim_start_matches('/');
    if rel.is_empty() || rel.contains("..") {
        return None;
    }
    let file = EMBEDDED_WEB.get_file(rel)?;
    let mime = mime_guess::from_path(file.path()).first_or_octet_stream();
    let mut headers = HeaderMap::new();
    if let Ok(value) = HeaderValue::from_str(mime.as_ref()) {
        headers.insert(header::CONTENT_TYPE, value);
    }
    Some((headers, file.contents()).into_response())
}

async fn html_shell(State(state): State<Arc<AppState>>) -> Response {
    let html = match &state.web_root {
        WebRoot::Dir(root) => read_index(root).unwrap_or_else(|| HTML_SHELL.to_string()),
        WebRoot::Embedded => embedded_index_html(),
    };
    let mut response = Html(html).into_response();
    set_session_cookie(response.headers_mut(), &state.session.0);
    response
}

async fn static_or_spa(State(state): State<Arc<AppState>>, req: Request) -> Response {
    let path = req.uri().path().to_string();
    if is_api_path(&path) {
        return StatusCode::NOT_FOUND.into_response();
    }
    match &state.web_root {
        WebRoot::Dir(root) => match ServeDir::new(root).oneshot(req).await {
            Ok(res) if res.status() != StatusCode::NOT_FOUND => res.into_response(),
            _ => match read_index(root) {
                Some(html) => Html(html).into_response(),
                None => StatusCode::NOT_FOUND.into_response(),
            },
        },
        WebRoot::Embedded => {
            if let Some(res) = embedded_file(&path) {
                return res;
            }
            Html(embedded_index_html()).into_response()
        }
    }
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
