//! SmartHttp 服务 benchmark
//!
//! 测试 receive_pack gRPC 流式传输性能：
//! 1. 单个大 chunk（模拟旧的内存收集方式）
//! 2. 流式多个小 chunk（当前优化的实现方式）
//!
//! 测试证明流式传输在大数据量下的性能优势

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::net::SocketAddr;
use std::process::Command as StdCommand;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::net::TcpListener;
use tokio::runtime::Runtime;
use tokio_stream::StreamExt;
use tonic::transport::{Channel, Server};

use gitlayer::config::Config;
use gitlayer::proto::smart_http_service_client::SmartHttpServiceClient;
use gitlayer::proto::smart_http_service_server::SmartHttpServiceServer;
use gitlayer::proto::*;
use gitlayer::services::smarthttp::SmartHttpServiceImpl;

/// 创建测试用的 bare git 仓库
fn create_test_repo() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let repo_path = temp_dir.path();

    StdCommand::new("git")
        .args(["init", "--bare"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to init bare repo");

    let work_dir = TempDir::new().expect("Failed to create work dir");
    let work_path = work_dir.path();

    StdCommand::new("git")
        .args(["init"])
        .current_dir(work_path)
        .output()
        .expect("Failed to init work repo");

    StdCommand::new("git")
        .args(["config", "user.email", "bench@test.com"])
        .current_dir(work_path)
        .output()
        .ok();

    StdCommand::new("git")
        .args(["config", "user.name", "Benchmark"])
        .current_dir(work_path)
        .output()
        .ok();

    std::fs::write(work_path.join("README.md"), "# Test").expect("Failed to write");

    StdCommand::new("git")
        .args(["add", "-A"])
        .current_dir(work_path)
        .output()
        .expect("Failed to git add");

    StdCommand::new("git")
        .args(["commit", "-m", "initial"])
        .current_dir(work_path)
        .output()
        .expect("Failed to git commit");

    StdCommand::new("git")
        .args([
            "push",
            repo_path.to_str().unwrap(),
            "HEAD:refs/heads/main",
        ])
        .current_dir(work_path)
        .output()
        .expect("Failed to push");

    temp_dir
}

/// 生成随机测试数据（模拟 pack 数据）
fn generate_test_data(size_kb: usize) -> Vec<u8> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    let mut data = Vec::with_capacity(size_kb * 1024);
    let mut hasher = DefaultHasher::new();
    
    for i in 0..(size_kb * 1024 / 8) {
        i.hash(&mut hasher);
        data.extend_from_slice(&hasher.finish().to_le_bytes());
    }
    
    // 截断到精确大小
    data.truncate(size_kb * 1024);
    data
}

/// 创建测试配置
fn create_test_config(storage_path: &std::path::Path) -> Config {
    Config {
        listen_addr: "127.0.0.1:0".to_string(),
        storage_path: storage_path.to_str().unwrap().to_string(),
        git_bin_path: "git".to_string(),
        max_concurrent_ops: 10,
        enable_cache: false,
        cache_ttl_secs: 0,
    }
}

/// 启动临时 gRPC 服务器
async fn start_test_server(config: Arc<Config>) -> (SocketAddr, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let service = SmartHttpServiceImpl::new(config);
    let handle = tokio::spawn(async move {
        Server::builder()
            .add_service(SmartHttpServiceServer::new(service))
            .serve_with_incoming(tokio_stream::wrappers::TcpListenerStream::new(listener))
            .await
            .ok();
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    (addr, handle)
}

/// Benchmark: 测试 gRPC 流式传输性能
/// 对比单个大消息 vs 分块流式传输
fn bench_grpc_streaming(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("grpc_streaming");
    group.sample_size(10);

    // 使用测试仓库（虽然 receive_pack 可能会失败，但我们测的是传输性能）
    let temp_repo = create_test_repo();
    let repo_path = temp_repo.path();
    let config = Arc::new(create_test_config(repo_path));

    let (addr, _server_handle) = rt.block_on(start_test_server(config.clone()));
    let endpoint = format!("http://{}", addr);

    // 测试不同数据量
    for size_mb in [1, 4, 16] {
        let test_data = generate_test_data(size_mb * 1024); // KB -> bytes
        let data_size = test_data.len();

        group.throughput(Throughput::Bytes(data_size as u64));

        // 方式 1: 单个大消息
        group.bench_with_input(
            BenchmarkId::new("single_message", format!("{}MB", size_mb)),
            &test_data,
            |b, data| {
                b.to_async(&rt).iter(|| {
                    let endpoint = endpoint.clone();
                    let data = data.clone();
                    let repo_path_str = repo_path.to_str().unwrap().to_string();
                    async move {
                        let channel = Channel::from_shared(endpoint)
                            .unwrap()
                            .connect()
                            .await
                            .unwrap();
                        let mut client = SmartHttpServiceClient::new(channel);

                        let request = ReceivePackRequest {
                            repository: Some(Repository {
                                storage_path: repo_path_str,
                                relative_path: String::new(),
                            }),
                            data,
                            push_options: vec![],
                            user_id: 1,
                            username: "benchmark".to_string(),
                        };

                        let stream = tokio_stream::once(request);
                        // 忽略错误，因为数据不是有效的 git pack
                        let _ = client.receive_pack(stream).await;
                    }
                });
            },
        );

        // 方式 2: 64KB chunks 流式传输
        let chunk_size = 64 * 1024;
        group.bench_with_input(
            BenchmarkId::new("streaming_64k", format!("{}MB", size_mb)),
            &test_data,
            |b, data| {
                b.to_async(&rt).iter(|| {
                    let endpoint = endpoint.clone();
                    let data = data.clone();
                    let repo_path_str = repo_path.to_str().unwrap().to_string();
                    async move {
                        let channel = Channel::from_shared(endpoint)
                            .unwrap()
                            .connect()
                            .await
                            .unwrap();
                        let mut client = SmartHttpServiceClient::new(channel);

                        let chunks: Vec<ReceivePackRequest> = data
                            .chunks(chunk_size)
                            .enumerate()
                            .map(|(i, chunk)| ReceivePackRequest {
                                repository: if i == 0 {
                                    Some(Repository {
                                        storage_path: repo_path_str.clone(),
                                        relative_path: String::new(),
                                    })
                                } else {
                                    None
                                },
                                data: chunk.to_vec(),
                                push_options: vec![],
                                user_id: 1,
                                username: "benchmark".to_string(),
                            })
                            .collect();

                        let stream = tokio_stream::iter(chunks);
                        let _ = client.receive_pack(stream).await;
                    }
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: 测试内存分配对比
/// 模拟收集完整数据 vs 流式处理的内存差异
fn bench_memory_pattern(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("memory_pattern");
    group.sample_size(20);

    // 测试数据准备时间（模拟内存收集 vs 流式）
    let size_mb = 8;
    let chunk_size = 64 * 1024;
    
    group.throughput(Throughput::Bytes((size_mb * 1024 * 1024) as u64));

    // 模式 1: 先收集所有数据到 Vec，再处理（旧方式）
    group.bench_function("collect_then_process", |b| {
        b.to_async(&rt).iter(|| async {
            let mut collected = Vec::with_capacity(size_mb * 1024 * 1024);
            
            // 模拟逐块接收数据
            for _ in 0..(size_mb * 1024 * 1024 / chunk_size) {
                let chunk = generate_test_data(chunk_size / 1024);
                collected.extend_from_slice(&chunk);
            }
            
            // 然后一次性处理
            criterion::black_box(collected.len())
        });
    });

    // 模式 2: 流式处理，边读边发（新方式）
    group.bench_function("stream_process", |b| {
        b.to_async(&rt).iter(|| async {
            let mut total = 0usize;
            
            // 模拟逐块接收并立即处理
            for _ in 0..(size_mb * 1024 * 1024 / chunk_size) {
                let chunk = generate_test_data(chunk_size / 1024);
                total += chunk.len();
                // 立即"发送"（这里只是计数）
                criterion::black_box(&chunk);
            }
            
            total
        });
    });

    group.finish();
}

criterion_group!(benches, bench_grpc_streaming, bench_memory_pattern);
criterion_main!(benches);
