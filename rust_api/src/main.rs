mod auth;
mod config;
mod db;
mod error;
mod job_runner;
mod models;
mod routes;

use std::collections::HashSet;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::DefaultBodyLimit;
use axum::middleware;
use axum::routing::{get, post};
use axum::{Router};
use tokio::sync::{Mutex, RwLock, Semaphore};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::info;

use crate::config::AppConfig;
use crate::db::Db;
use crate::routes::jobs;
use crate::routes::uploads;
use crate::routes::health;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AppConfig>,
    pub db: Arc<Db>,
    pub downloads_lock: Arc<Mutex<()>>,
    pub canceled_jobs: Arc<RwLock<HashSet<String>>>,
    pub job_slots: Arc<Semaphore>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "rust_api=info,tower_http=info".into()),
        )
        .init();

    let config = Arc::new(AppConfig::from_env()?);
    let db = Arc::new(Db::new(config.jobs_db_path.clone()));
    db.init()?;

    let state = AppState {
        config: config.clone(),
        db,
        downloads_lock: Arc::new(Mutex::new(())),
        canceled_jobs: Arc::new(RwLock::new(HashSet::new())),
        job_slots: Arc::new(Semaphore::new(config.max_running_jobs)),
    };

    let api_routes = Router::new()
        .route(
            "/api/v1/uploads",
            post(uploads::upload_pdf).layer(DefaultBodyLimit::disable()),
        )
        .route("/api/v1/jobs", post(jobs::create_job).get(jobs::list_jobs))
        .route("/api/v1/jobs/:job_id", get(jobs::get_job))
        .route("/api/v1/jobs/:job_id/artifacts", get(jobs::get_job_artifacts))
        .route("/api/v1/jobs/:job_id/pdf", get(jobs::download_pdf))
        .route("/api/v1/jobs/:job_id/markdown", get(jobs::download_markdown))
        .route("/api/v1/jobs/:job_id/markdown/images/*path", get(jobs::download_markdown_image))
        .route("/api/v1/jobs/:job_id/download", get(jobs::download_bundle))
        .route("/api/v1/jobs/:job_id/cancel", post(jobs::cancel_job))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            auth::require_api_key,
        ));

    let app = Router::new()
        .route("/health", get(health::health))
        .merge(api_routes)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state.clone());

    let simple_app = Router::new()
        .route("/health", get(health::health))
        .route(
            "/api/v1/translate/bundle",
            post(jobs::translate_bundle).layer(DefaultBodyLimit::disable()),
        )
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            auth::require_api_key,
        ))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    let simple_addr = SocketAddr::from(([0, 0, 0, 0], config.simple_port));
    info!(
        "rust_api auth enabled: {} keys, max running jobs: {}",
        config.api_keys.len(),
        config.max_running_jobs
    );
    info!("rust_api full api listening on {}", addr);
    info!("rust_api simple api listening on {}", simple_addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    let simple_listener = tokio::net::TcpListener::bind(simple_addr).await?;
    let full_server = axum::serve(listener, app);
    let simple_server = axum::serve(simple_listener, simple_app);
    tokio::try_join!(full_server, simple_server)?;
    Ok(())
}
