mod auth;
mod db;
mod server;
mod store;
mod upload;

// Pre-generated prost/tonic types for arx.proto.
// To regenerate: install protoc and run `cargo build -p arx-grpc`.
#[path = "arx_gen.rs"]
pub mod arx;

use std::path::PathBuf;
use std::sync::Arc;

use axum::{
    Router,
    routing::{get, post},
};
use dashmap::DashMap;
use http::{HeaderValue, Method};
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tonic::service::Routes;
use tonic_web::GrpcWebLayer;
use tower_http::cors::{Any, CorsLayer};

use arx::arx_service_server::ArxServiceServer;
use db::AuthDb;
use server::ArxServiceImpl;
use store::ArchiveStore;
use upload::{UploadState, body_limit, handle_upload};

/// Default data directory: $HOME/.arx-grpc or ./.arx-grpc
fn default_root_dir() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        let p = PathBuf::from(home).join(".arx-grpc");
        if std::fs::create_dir_all(&p).is_ok() {
            return p;
        }
    }
    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(".arx-grpc")
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Structured logging — respects RUST_LOG env var (default: info).
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(50051);

    let root_dir = std::env::var("ROOT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| default_root_dir());

    let admin_key = std::env::var("ARX_ADMIN_KEY").unwrap_or_default();
    if admin_key.is_empty() {
        tracing::warn!(
            "ARX_ADMIN_KEY not set — admin RPCs (CreateTenant, CreateUser, etc.) are disabled"
        );
    }

    std::fs::create_dir_all(&root_dir).map_err(|e| {
        tracing::error!("cannot create ROOT_DIR {:?}: {e}", root_dir);
        e
    })?;

    let db_path = root_dir.join("arx.db");
    let db = Arc::new(
        AuthDb::open(&db_path)
            .await
            .map_err(|e| format!("failed to open auth DB: {e}"))?,
    );

    let store = Arc::new(ArchiveStore::new(root_dir.clone()));

    // Shared per-archive write locks — prevents concurrent writes corrupting an archive.
    let archive_locks: Arc<DashMap<String, Arc<Mutex<()>>>> = Arc::new(DashMap::new());

    let addr: std::net::SocketAddr = format!("0.0.0.0:{port}").parse()?;
    tracing::info!("arx-grpc listening on {addr}");
    tracing::info!("  root: {root_dir:?}");
    tracing::info!("  db:   {db_path:?}");

    let svc = ArxServiceServer::new(ArxServiceImpl {
        store: Arc::clone(&store),
        db: Arc::clone(&db),
        admin_key: Arc::new(admin_key),
        archive_locks: Arc::clone(&archive_locks),
    });

    // CORS: lock to CORS_ORIGIN env var in production; fall back to Any for local dev.
    let cors = {
        let base = CorsLayer::new()
            .allow_methods([Method::POST, Method::GET, Method::OPTIONS])
            .allow_headers(Any)
            .expose_headers(Any);
        if let Ok(origin) = std::env::var("CORS_ORIGIN") {
            match origin.parse::<HeaderValue>() {
                Ok(hv) => {
                    tracing::info!("CORS restricted to origin: {origin}");
                    base.allow_origin(hv)
                }
                Err(_) => {
                    tracing::warn!("CORS_ORIGIN is not a valid header value, falling back to Any");
                    base.allow_origin(Any)
                }
            }
        } else {
            tracing::warn!("CORS_ORIGIN not set — allowing all origins (development mode)");
            base.allow_origin(Any)
        }
    };

    // gRPC service as an axum Router, wrapped in GrpcWebLayer so browsers can speak grpc-web.
    let grpc_router = Routes::new(svc)
        .prepare()
        .into_axum_router()
        .layer(GrpcWebLayer::new());

    let upload_state = UploadState {
        store: Arc::clone(&store),
        db: Arc::clone(&db),
        archive_locks: Arc::clone(&archive_locks),
    };

    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/api/upload/{archive_id}", post(handle_upload))
        .layer(body_limit())
        .with_state(upload_state)
        .merge(grpc_router)
        .layer(cors);

    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    tracing::info!("arx-grpc shutdown complete");
    Ok(())
}

async fn shutdown_signal() {
    use tokio::signal;

    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let sigterm = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let sigterm = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => tracing::info!("received Ctrl+C, shutting down…"),
        _ = sigterm => tracing::info!("received SIGTERM, shutting down successfully…"),
    }
}
