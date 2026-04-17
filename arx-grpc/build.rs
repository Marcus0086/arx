fn main() {
    // Re-run if proto changes. Code is pre-generated in src/arx_gen.rs.
    // Regenerate with: protoc installed + PROTOC=$(which protoc) cargo build -p arx-grpc
    println!("cargo:rerun-if-changed=proto/arx.proto");
    println!("cargo:rerun-if-changed=src/arx_gen.rs");
}
