mod auth;
mod db;
mod server;
mod store;

// Pre-generated prost/tonic types for arx.proto.
// To regenerate: install protoc and run `cargo build -p arx-grpc`.
#[path = "arx_gen.rs"]
pub mod arx;

use std::path::PathBuf;
use std::sync::Arc;

use http::Method;
use tonic::transport::Server;
use tonic_web::GrpcWebLayer;
use tower_http::cors::{Any, CorsLayer};

use arx::arx_service_server::ArxServiceServer;
use db::AuthDb;
use server::ArxServiceImpl;
use store::ArchiveStore;

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
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(50051);

    let root_dir = std::env::var("ROOT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| default_root_dir());

    let admin_key = std::env::var("ARX_ADMIN_KEY").unwrap_or_default();
    if admin_key.is_empty() {
        eprintln!(
            "warn: ARX_ADMIN_KEY not set — admin RPCs (CreateTenant, CreateUser, etc.) are disabled"
        );
    }

    std::fs::create_dir_all(&root_dir).map_err(|e| {
        eprintln!(
            "error: cannot create ROOT_DIR {:?}: {e}\n\
             Set ROOT_DIR env var to a writable directory.",
            root_dir
        );
        e
    })?;

    let db_path = root_dir.join("arx.db");
    let db = Arc::new(
        AuthDb::open(&db_path)
            .await
            .map_err(|e| format!("failed to open auth DB: {e}"))?,
    );

    let store = Arc::new(ArchiveStore::new(root_dir.clone()));

    let addr = format!("0.0.0.0:{port}").parse()?;
    eprintln!("arx-grpc listening on {addr}");
    eprintln!("  root: {root_dir:?}");
    eprintln!("  db:   {db_path:?}");

    let svc = ArxServiceServer::new(ArxServiceImpl {
        store,
        db,
        admin_key: Arc::new(admin_key),
    });

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::POST, Method::GET, Method::OPTIONS])
        .allow_headers(Any)
        .expose_headers(Any);

    Server::builder()
        .accept_http1(true) // required for grpc-web over HTTP/1.1
        .layer(cors)
        .layer(GrpcWebLayer::new()) // enables grpc-web from browsers
        .add_service(svc)
        .serve_with_shutdown(addr, shutdown_signal())
        .await?;

    eprintln!("arx-grpc shutdown complete");
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
        _ = ctrl_c => eprintln!("arx-grpc received Ctrl+C, shutting down…"),
        _ = sigterm => eprintln!("arx-grpc received SIGTERM, shutting down…"),
    }
}
