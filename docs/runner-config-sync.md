# Runner 配置双向同步机制

## 问题
之前通过 API 修改 runner 的 tags 等配置后，这些修改只保存在数据库中。当 runner 重启时，会用 config.toml 中的旧配置重新注册，导致数据库中的修改丢失。

## 解决方案
实现配置双向同步机制：当管理员通过 API 修改 runner 配置时，后端会通过 WebSocket 实时通知在线的 runner 更新其 config.toml 文件。

## 实现细节

### 1. 后端 (devops)

#### 新增消息类型
在 `ServerMessage` 枚举中添加 `ConfigUpdate` 消息：

```rust
#[serde(rename = "config_update")]
ConfigUpdate {
    tags: Option<Vec<String>>,
    description: Option<String>,
    run_untagged: Option<bool>,
    maximum_timeout: Option<i32>,
}
```

#### 更新 API Handlers
所有更新 runner 的 API 端点（admin/user/namespace/project 级别）现在会：

1. 更新数据库中的配置
2. 如果 runner 在线，通过 WebSocket 发送 `ConfigUpdate` 消息

相关函数：
- `admin_update_runner` - 系统级 runner 更新
- `user_update_runner` - 用户级 runner 更新
- `namespace_update_runner` - 命名空间级 runner 更新
- `project_update_runner` - 项目级 runner 更新

新增辅助函数：
- `send_config_update` - 发送配置更新消息给在线 runner

### 2. Runner 客户端 (gitfox-runner)

#### 消息处理
在 `handle_server_message` 方法中添加对 `ConfigUpdate` 消息的处理：

```rust
ServerMessage::ConfigUpdate { tags, description, run_untagged, maximum_timeout } => {
    info!("Received configuration update from server");
    if let Err(e) = self.update_config(tags, description, run_untagged, maximum_timeout) {
        error!("Failed to update configuration: {}", e);
    } else {
        info!("Configuration updated and saved successfully");
    }
}
```

#### 配置更新与持久化
新增 `update_config` 方法：

1. 更新内存中的配置（`self.config`）
2. 调用 `config.save()` 保存到 config.toml 文件
3. 记录配置变更日志

相关修改：
- `Runner` 结构体新增 `config_path: PathBuf` 字段
- `Runner::new()` 方法签名更新为接受 config 和 config_path
- `main.rs` 中传递 config_path 参数

## 使用示例

### 通过 API 更新 runner tags

```bash
# 更新系统级 runner
curl -X PUT http://localhost:8080/api/v1/admin/runners/1 \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "tags": ["docker", "linux", "amd64"],
    "description": "Updated production runner"
  }'
```

### Runner 日志输出

当收到配置更新时，runner 会输出：

```
[INFO] Received configuration update from server
[INFO]   Tags: shell -> docker, linux, amd64
[INFO] Configuration saved to: /home/user/.gitfox-runner/config.toml
[INFO] Configuration updated and saved successfully
```

## 配置持久化流程

```
管理员通过 API 更新配置
    ↓
后端更新数据库
    ↓
后端检查 runner 是否在线
    ↓ (如果在线)
通过 WebSocket 发送 ConfigUpdate 消息
    ↓
Runner 接收消息
    ↓
Runner 更新内存配置
    ↓
Runner 保存到 config.toml
    ↓
配置持久化完成
```

## 注意事项

1. **在线 runner**: 只有在线的 runner 才会收到实时更新。离线 runner 下次注册时仍会使用 config.toml 中的配置。

2. **配置字段映射**:
   - `tags`: 直接支持并持久化
   - `description`: 保留接口，runner config 暂无此字段
   - `run_untagged`: 服务器端配置，runner 无对应字段
   - `maximum_timeout`: 服务器端配置，与 runner 的 `script_timeout` 不同

3. **重启后一致性**: Runner 重启后会从更新的 config.toml 加载配置，保证与数据库一致。

4. **权限控制**: 只有对应级别的管理员才能更新 runner 配置：
   - 系统级: admin 用户
   - 用户级: runner 所有者
   - 命名空间级: namespace owner
   - 项目级: project owner/maintainer

## 文件修改清单

### 后端
- `src/handlers/runner.rs`: 添加 ConfigUpdate 消息类型和发送逻辑

### Runner 客户端  
- `gitfox-runner/src/messages.rs`: 添加 ConfigUpdate 消息定义
- `gitfox-runner/src/runner.rs`: 添加配置更新处理和持久化逻辑
- `gitfox-runner/src/main.rs`: 传递 config_path 参数

## 测试建议

1. 启动 devops 后端和 runner
2. 通过 API 修改 runner 的 tags
3. 检查 runner 日志确认收到更新
4. 查看 config.toml 确认配置已保存
5. 重启 runner 确认使用新配置注册
