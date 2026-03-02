fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Compile all proto files
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos(
            &[
                "proto/repository.proto",
                "proto/ref.proto",
                "proto/commit.proto",
                "proto/blob.proto",
                "proto/tree.proto",
                "proto/diff.proto",
                "proto/smarthttp.proto",
                "proto/operations.proto",
                "proto/health.proto",
            ],
            &["proto"],
        )?;
    
    Ok(())
}
