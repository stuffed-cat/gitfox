//! Build script for compiling proto files
//! 主应用作为 gRPC 服务端提供权限认证服务和 LFS 服务

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 编译 auth.proto - 权限认证服务
    // 编译 lfs.proto - LFS 服务
    tonic_build::configure()
        .build_server(true)  // 主应用作为服务端
        .build_client(false) // 不需要客户端
        .compile_protos(
            &[
                "gitlayer/proto/auth.proto",
                "gitlayer/proto/lfs.proto",
            ],
            &["gitlayer/proto"],
        )?;

    // 当 proto 文件变化时重新编译
    println!("cargo:rerun-if-changed=gitlayer/proto/auth.proto");
    println!("cargo:rerun-if-changed=gitlayer/proto/lfs.proto");

    Ok(())
}
