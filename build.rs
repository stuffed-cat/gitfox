//! Build script for compiling proto files
//! 主应用作为 gRPC 服务端提供权限认证服务和 LFS 服务
//! 主应用作为 gRPC 客户端调用 GitLayer 服务

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 编译 auth.proto - 权限认证服务（作为服务端）
    // 编译 lfs.proto - LFS 服务（作为服务端）
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

    // 编译 GitLayer 服务（作为客户端）
    // 包括所有 Git 操作服务
    tonic_build::configure()
        .build_server(false) // 不作为服务端
        .build_client(true)  // 主应用作为客户端调用 GitLayer
        .compile_protos(
            &[
                "gitlayer/proto/repository.proto",
                "gitlayer/proto/ref.proto",
                "gitlayer/proto/commit.proto",
                "gitlayer/proto/tree.proto",
                "gitlayer/proto/blob.proto",
                "gitlayer/proto/diff.proto",
                "gitlayer/proto/gpg.proto",
                "gitlayer/proto/operations.proto",
            ],
            &["gitlayer/proto"],
        )?;

    // 当 proto 文件变化时重新编译
    println!("cargo:rerun-if-changed=gitlayer/proto/auth.proto");
    println!("cargo:rerun-if-changed=gitlayer/proto/lfs.proto");
    println!("cargo:rerun-if-changed=gitlayer/proto/repository.proto");
    println!("cargo:rerun-if-changed=gitlayer/proto/ref.proto");
    println!("cargo:rerun-if-changed=gitlayer/proto/commit.proto");
    println!("cargo:rerun-if-changed=gitlayer/proto/tree.proto");
    println!("cargo:rerun-if-changed=gitlayer/proto/blob.proto");
    println!("cargo:rerun-if-changed=gitlayer/proto/diff.proto");
    println!("cargo:rerun-if-changed=gitlayer/proto/gpg.proto");
    println!("cargo:rerun-if-changed=gitlayer/proto/operations.proto");

    Ok(())
}
