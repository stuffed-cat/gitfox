fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Compile GitLayer proto definitions for client usage
    let proto_files = [
        "../gitlayer/proto/repository.proto",
        "../gitlayer/proto/ref.proto", 
        "../gitlayer/proto/commit.proto",
        "../gitlayer/proto/blob.proto",
        "../gitlayer/proto/tree.proto",
        "../gitlayer/proto/diff.proto",
        "../gitlayer/proto/smarthttp.proto",
        "../gitlayer/proto/operations.proto",
        "../gitlayer/proto/health.proto",
        "../gitlayer/proto/auth.proto",
        "../gitlayer/proto/lfs.proto",
    ];
    
    // Only compile if proto files exist
    let first_proto = std::path::Path::new(&proto_files[0]);
    if first_proto.exists() {
        tonic_build::configure()
            .build_server(false)  // Only need client
            .compile_protos(&proto_files, &["../gitlayer/proto"])?;
    }
    
    Ok(())
}
