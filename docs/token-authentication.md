# Token 认证使用说明

## 支持的 Token 类型

GitFox API 支持三种 token 认证：

1. **JWT Token** - 登录后的会话 token，完全权限
2. **Personal Access Token (PAT)** - `gfpat_*` 开头，可配置 scopes
3. **OAuth2 Access Token** - 第三方应用授权，可配置 scopes

## 使用方法

所有 API 请求使用 `Authorization: Bearer <token>` header：

```bash
curl -H "Authorization: Bearer gfpat_abc123..." https://gitfox.example.com/api/v1/user/profile
```

## Scopes 权限

PAT 和 OAuth token 支持以下 scopes：

- `read_api` / `write_api` - 读取/写入 API
- `read_repository` / `write_repository` - 克隆/推送仓库
- `read_user` / `write_user` - 读取/更新用户信息
- `read_registry` / `write_registry` - 拉取/推送容器镜像
- `admin` - 管理员权限（包含所有 scopes）

**继承规则**:
- `write_*` 自动包含对应的 `read_*`
- `admin` 包含所有权限

## Handler 中检查权限

```rust
use crate::middleware::AuthenticatedUser;
use crate::models::Scope;

pub async fn my_handler(auth: AuthenticatedUser) -> AppResult<HttpResponse> {
    // 检查单个 scope
    if !auth.has_scope(&Scope::WriteApi) {
        return Err(AppError::Forbidden("需要 write_api 权限".into()));
    }
    
    // 检查多个 scope（任一即可）
    if !auth.has_any_scope(&[Scope::WriteApi, Scope::Admin]) {
        return Err(AppError::Forbidden("权限不足".into()));
    }
    
    // 业务逻辑...
}
```

## 创建 Personal Access Token

```bash
POST /api/v1/user/access_tokens
Authorization: Bearer <jwt-token>
Content-Type: application/json

{
  "name": "my-token",
  "scopes": ["read_api", "read_repository"],
  "expires_in_days": 30
}
```

响应包含完整 token（仅显示一次）：
```json
{
  "id": 123,
  "token": "gfpat_a1b2c3d4e5f678901234567890abcdef",
  "name": "my-token",
  "scopes": ["read_api", "read_repository"],
  "expires_at": "2026-03-28T12:00:00Z"
}
```

## 注意事项

- JWT token 拥有完全权限（`TokenScope::Full`）
- PAT/OAuth token 权限 = 用户角色权限 ∩ token scopes
- Token 在数据库中以 SHA-256 hash 存储
- 即使是 Admin 用户，PAT 也受 scopes 限制
