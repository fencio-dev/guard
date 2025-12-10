fn main() -> Result<(), Box<dyn std::error::Error>> {
    let proto_path = if std::path::Path::new("/rust-build/data_plane/proto").exists() {
        // Docker build path
        "/rust-build/data_plane/proto/rule_installation.proto"
    } else {
        // Local development path
        "../../proto/rule_installation.proto"
    };

    let include_path = if std::path::Path::new("/rust-build/data_plane/proto").exists() {
        "/rust-build/data_plane/proto"
    } else {
        "../../proto"
    };

    tonic_build::configure()
        .build_server(true)
        .build_client(false)
        .compile_protos(&[proto_path], &[include_path])?;
    Ok(())
}
