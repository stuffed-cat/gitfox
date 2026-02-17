# GitFox OAuth2 Provider 使用指南

GitFox 实现了标准的 OAuth2 授权服务器功能，允许第三方应用程序通过 OAuth2 协议访问 GitFox 用户的数据。本文档介绍如何将 GitFox 作为 OAuth2 Provider（授权服务器）使用。

## 目录

- [概述](#概述)
- [注册 OAuth 应用](#注册-oauth-应用)
- [OAuth2 授权流程](#oauth2-授权流程)
- [PKCE 支持](#pkce-支持)
- [刷新访问令牌](#刷新访问令牌)
- [令牌撤销 (RFC 7009)](#令牌撤销-rfc-7009)
- [OpenID Connect 支持](#openid-connect-支持)
- [获取用户信息](#获取用户信息)
- [作用域（Scopes）](#作用域scopes)
- [示例代码](#示例代码)

## 概述

### 支持的授权类型

- **Authorization Code Grant**（授权码模式）- 推荐用于服务端应用
- **Authorization Code Grant + PKCE**（带 PKCE 的授权码模式）- 推荐用于移动应用和单页应用（SPA）
- **Refresh Token Grant**（刷新令牌）- 用于获取新的访问令牌

### OAuth2 端点

| 端点 | URL | 用途 |
|------|-----|------|
| 授权端点 | `GET /oauth/authorize` | 获取授权码 |
| 令牌端点 | `POST /oauth/token` | 交换授权码获取访问令牌 |
| 令牌撤销端点 (RFC 7009) | `POST /oauth/revoke` | 撤销访问令牌或刷新令牌 |
| OIDC UserInfo 端点 | `GET /oauth/userinfo` | 获取当前授权用户信息 |
| 用户信息端点 | `GET /api/v1/auth/me` | 获取当前用户信息（使用 JWT） |

## 注册 OAuth 应用

在使用 GitFox 作为 OAuth Provider 之前，你需要先注册一个 OAuth 应用程序。

### 1. 通过 API 创建应用

**请求：**

```http
POST /api/v1/oauth/applications
Authorization: Bearer <your_jwt_token>
Content-Type: application/json

{
  "name": "My Awesome App",
  "description": "A cool application that integrates with GitFox",
  "homepage_url": "https://myapp.example.com",
  "redirect_uris": [
    "https://myapp.example.com/oauth/callback",
    "http://localhost:3000/oauth/callback"
  ],
  "scopes": ["read_user", "api", "read_repository"],
  "confidential": true
}
```

**参数说明：**

- `name` **(必需)** - 应用名称
- `redirect_uris` **(必需)** - 重定向 URI 数组，授权后会重定向到这些地址
- `scopes` - 应用可以请求的作用域列表，默认为 `["read_user"]`
- `description` - 应用描述
- `homepage_url` - 应用主页 URL
- `confidential` - 是否为机密客户端（服务端应用为 `true`，SPA/移动应用为 `false`），默认为 `true`

**响应：**

```json
{
  "id": 1,
  "name": "My Awesome App",
  "uid": "a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6",
  "secret": "gfcs_1a2b3c4d5e6f7g8h9i0j1k2l3m4n5o6p7q8r9s0t1u2v3w4x5y6z7a8b9c0d1e2f3",
  "redirect_uris": [
    "https://myapp.example.com/oauth/callback",
    "http://localhost:3000/oauth/callback"
  ],
  "scopes": ["read_user", "api", "read_repository"],
  "created_at": "2024-02-06T10:00:00Z"
}
```

**重要：** `client_secret`（也就是响应中的 `secret`）只会在创建时返回一次，请妥善保存。如果丢失，需要重新生成。

### 2. 管理已创建的应用

#### 列出你的应用

```http
GET /api/v1/oauth/applications
Authorization: Bearer <your_jwt_token>
```

#### 获取特定应用

```http
GET /api/v1/oauth/applications/{id}
Authorization: Bearer <your_jwt_token>
```

#### 更新应用

```http
PUT /api/v1/oauth/applications/{id}
Authorization: Bearer <your_jwt_token>
Content-Type: application/json

{
  "name": "My Updated App Name",
  "redirect_uris": ["https://newapp.example.com/callback"]
}
```

#### 重新生成客户端密钥

如果客户端密钥泄露或丢失，可以重新生成：

```http
POST /api/v1/oauth/applications/{id}/regenerate_secret
Authorization: Bearer <your_jwt_token>
```

#### 删除应用

```http
DELETE /api/v1/oauth/applications/{id}
Authorization: Bearer <your_jwt_token>
```

## OAuth2 授权流程

### Authorization Code Flow（授权码模式）

这是最常用也是最安全的 OAuth2 流程，适用于服务端应用。

#### 步骤 1：引导用户到授权页面

将用户重定向到 GitFox 的授权端点：

```
GET https://gitfox.example.com/oauth/authorize?
  client_id=a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6&
  redirect_uri=https://myapp.example.com/oauth/callback&
  response_type=code&
  scope=read_user%20api&
  state=random_string_for_csrf_protection
```

**参数说明：**

- `client_id` **(必需)** - 你的应用的客户端 ID（`uid`）
- `redirect_uri` **(必需)** - 回调地址，必须与注册时填写的一致
- `response_type` **(必需)** - 固定为 `code`
- `scope` - 请求的权限范围，空格分隔
- `state` - 随机字符串，用于防止 CSRF 攻击，建议使用

#### 步骤 2：用户授权

用户在 GitFox 上看到授权页面，显示你的应用请求的权限。如果是**受信任的应用**（`trusted = true`），则会跳过此步骤。

用户点击"授权"后，GitFox 会将用户重定向回你的 `redirect_uri`：

```
https://myapp.example.com/oauth/callback?
  code=authorization_code_here&
  state=random_string_for_csrf_protection
```

#### 步骤 3：交换授权码获取访问令牌

在你的服务端，使用授权码换取访问令牌：

```http
POST /oauth/token
Content-Type: application/x-www-form-urlencoded

grant_type=authorization_code&
code=authorization_code_here&
redirect_uri=https://myapp.example.com/oauth/callback&
client_id=a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6&
client_secret=gfcs_1a2b3c4d5e6f7g8h9i0j1k2l3m4n5o6p7q8r9s0t1u2v3w4x5y6z7a8b9c0d1e2f3
```

或者使用 HTTP Basic Authentication：

```http
POST /oauth/token
Authorization: Basic base64(client_id:client_secret)
Content-Type: application/x-www-form-urlencoded

grant_type=authorization_code&
code=authorization_code_here&
redirect_uri=https://myapp.example.com/oauth/callback
```

**响应：**

```json
{
  "access_token": "gfat_1a2b3c4d5e6f7g8h9i0j1k2l3m4n5o6p7q8r9s0t1u2v3w4x5y6z7a8b9c0d1e2f3",
  "token_type": "Bearer",
  "expires_in": 7200,
  "refresh_token": "gfat_9z8y7x6w5v4u3t2s1r0q9p8o7n6m5l4k3j2i1h0g9f8e7d6c5b4a3z2y1x0w9v8u7",
  "scope": "read_user api",
  "id_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "created_at": 1707217200
}
```

**字段说明：**

- `access_token` - 访问令牌，用于调用 API
- `token_type` - 令牌类型，固定为 `Bearer`
- `expires_in` - 访问令牌过期时间（秒），默认为 7200（2小时）
- `refresh_token` - 刷新令牌，用于获取新的访问令牌
- `scope` - 实际授予的权限范围
- `id_token` - OpenID Connect ID Token（JWT 格式），仅当请求包含 `openid` scope 时返回
- `created_at` - 令牌创建时间（Unix 时间戳）

#### 步骤 4：使用访问令牌调用 API

在后续的 API 请求中，在 `Authorization` header 中携带访问令牌：

```http
GET /api/v1/auth/me
Authorization: Bearer gfat_1a2b3c4d5e6f7g8h9i0j1k2l3m4n5o6p7q8r9s0t1u2v3w4x5y6z7a8b9c0d1e2f3
```

## PKCE 支持

PKCE (Proof Key for Code Exchange) 是 OAuth2 的扩展，主要用于保护**公共客户端**（如移动应用、单页应用）免受授权码拦截攻击。

### 使用 PKCE 的授权流程

#### 步骤 1：生成 Code Verifier 和 Code Challenge

```javascript
// 生成随机的 code_verifier (43-128 字符)
const codeVerifier = generateRandomString(64);

// 使用 SHA256 生成 code_challenge
const codeChallenge = base64UrlEncode(sha256(codeVerifier));
```

#### 步骤 2：授权请求

```
GET https://gitfox.example.com/oauth/authorize?
  client_id=a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6&
  redirect_uri=https://myapp.example.com/oauth/callback&
  response_type=code&
  scope=read_user&
  state=random_state&
  code_challenge=BASE64URL(SHA256(code_verifier))&
  code_challenge_method=S256
```

**新增参数：**

- `code_challenge` - Code Verifier 的 SHA256 哈希值（Base64 URL 编码）
- `code_challenge_method` - 挑战方法，支持 `plain` 或 `S256`（推荐）

#### 步骤 3：交换令牌时提供 Code Verifier

```http
POST /oauth/token
Content-Type: application/x-www-form-urlencoded

grant_type=authorization_code&
code=authorization_code_here&
redirect_uri=https://myapp.example.com/oauth/callback&
client_id=a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6&
code_verifier=original_code_verifier_string
```

**注意：** 使用 PKCE 时，公共客户端（`confidential = false`）可以不提供 `client_secret`。

## 刷新访问令牌

访问令牌默认 2 小时后过期。使用刷新令牌可以获取新的访问令牌，而无需用户重新授权。

```http
POST /oauth/token
Content-Type: application/x-www-form-urlencoded

grant_type=refresh_token&
refresh_token=gfat_9z8y7x6w5v4u3t2s1r0q9p8o7n6m5l4k3j2i1h0g9f8e7d6c5b4a3z2y1x0w9v8u7&
client_id=a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6&
client_secret=gfcs_1a2b3c4d5e6f7g8h9i0j1k2l3m4n5o6p7q8r9s0t1u2v3w4x5y6z7a8b9c0d1e2f3
```

**响应：**

```json
{
  "access_token": "gfat_new_access_token",
  "token_type": "Bearer",
  "expires_in": 7200,
  "refresh_token": "gfat_new_refresh_token",
  "scope": "read_user api"
}
```

**注意：**

- 每次刷新都会生成新的访问令牌和刷新令牌
- 旧的访问令牌和刷新令牌会被撤销
- 刷新令牌默认 30 天后过期

## 令牌撤销 (RFC 7009)

GitFox 实现了 [RFC 7009 OAuth 2.0 Token Revocation](https://tools.ietf.org/html/rfc7009) 标准，允许客户端撤销访问令牌或刷新令牌。

### 撤销令牌

```http
POST /oauth/revoke
Content-Type: application/x-www-form-urlencoded

token=gfat_1a2b3c4d5e6f7g8h9i0j1k2l3m4n5o6p7q8r9s0t1u2v3w4x5y6z7a8b9c0d1e2f3&
token_type_hint=access_token&
client_id=a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6&
client_secret=gfcs_1a2b3c4d5e6f7g8h9i0j1k2l3m4n5o6p7q8r9s0t1u2v3w4x5y6z7a8b9c0d1e2f3
```

或使用 HTTP Basic Authentication：

```http
POST /oauth/revoke
Authorization: Basic base64(client_id:client_secret)
Content-Type: application/x-www-form-urlencoded

token=gfat_1a2b3c4d5e6f7g8h9i0j1k2l3m4n5o6p7q8r9s0t1u2v3w4x5y6z7a8b9c0d1e2f3&
token_type_hint=access_token
```

**参数说明：**

- `token` **(必需)** - 要撤销的令牌（访问令牌或刷新令牌）
- `token_type_hint` - 令牌类型提示，可选值：`access_token` 或 `refresh_token`
- `client_id` - 客户端 ID（使用 Basic Auth 时可省略）
- `client_secret` - 客户端密钥（机密客户端必需，使用 Basic Auth 时可省略）

**响应：**

符合 RFC 7009 规范，无论令牌是否存在或已被撤销，服务器都返回 `200 OK` 空响应。

**示例代码（JavaScript）：**

```javascript
async function revokeToken(token, client) {
  await fetch('https://gitfox.example.com/oauth/revoke', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/x-www-form-urlencoded',
      'Authorization': `Basic ${btoa(`${client.id}:${client.secret}`)}`,
    },
    body: new URLSearchParams({
      token: token,
      token_type_hint: 'access_token',
    }),
  });
  
  // 令牌已被撤销
  console.log('Token revoked successfully');
}
```

## OpenID Connect 支持

GitFox 实现了 OpenID Connect (OIDC) 核心功能，提供标准化的身份认证层。

### 请求 OIDC 认证

在授权请求中包含 `openid` scope：

```
GET https://gitfox.example.com/oauth/authorize?
  client_id=a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6&
  redirect_uri=https://myapp.example.com/oauth/callback&
  response_type=code&
  scope=openid%20email%20profile&
  state=random_string
```

### ID Token

当请求包含 `openid` scope 时，token 端点会在响应中返回 `id_token`（JWT 格式）：

```json
{
  "access_token": "gfat_...",
  "token_type": "Bearer",
  "expires_in": 7200,
  "refresh_token": "gfat_...",
  "scope": "openid email profile",
  "id_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJodHRwczovL2dpdGZveC5leGFtcGxlLmNvbSIsInN1YiI6IjEyMyIsImF1ZCI6ImNsaWVudF9pZCIsImV4cCI6MTcwNzIyMDgwMCwiaWF0IjoxNzA3MjE3MjAwLCJhenAiOiJjbGllbnRfaWQiLCJwcmVmZXJyZWRfdXNlcm5hbWUiOiJqb2huZG9lIiwiZW1haWwiOiJqb2huQGV4YW1wbGUuY29tIiwiZW1haWxfdmVyaWZpZWQiOnRydWV9...",
  "created_at": 1707217200
}
```

**ID Token Claims（声明）：**

- `iss` - 发行者（GitFox 实例 URL）
- `sub` - 主体标识符（用户 ID）
- `aud` - 受众（客户端 ID）
- `exp` - 过期时间（Unix 时间戳）
- `iat` - 签发时间（Unix 时间戳）
- `azp` - 授权方（客户端 ID）
- `preferred_username` - 用户名
- `email` - 电子邮件（需要 `email` scope）
- `email_verified` - 邮箱是否已验证

**验证 ID Token（JavaScript）：**

```javascript
const jwt = require('jsonwebtoken');

function verifyIdToken(idToken, clientId, jwtSecret) {
  try {
    const decoded = jwt.verify(idToken, jwtSecret, {
      audience: clientId,
      algorithms: ['HS256'],
    });
    
    console.log('User ID:', decoded.sub);
    console.log('Username:', decoded.preferred_username);
    console.log('Email:', decoded.email);
    
    return decoded;
  } catch (err) {
    console.error('Invalid ID token:', err);
    return null;
  }
}
```

### UserInfo 端点

OIDC 标准 UserInfo 端点提供用户信息：

```http
GET /oauth/userinfo
Authorization: Bearer gfat_1a2b3c4d5e6f7g8h9i0j1k2l3m4n5o6p7q8r9s0t1u2v3w4x5y6z7a8b9c0d1e2f3
```

**响应：**

```json
{
  "sub": "123",
  "preferred_username": "johndoe",
  "name": "John Doe",
  "email": "john@example.com",
  "email_verified": true,
  "picture": null,
  "profile": "https://gitfox.example.com/johndoe",
  "created_at": 1707217200,
  "updated_at": 1707217200
}
```

**字段说明：**

- `sub` - 用户唯一标识符
- `preferred_username` - 用户名
- `name` - 显示名称
- `email` - 电子邮件（需要 `email` scope）
- `email_verified` - 邮箱验证状态
- `picture` - 头像 URL
- `profile` - 用户主页 URL
- `created_at` - 账户创建时间
- `updated_at` - 最后更新时间

**注意：** 访问 UserInfo 端点需要在授权时请求 `openid` scope，否则会返回 403 Forbidden。

### OIDC Scopes

- `openid` **(必需)** - 启用 OIDC，返回 `id_token` 和访问 UserInfo 端点
- `email` - 在 ID Token 和 UserInfo 中包含邮箱信息
- `profile` - 在 UserInfo 中包含完整的用户资料

## 获取用户信息

使用访问令牌获取当前已授权用户的信息：

```http
GET /api/v1/auth/me
Authorization: Bearer gfat_your_access_token
```

**响应示例：**

```json
{
  "id": 1,
  "username": "johndoe",
  "email": "john@example.com",
  "display_name": "John Doe",
  "avatar_url": "https://gitfox.example.com/avatars/john.png",
  "is_admin": false,
  "created_at": "2024-01-01T00:00:00Z",
  "two_factor_enabled": true
}
```

## 作用域（Scopes）

GitFox 支持以下作用域：

| Scope | 描述 |
|-------|------|
| `read_user` | 读取用户基本信息 |
| `write_user` | 修改用户信息 |
| `api` | 完整的 API 访问权限（读写） |
| `read_repository` | 读取仓库内容 |
| `write_repository` | 修改仓库内容（推送、创建分支等） |
| `read_api` | 只读 API 访问 |
| `write_api` | 读写 API 访问 |
| `sudo` | 管理员权限（危险，谨慎授予） |

**默认作用域：** 如果不指定 `scope` 参数，默认为 `read_user`。

**多个作用域：** 使用空格分隔多个作用域，例如：`scope=read_user api read_repository`

## 示例代码

### Node.js (Express) 示例

```javascript
const express = require('express');
const axios = require('axios');
const crypto = require('crypto');

const app = express();

const GITFOX_BASE_URL = 'https://gitfox.example.com';
const CLIENT_ID = 'your_client_id';
const CLIENT_SECRET = 'your_client_secret';
const REDIRECT_URI = 'http://localhost:3000/oauth/callback';

// 步骤 1：重定向到 GitFox 授权页面
app.get('/oauth/login', (req, res) => {
  const state = crypto.randomBytes(16).toString('hex');
  req.session.oauthState = state;

  const authUrl = new URL(`${GITFOX_BASE_URL}/oauth/authorize`);
  authUrl.searchParams.set('client_id', CLIENT_ID);
  authUrl.searchParams.set('redirect_uri', REDIRECT_URI);
  authUrl.searchParams.set('response_type', 'code');
  authUrl.searchParams.set('scope', 'read_user api');
  authUrl.searchParams.set('state', state);

  res.redirect(authUrl.toString());
});

// 步骤 2 & 3：处理回调，交换授权码
app.get('/oauth/callback', async (req, res) => {
  const { code, state } = req.query;

  // 验证 state 参数防止 CSRF
  if (state !== req.session.oauthState) {
    return res.status(400).send('Invalid state parameter');
  }

  try {
    // 交换授权码获取访问令牌
    const tokenResponse = await axios.post(
      `${GITFOX_BASE_URL}/oauth/token`,
      new URLSearchParams({
        grant_type: 'authorization_code',
        code,
        redirect_uri: REDIRECT_URI,
        client_id: CLIENT_ID,
        client_secret: CLIENT_SECRET,
      }),
      {
        headers: {
          'Content-Type': 'application/x-www-form-urlencoded',
        },
      }
    );

    const { access_token, refresh_token } = tokenResponse.data;

    // 保存令牌（实际应用中应加密存储）
    req.session.accessToken = access_token;
    req.session.refreshToken = refresh_token;

    // 获取用户信息
    const userResponse = await axios.get(
      `${GITFOX_BASE_URL}/api/v1/auth/me`,
      {
        headers: {
          Authorization: `Bearer ${access_token}`,
        },
      }
    );

    res.json({
      message: 'Login successful',
      user: userResponse.data,
    });
  } catch (error) {
    console.error('OAuth error:', error.response?.data || error.message);
    res.status(500).send('OAuth authentication failed');
  }
});

app.listen(3000, () => {
  console.log('Server running on http://localhost:3000');
});
```

### Python (Flask) 示例

```python
from flask import Flask, redirect, request, session
import requests
import secrets

app = Flask(__name__)
app.secret_key = 'your-secret-key'

GITFOX_BASE_URL = 'https://gitfox.example.com'
CLIENT_ID = 'your_client_id'
CLIENT_SECRET = 'your_client_secret'
REDIRECT_URI = 'http://localhost:5000/oauth/callback'

# 步骤 1：重定向到 GitFox 授权页面
@app.route('/oauth/login')
def oauth_login():
    state = secrets.token_hex(16)
    session['oauth_state'] = state
    
    auth_url = f"{GITFOX_BASE_URL}/oauth/authorize"
    params = {
        'client_id': CLIENT_ID,
        'redirect_uri': REDIRECT_URI,
        'response_type': 'code',
        'scope': 'read_user api',
        'state': state
    }
    
    query_string = '&'.join([f"{k}={v}" for k, v in params.items()])
    return redirect(f"{auth_url}?{query_string}")

# 步骤 2 & 3：处理回调，交换授权码
@app.route('/oauth/callback')
def oauth_callback():
    code = request.args.get('code')
    state = request.args.get('state')
    
    # 验证 state 参数
    if state != session.get('oauth_state'):
        return 'Invalid state parameter', 400
    
    # 交换授权码获取访问令牌
    token_response = requests.post(
        f"{GITFOX_BASE_URL}/oauth/token",
        data={
            'grant_type': 'authorization_code',
            'code': code,
            'redirect_uri': REDIRECT_URI,
            'client_id': CLIENT_ID,
            'client_secret': CLIENT_SECRET
        },
        headers={'Content-Type': 'application/x-www-form-urlencoded'}
    )
    
    if token_response.status_code != 200:
        return 'Token exchange failed', 500
    
    tokens = token_response.json()
    access_token = tokens['access_token']
    
    # 保存令牌
    session['access_token'] = access_token
    session['refresh_token'] = tokens.get('refresh_token')
    
    # 获取用户信息
    user_response = requests.get(
        f"{GITFOX_BASE_URL}/api/v1/auth/me",
        headers={'Authorization': f"Bearer {access_token}"}
    )
    
    return {
        'message': 'Login successful',
        'user': user_response.json()
    }

if __name__ == '__main__':
    app.run(debug=True)
```

### JavaScript (SPA with PKCE) 示例

```javascript
// 生成随机字符串
function generateRandomString(length) {
  const chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~';
  let result = '';
  const randomValues = new Uint8Array(length);
  crypto.getRandomValues(randomValues);
  randomValues.forEach(v => result += chars[v % chars.length]);
  return result;
}

// SHA256 哈希
async function sha256(plain) {
  const encoder = new TextEncoder();
  const data = encoder.encode(plain);
  return await crypto.subtle.digest('SHA-256', data);
}

// Base64 URL 编码
function base64UrlEncode(arrayBuffer) {
  const bytes = new Uint8Array(arrayBuffer);
  let binary = '';
  bytes.forEach(b => binary += String.fromCharCode(b));
  return btoa(binary)
    .replace(/\+/g, '-')
    .replace(/\//g, '_')
    .replace(/=+$/, '');
}

// 开始 OAuth 流程
async function startOAuthLogin() {
  const GITFOX_BASE_URL = 'https://gitfox.example.com';
  const CLIENT_ID = 'your_client_id';
  const REDIRECT_URI = 'http://localhost:3000/callback';

  // 生成 PKCE 参数
  const codeVerifier = generateRandomString(64);
  const codeChallenge = base64UrlEncode(await sha256(codeVerifier));
  const state = generateRandomString(32);

  // 保存到 sessionStorage
  sessionStorage.setItem('pkce_code_verifier', codeVerifier);
  sessionStorage.setItem('oauth_state', state);

  // 构建授权 URL
  const authUrl = new URL(`${GITFOX_BASE_URL}/oauth/authorize`);
  authUrl.searchParams.set('client_id', CLIENT_ID);
  authUrl.searchParams.set('redirect_uri', REDIRECT_URI);
  authUrl.searchParams.set('response_type', 'code');
  authUrl.searchParams.set('scope', 'read_user api');
  authUrl.searchParams.set('state', state);
  authUrl.searchParams.set('code_challenge', codeChallenge);
  authUrl.searchParams.set('code_challenge_method', 'S256');

  // 重定向
  window.location.href = authUrl.toString();
}

// 处理回调
async function handleOAuthCallback() {
  const params = new URLSearchParams(window.location.search);
  const code = params.get('code');
  const state = params.get('state');

  // 验证 state
  if (state !== sessionStorage.getItem('oauth_state')) {
    console.error('Invalid state parameter');
    return;
  }

  const codeVerifier = sessionStorage.getItem('pkce_code_verifier');

  const GITFOX_BASE_URL = 'https://gitfox.example.com';
  const CLIENT_ID = 'your_client_id';
  const REDIRECT_URI = 'http://localhost:3000/callback';

  // 交换授权码
  const tokenResponse = await fetch(`${GITFOX_BASE_URL}/oauth/token`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/x-www-form-urlencoded',
    },
    body: new URLSearchParams({
      grant_type: 'authorization_code',
      code: code,
      redirect_uri: REDIRECT_URI,
      client_id: CLIENT_ID,
      code_verifier: codeVerifier,
    }),
  });

  const tokens = await tokenResponse.json();
  
  // 保存令牌
  localStorage.setItem('access_token', tokens.access_token);
  localStorage.setItem('refresh_token', tokens.refresh_token);

  // 获取用户信息
  const userResponse = await fetch(`${GITFOX_BASE_URL}/api/v1/auth/me`, {
    headers: {
      Authorization: `Bearer ${tokens.access_token}`,
    },
  });

  const user = await userResponse.json();
  console.log('Logged in user:', user);
}
```

## 安全最佳实践

1. **始终使用 HTTPS**：在生产环境中，所有 OAuth 流程必须通过 HTTPS 进行
2. **验证 state 参数**：防止 CSRF 攻击
3. **妥善保管客户端密钥**：不要将 `client_secret` 提交到版本控制系统
4. **使用 PKCE**：对于公共客户端（SPA、移动应用），必须使用 PKCE
5. **最小权限原则**：只请求应用所需的最小作用域
6. **安全存储令牌**：在服务端加密存储令牌，在客户端使用 HttpOnly Cookie 或安全的本地存储
7. **设置短期访问令牌**：使用刷新令牌机制，保持访问令牌短期有效
8. **验证 redirect_uri**：严格验证回调 URI，防止开放重定向攻击

## 常见问题

### 1. 如何撤销访问令牌？

GitFox 实现了 RFC 7009 Token Revocation 标准。参见「令牌撤销」章节了解详情。

### 2. 受信任应用是什么？

受信任应用（`trusted = true`）会跳过用户授权确认步骤，直接发放访问令牌。这通常用于第一方应用。只有管理员可以将应用设置为受信任。

### 3. 如何限制特定用户或组织的访问？

应用级别的访问控制需要在应用层实现。GitFox 目前不提供内置的用户/组织级别的应用访问限制。

### 4. 支持 OpenID Connect (OIDC) 吗？

是的，GitFox 支持 OIDC 核心功能。当请求包含 `openid` scope 时，token 响应会包含 `id_token`（JWT 格式），并提供标准的 `/oauth/userinfo` 端点。参见「OpenID Connect 支持」章节。

## 相关文档

- [GitFox API 文档](./api-reference.md)
- [Personal Access Tokens 使用指南](./personal-access-tokens.md)
- [GitFox 作为 OAuth Client 配置](./oauth-client-configuration.md)

---

**版本：** v1.0  
**最后更新：** 2024-02-06
