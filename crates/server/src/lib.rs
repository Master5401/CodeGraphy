#![forbid(unsafe_code)]

use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU16, Ordering};
use std::sync::{Arc, RwLock};

use anyhow::{Context, Result, anyhow};
use axum::extract::{Path as AxumPath, Query, State};
use axum::http::header::{
    CACHE_CONTROL, CONTENT_SECURITY_POLICY, HOST, HeaderValue, ORIGIN, REFERER,
    X_CONTENT_TYPE_OPTIONS,
};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use codewiki_core::catalog::AnalysisFactQuery;
use codewiki_core::{CatalogProgress, SourcePreview, WorkspaceStore};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::net::TcpListener;
use tokio::sync::watch;
use tokio::task::JoinHandle;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::set_header::SetResponseHeaderLayer;

#[derive(Clone)]
pub struct ServerState {
    workspace: Arc<RwLock<Option<WorkspaceStore>>>,
    progress: watch::Sender<RuntimeProgress>,
    scan_active: Arc<AtomicBool>,
    session_token: Arc<String>,
    port: Arc<AtomicU16>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct RuntimeProgress {
    pub state: String,
    pub files_seen: u64,
    pub files_indexed: u64,
    pub text_indexed: u64,
    pub diagnostics: u64,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct WorkspaceStatus {
    pub open: bool,
    pub root: Option<String>,
    pub indexed_files: u64,
    pub progress: RuntimeProgress,
}

#[derive(Debug, Serialize)]
pub struct AnalysisOverview {
    pub run: Option<Value>,
    pub calls: Vec<Value>,
    pub unresolved_calls: Vec<Value>,
}

#[derive(Deserialize)]
pub struct WorkspaceRequest {
    pub path: String,
}

#[derive(Deserialize)]
pub struct FilesQuery {
    pub q: Option<String>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[derive(Deserialize)]
pub struct TreeQuery {
    pub path: Option<String>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[derive(Debug, Serialize)]
struct ApiErrorBody {
    error: String,
}

impl ServerState {
    pub fn new(session_token: String) -> Self {
        let (progress, _) = watch::channel(RuntimeProgress {
            state: "project-picker".to_owned(),
            ..RuntimeProgress::default()
        });
        Self {
            workspace: Arc::new(RwLock::new(None)),
            progress,
            scan_active: Arc::new(AtomicBool::new(false)),
            session_token: Arc::new(session_token),
            port: Arc::new(AtomicU16::new(0)),
        }
    }

    pub fn set_port(&self, port: u16) {
        self.port.store(port, Ordering::Relaxed);
    }

    pub fn start_index(&self, store: WorkspaceStore) -> Result<JoinHandle<()>> {
        if self
            .scan_active
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err(anyhow!("workspace indexing is already in progress"));
        }

        *self
            .workspace
            .write()
            .map_err(|_| anyhow!("workspace state is unavailable"))? = Some(store.clone());
        self.progress.send_replace(RuntimeProgress {
            state: "indexing".to_owned(),
            ..RuntimeProgress::default()
        });

        let progress = self.progress.clone();
        let scan_active = self.scan_active.clone();
        Ok(tokio::task::spawn_blocking(move || {
            let result = store.index_workspace(|item| {
                progress.send_replace(runtime_progress("indexing", item, None));
            });
            match result {
                Ok(report) => {
                    progress.send_replace(RuntimeProgress {
                        state: "ready".to_owned(),
                        files_seen: report.files_seen,
                        files_indexed: report.files_indexed,
                        text_indexed: report.text_indexed,
                        diagnostics: report.diagnostics,
                        error: None,
                    });
                }
                Err(error) => {
                    progress.send_replace(RuntimeProgress {
                        state: "failed".to_owned(),
                        error: Some(format!("{error:#}")),
                        ..RuntimeProgress::default()
                    });
                }
            }
            scan_active.store(false, Ordering::Release);
        }))
    }

    fn store(&self) -> Result<WorkspaceStore, ApiError> {
        self.workspace
            .read()
            .map_err(|_| ApiError::internal("workspace state is unavailable"))?
            .clone()
            .ok_or_else(|| ApiError::conflict("open a workspace to continue"))
    }

    fn progress(&self) -> RuntimeProgress {
        self.progress.borrow().clone()
    }

    fn authorize(&self, headers: &HeaderMap) -> Result<(), ApiError> {
        let port = self.port.load(Ordering::Relaxed);
        let expected_host = format!("127.0.0.1:{port}");
        let host = headers
            .get(HOST)
            .and_then(|value| value.to_str().ok())
            .ok_or_else(|| ApiError::forbidden("missing Host header"))?;
        if host != expected_host {
            return Err(ApiError::forbidden("Host header is not allowed"));
        }

        let expected_origin = format!("http://{expected_host}");
        let origin = headers.get(ORIGIN).and_then(|value| value.to_str().ok());
        let origin_is_allowed = match origin {
            Some(origin) => origin == expected_origin,
            None => headers
                .get(REFERER)
                .and_then(|value| value.to_str().ok())
                .is_some_and(|referer| referer.starts_with(&format!("{expected_origin}/"))),
        };
        if !origin_is_allowed {
            return Err(ApiError::forbidden("Origin is not allowed"));
        }

        let provided = headers
            .get("x-codewiki-token")
            .and_then(|value| value.to_str().ok())
            .ok_or_else(|| ApiError::unauthorized("missing session token"))?;
        if provided != self.session_token.as_str() {
            return Err(ApiError::unauthorized("invalid session token"));
        }
        Ok(())
    }
}

fn runtime_progress(
    state: &str,
    progress: CatalogProgress,
    error: Option<String>,
) -> RuntimeProgress {
    RuntimeProgress {
        state: state.to_owned(),
        files_seen: progress.files_seen,
        files_indexed: progress.files_indexed,
        text_indexed: progress.text_indexed,
        diagnostics: progress.diagnostics,
        error,
    }
}

pub async fn serve(
    listener: TcpListener,
    state: ServerState,
    static_root: impl AsRef<Path>,
) -> Result<()> {
    let static_root = static_root.as_ref().to_path_buf();
    let index_path = static_root.join("index.html");
    if !index_path.is_file() {
        return Err(anyhow!(
            "web build not found at {}; run `npm run build` in web/",
            index_path.display()
        ));
    }

    let app = Router::new()
        .route("/api/status", get(status))
        .route("/api/workspace", post(open_workspace))
        .route("/api/files", get(files))
        .route("/api/tree", get(tree))
        .route("/api/analysis", get(analysis))
        .route("/api/source/{file_id}", get(source))
        .with_state(state)
        .fallback_service(
            ServeDir::new(static_root)
                .append_index_html_on_directories(true)
                .not_found_service(ServeFile::new(index_path)),
        )
        .layer(SetResponseHeaderLayer::if_not_present(
            X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            CONTENT_SECURITY_POLICY,
            HeaderValue::from_static(
                "default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' data:; connect-src 'self'; base-uri 'none'; frame-ancestors 'none'; form-action 'none'",
            ),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            CACHE_CONTROL,
            HeaderValue::from_static("no-store"),
        ));

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("local CodeGraphy server failed")
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}

async fn status(State(state): State<ServerState>, headers: HeaderMap) -> Response {
    if let Err(error) = state.authorize(&headers) {
        return error.into_response();
    }
    let progress = state.progress();
    match state.workspace.read() {
        Ok(workspace) => match workspace.as_ref() {
            Some(store) => match store.file_count() {
                Ok(indexed_files) => Json(WorkspaceStatus {
                    open: true,
                    root: Some(store.root().to_string_lossy().into_owned()),
                    indexed_files,
                    progress,
                })
                .into_response(),
                Err(error) => {
                    ApiError::internal(format!("could not read index: {error:#}")).into_response()
                }
            },
            None => Json(WorkspaceStatus {
                open: false,
                root: None,
                indexed_files: 0,
                progress,
            })
            .into_response(),
        },
        Err(_) => ApiError::internal("workspace state is unavailable").into_response(),
    }
}

async fn open_workspace(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(request): Json<WorkspaceRequest>,
) -> Response {
    if let Err(error) = state.authorize(&headers) {
        return error.into_response();
    }
    if state.scan_active.load(Ordering::Acquire) {
        return ApiError::conflict("indexing is already in progress").into_response();
    }
    let store = match WorkspaceStore::open(&request.path) {
        Ok(store) => store,
        Err(error) => {
            return ApiError::bad_request(format!("cannot open workspace: {error:#}"))
                .into_response();
        }
    };
    match state.start_index(store) {
        Ok(_) => (StatusCode::ACCEPTED, Json(state.progress())).into_response(),
        Err(error) => {
            ApiError::conflict(format!("could not start indexing: {error:#}")).into_response()
        }
    }
}

async fn files(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Query(query): Query<FilesQuery>,
) -> Response {
    if let Err(error) = state.authorize(&headers) {
        return error.into_response();
    }
    let store = match state.store() {
        Ok(store) => store,
        Err(error) => return error.into_response(),
    };
    let limit = query.limit.unwrap_or(50).clamp(1, 100);
    let offset = query.offset.unwrap_or(0);
    let result = match query.q.as_deref().filter(|value| !value.trim().is_empty()) {
        Some(query) => store.search_files(query, limit, offset),
        None => store.list_files(limit, offset),
    };
    match result {
        Ok(hits) => Json(hits).into_response(),
        Err(error) => ApiError::internal(format!("file query failed: {error:#}")).into_response(),
    }
}

async fn tree(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Query(query): Query<TreeQuery>,
) -> Response {
    if let Err(error) = state.authorize(&headers) {
        return error.into_response();
    }
    let store = match state.store() {
        Ok(store) => store,
        Err(error) => return error.into_response(),
    };
    let result = store.list_directory(
        query.path.as_deref().unwrap_or_default(),
        query.limit.unwrap_or(200),
        query.offset.unwrap_or(0),
    );
    match result {
        Ok(listing) => Json(listing).into_response(),
        Err(error) => {
            ApiError::bad_request(format!("directory query failed: {error:#}")).into_response()
        }
    }
}

async fn source(
    State(state): State<ServerState>,
    headers: HeaderMap,
    AxumPath(file_id): AxumPath<String>,
) -> Response {
    if let Err(error) = state.authorize(&headers) {
        return error.into_response();
    }
    let store = match state.store() {
        Ok(store) => store,
        Err(error) => return error.into_response(),
    };
    match store.read_source_preview(&file_id) {
        Ok(preview) => Json::<SourcePreview>(preview).into_response(),
        Err(error) => {
            ApiError::not_found(format!("source preview unavailable: {error:#}")).into_response()
        }
    }
}

async fn analysis(State(state): State<ServerState>, headers: HeaderMap) -> Response {
    if let Err(error) = state.authorize(&headers) {
        return error.into_response();
    }
    let store = match state.store() {
        Ok(store) => store,
        Err(error) => return error.into_response(),
    };
    let result = (|| {
        Ok::<_, anyhow::Error>(AnalysisOverview {
            run: store.analysis_run_summary("atlas-engine", "java-structural")?,
            calls: store.query_analysis_facts(AnalysisFactQuery {
                provider: "atlas-engine",
                profile: "java-structural",
                fact_kind: "relation",
                relation_kind: Some("calls"),
                resolution_state: None,
                limit: 100,
                offset: 0,
            })?,
            unresolved_calls: store.query_analysis_facts(AnalysisFactQuery {
                provider: "atlas-engine",
                profile: "java-structural",
                fact_kind: "unresolved_call",
                relation_kind: None,
                resolution_state: None,
                limit: 100,
                offset: 0,
            })?,
        })
    })();
    match result {
        Ok(overview) => Json(overview).into_response(),
        Err(error) => {
            ApiError::internal(format!("analysis query failed: {error:#}")).into_response()
        }
    }
}

#[derive(Debug)]
struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: message.into(),
        }
    }

    fn unauthorized(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::UNAUTHORIZED,
            message: message.into(),
        }
    }

    fn forbidden(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::FORBIDDEN,
            message: message.into(),
        }
    }

    fn not_found(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            message: message.into(),
        }
    }

    fn conflict(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::CONFLICT,
            message: message.into(),
        }
    }

    fn internal(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: message.into(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(ApiErrorBody {
                error: self.message,
            }),
        )
            .into_response()
    }
}

#[cfg(test)]
mod tests {
    use axum::http::HeaderMap;

    use super::ServerState;

    fn valid_headers(token: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert("host", "127.0.0.1:43121".parse().expect("valid host"));
        headers.insert(
            "origin",
            "http://127.0.0.1:43121".parse().expect("valid origin"),
        );
        headers.insert("x-codewiki-token", token.parse().expect("valid token"));
        headers
    }

    #[test]
    fn api_requires_loopback_host_origin_and_session_token() {
        let state = ServerState::new("session-secret".to_owned());
        state.set_port(43121);
        assert!(state.authorize(&valid_headers("session-secret")).is_ok());
        assert!(state.authorize(&valid_headers("wrong")).is_err());

        let mut wrong_origin = valid_headers("session-secret");
        wrong_origin.insert(
            "origin",
            "https://example.com".parse().expect("valid origin"),
        );
        assert!(state.authorize(&wrong_origin).is_err());
    }
}
