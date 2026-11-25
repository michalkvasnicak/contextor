use std::net::SocketAddr;
use std::path::PathBuf;

use anyhow::{Context, Result};
use axum::body::Bytes;
use axum::extract::{Query, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::{Extension, Json};
use serde::Deserialize;
use tokio::net::TcpListener;
use tokio::runtime::Builder;
use utoipa::{IntoParams, OpenApi};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{DEFAULT_LIMIT, Format, index, write_text_results};

#[derive(OpenApi)]
#[openapi(info(
    title = "Contextor API",
    version = "1.0.0",
    description = "Read-only access to the last completed project index."
))]
struct ApiDoc;

#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
struct QueryParams {
    /// Exact, case-sensitive feature identifier.
    #[param(example = "billing")]
    identifier: String,
    /// Maximum number of results.
    #[param(default = default_limit, minimum = 0, maximum = 4294967295u32)]
    limit: Option<u32>,
    /// Response format.
    #[param(default = Format::default)]
    #[serde(default)]
    format: Format,
}

fn default_limit() -> u32 {
    DEFAULT_LIMIT
}

#[derive(Clone)]
struct OpenApiDocument(Bytes);

pub fn serve(root: PathBuf, listen: SocketAddr) -> Result<()> {
    Builder::new_current_thread().enable_all().build()?.block_on(async {
        let (router, document) = OpenApiRouter::with_openapi(ApiDoc::openapi())
            .routes(routes!(openapi))
            .routes(routes!(query))
            .split_for_parts();
        let document = serde_json::to_vec(&document).context("serializing OpenAPI document")?;
        let app = router
            .layer(Extension(OpenApiDocument(document.into())))
            .with_state(root);
        let listener = TcpListener::bind(listen)
            .await
            .with_context(|| format!("binding HTTP server to {listen}"))?;
        eprintln!("Contextor listening on http://{}", listener.local_addr()?);
        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal())
            .await?;
        Ok(())
    })
}

/// Get the generated OpenAPI document.
#[utoipa::path(
    get,
    path = "/v1/openapi",
    responses(
        (status = 200, description = "The OpenAPI document.", body = Object, content_type = "application/json")
    )
)]
async fn openapi(Extension(document): Extension<OpenApiDocument>) -> impl IntoResponse {
    ([(header::CONTENT_TYPE, "application/json")], document.0)
}

/// Find targets tagged with a feature identifier.
#[utoipa::path(
    get,
    path = "/v1/query",
    params(QueryParams),
    responses(
        (status = 200, description = "Matching targets in the requested format.", content(
            (String = "text/plain"),
            (Vec<crate::Match> = "application/json")
        )),
        (status = 400, description = "Missing or invalid query parameters.", body = String),
        (status = 500, description = "The server could not query the index.", body = String)
    )
)]
async fn query(State(root): State<PathBuf>, Query(params): Query<QueryParams>) -> Response {
    let QueryParams { identifier, limit, format } = params;
    let results = tokio::task::spawn_blocking(move || {
        index::query(&root, &identifier, limit.unwrap_or_else(default_limit))
    })
    .await
    .context("running query task")
    .and_then(|result| result);

    match results {
        Ok(matches) => match format {
            Format::Json => Json(matches).into_response(),
            Format::Text => {
                let mut body = Vec::new();
                match write_text_results(&mut body, &matches) {
                    Ok(()) => ([(header::CONTENT_TYPE, "text/plain; charset=utf-8")], body)
                        .into_response(),
                    Err(error) => query_error(error.into()),
                }
            }
        },
        Err(error) => query_error(error),
    }
}

fn query_error(error: anyhow::Error) -> Response {
    eprintln!("HTTP query failed: {error:#}");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        "Could not query the index. Check the server logs.",
    )
        .into_response()
}

async fn shutdown_signal() {
    if let Err(error) = tokio::signal::ctrl_c().await {
        eprintln!("Could not install Ctrl+C handler: {error}");
    }
}
