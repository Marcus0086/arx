fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Re-run if proto changes. Actual code generation requires protoc; see src/arx_gen.rs
    // for the pre-generated version used when protoc is not available.
    println!("cargo:rerun-if-changed=proto/arx.proto");
    println!("cargo:rerun-if-changed=src/arx_gen.rs");

    // Try to regenerate from proto (requires protoc in PATH or PROTOC env var).
    // If protoc is unavailable, the pre-generated src/arx_gen.rs is used instead.
    if std::env::var("PROTOC")
        .ok()
        .or_else(|| which_protoc())
        .is_some()
    {
        tonic_build::configure()
            .build_server(true)
            .build_client(false)
            .out_dir("src")
            .file_descriptor_set_path("src/arx.bin")
            .compile_protos(&["proto/arx.proto"], &["proto"])?;
        eprintln!("cargo:warning=protoc found, regenerated src/arx_gen.rs");
    } else {
        eprintln!(
            "cargo:warning=protoc not found — using pre-generated src/arx_gen.rs. \
             Install protoc (`brew install protobuf` or add to PATH) to regenerate."
        );
    }
    Ok(())
}

fn which_protoc() -> Option<String> {
    std::process::Command::new("protoc")
        .arg("--version")
        .output()
        .ok()
        .map(|_| "protoc".to_string())
}
