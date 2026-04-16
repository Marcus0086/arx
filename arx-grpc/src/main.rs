mod auth;
mod server;
mod store;

// Pre-generated prost/tonic types for arx.proto.
// To regenerate: install protoc and run `cargo build -p arx-grpc`.
#[path = "arx_gen.rs"]
pub mod arx;

use std::path::PathBuf;
use std::sync::Arc;

use tonic::transport::Server;

use arx::arx_service_server::ArxServiceServer;
use auth::TenantStore;
use server::ArxServiceImpl;
use store::ArchiveStore;

/// Default data directory resolution order:
///   1. $HOME/.arx-grpc  (preferred — keeps data out of the project tree)
///   2. ./.arx-grpc      (fallback — relative to CWD, always writable)
fn default_root_dir() -> PathBuf {
    // Try $HOME first
    if let Ok(home) = std::env::var("HOME") {
        let p = PathBuf::from(home).join(".arx-grpc");
        // Only use it if we can actually create/access it
        if std::fs::create_dir_all(&p).is_ok() {
            return p;
        }
    }
    // Fall back to CWD-relative path
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

    let tenants_path = std::env::var("TENANTS_JSON")
        .map(PathBuf::from)
        .unwrap_or_else(|_| root_dir.join("tenants.json"));

    std::fs::create_dir_all(&root_dir).map_err(|e| {
        eprintln!(
            "error: cannot create ROOT_DIR {:?}: {e}\n\
             Set ROOT_DIR env var to a writable directory, e.g.:\n\
             ROOT_DIR=/tmp/arx-data cargo run -p arx-grpc",
            root_dir
        );
        e
    })?;

    let tenants = Arc::new(TenantStore::load(&tenants_path));
    let store = Arc::new(ArchiveStore::new(root_dir.clone()));

    let addr = format!("0.0.0.0:{port}").parse()?;
    eprintln!("arx-grpc listening on {addr}");
    eprintln!("  root:    {root_dir:?}");
    eprintln!("  tenants: {tenants_path:?}");

    let svc = ArxServiceServer::new(ArxServiceImpl { store, tenants });
    Server::builder()
        .add_service(svc)
        .serve(addr)
        .await?;

    Ok(())
}
