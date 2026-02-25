use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt;

/// 类型安全的 Scope 定义
/// 使用层级命名空间格式: resource:action
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    // API 访问
    #[serde(rename = "api:read")]
    ApiRead,
    #[serde(rename = "api:write")]
    ApiWrite,
    
    // 仓库访问
    #[serde(rename = "repository:read")]
    RepositoryRead,
    #[serde(rename = "repository:write")]
    RepositoryWrite,
    
    // 用户信息
    #[serde(rename = "user:read")]
    UserRead,
    #[serde(rename = "user:write")]
    UserWrite,
    
    // 注册表访问
    #[serde(rename = "registry:read")]
    RegistryRead,
    #[serde(rename = "registry:write")]
    RegistryWrite,
    
    // CI/CD
    #[serde(rename = "cicd:read")]
    CicdRead,
    #[serde(rename = "cicd:write")]
    CicdWrite,
    
    // 管理员权限（所有权限）
    #[serde(rename = "admin")]
    Admin,
}

impl Scope {
    /// 转换为字符串表示
    pub fn as_str(&self) -> &'static str {
        match self {
            Scope::ApiRead => "api:read",
            Scope::ApiWrite => "api:write",
            Scope::RepositoryRead => "repository:read",
            Scope::RepositoryWrite => "repository:write",
            Scope::UserRead => "user:read",
            Scope::UserWrite => "user:write",
            Scope::RegistryRead => "registry:read",
            Scope::RegistryWrite => "registry:write",
            Scope::CicdRead => "cicd:read",
            Scope::CicdWrite => "cicd:write",
            Scope::Admin => "admin",
        }
    }

    /// 从字符串解析（兼容旧格式）
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            // 新格式
            "api:read" => Some(Scope::ApiRead),
            "api:write" => Some(Scope::ApiWrite),
            "repository:read" => Some(Scope::RepositoryRead),
            "repository:write" => Some(Scope::RepositoryWrite),
            "user:read" => Some(Scope::UserRead),
            "user:write" => Some(Scope::UserWrite),
            "registry:read" => Some(Scope::RegistryRead),
            "registry:write" => Some(Scope::RegistryWrite),
            "cicd:read" => Some(Scope::CicdRead),
            "cicd:write" => Some(Scope::CicdWrite),
            "admin" => Some(Scope::Admin),
            
            // 兼容旧格式（read_api -> api:read）
            "read_api" => Some(Scope::ApiRead),
            "write_api" => Some(Scope::ApiWrite),
            "read_repository" => Some(Scope::RepositoryRead),
            "write_repository" => Some(Scope::RepositoryWrite),
            "read_user" => Some(Scope::UserRead),
            "write_user" => Some(Scope::UserWrite),
            "read_registry" => Some(Scope::RegistryRead),
            "write_registry" => Some(Scope::RegistryWrite),
            
            _ => None,
        }
    }

    /// 获取所有可用的 scopes
    pub fn all() -> Vec<Self> {
        vec![
            Scope::ApiRead,
            Scope::ApiWrite,
            Scope::RepositoryRead,
            Scope::RepositoryWrite,
            Scope::UserRead,
            Scope::UserWrite,
            Scope::RegistryRead,
            Scope::RegistryWrite,
            Scope::CicdRead,
            Scope::CicdWrite,
            Scope::Admin,
        ]
    }

    /// 检查此 scope 是否隐含包含另一个 scope
    pub fn implies(&self, other: &Scope) -> bool {
        if self == other {
            return true;
        }

        match self {
            // Admin 包含所有权限
            Scope::Admin => true,
            
            // Write 包含对应的 Read
            Scope::ApiWrite => matches!(other, Scope::ApiRead),
            Scope::RepositoryWrite => matches!(other, Scope::RepositoryRead),
            Scope::UserWrite => matches!(other, Scope::UserRead),
            Scope::RegistryWrite => matches!(other, Scope::RegistryRead),
            Scope::CicdWrite => matches!(other, Scope::CicdRead),
            
            _ => false,
        }
    }
}

impl fmt::Display for Scope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Token 权限级别（替代 Option<Vec<String>>，更清晰的语义）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TokenScope {
    /// 完全访问（JWT token）
    Full,
    
    /// 受限访问（PAT/OAuth，使用 HashSet 提升性能）
    Limited {
        #[serde(skip)]
        scopes: HashSet<Scope>,
        
        // 用于序列化的字符串列表
        #[serde(rename = "scopes")]
        scope_strings: Vec<String>,
    },
}

impl TokenScope {
    /// 创建完全访问权限
    pub fn full() -> Self {
        TokenScope::Full
    }

    /// 创建受限访问权限
    pub fn limited(scopes: Vec<Scope>) -> Self {
        let scope_strings: Vec<String> = scopes.iter().map(|s| s.as_str().to_string()).collect();
        let scopes: HashSet<Scope> = scopes.into_iter().collect();
        
        TokenScope::Limited {
            scopes,
            scope_strings,
        }
    }

    /// 从字符串列表创建（用于数据库反序列化）
    pub fn from_strings(strings: Vec<String>) -> Self {
        let scopes: HashSet<Scope> = strings
            .iter()
            .filter_map(|s| Scope::from_str(s))
            .collect();
        
        TokenScope::Limited {
            scopes,
            scope_strings: strings,
        }
    }

    /// 检查是否有特定 scope（O(1) 复杂度）
    pub fn has(&self, scope: &Scope) -> bool {
        match self {
            TokenScope::Full => true,
            TokenScope::Limited { scopes, .. } => {
                // 直接检查
                if scopes.contains(scope) {
                    return true;
                }
                
                // 检查是否有隐含此 scope 的更高权限
                scopes.iter().any(|s| s.implies(scope))
            }
        }
    }

    /// 检查是否有任一 scope
    pub fn has_any(&self, required: &[Scope]) -> bool {
        required.iter().any(|scope| self.has(scope))
    }

    /// 检查是否有所有 scopes
    pub fn has_all(&self, required: &[Scope]) -> bool {
        required.iter().all(|scope| self.has(scope))
    }

    /// 获取 scope 列表（用于显示/日志）
    pub fn list(&self) -> Vec<String> {
        match self {
            TokenScope::Full => vec!["*".to_string()],
            TokenScope::Limited { scope_strings, .. } => scope_strings.clone(),
        }
    }

    /// 是否为完全访问
    pub fn is_full(&self) -> bool {
        matches!(self, TokenScope::Full)
    }
}

impl Default for TokenScope {
    fn default() -> Self {
        TokenScope::Full
    }
}

/// Token 审计信息（用于日志脱敏）
#[derive(Debug, Clone, Serialize)]
pub struct TokenAuditInfo {
    pub token_type: TokenType,
    pub token_prefix: String, // 只保留前缀，如 "gfpat_a1b2..."
    pub user_id: i64,
    pub username: String,
    pub scopes: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TokenType {
    Jwt,
    PersonalAccessToken,
    OAuth2,
}

impl TokenAuditInfo {
    /// 脱敏 token（只保留前8字符 + "..."）
    pub fn mask_token(token: &str) -> String {
        if token.len() <= 12 {
            return "***".to_string();
        }
        format!("{}...{}", &token[..8], &token[token.len()-4..])
    }

    /// 创建审计信息
    pub fn new(
        token_type: TokenType,
        token: &str,
        user_id: i64,
        username: String,
        scopes: TokenScope,
    ) -> Self {
        Self {
            token_type,
            token_prefix: Self::mask_token(token),
            user_id,
            username,
            scopes: scopes.list(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scope_implies() {
        assert!(Scope::Admin.implies(&Scope::ApiRead));
        assert!(Scope::ApiWrite.implies(&Scope::ApiRead));
        assert!(!Scope::ApiRead.implies(&Scope::ApiWrite));
    }

    #[test]
    fn test_token_scope_performance() {
        let scopes = TokenScope::limited(vec![
            Scope::ApiWrite,
            Scope::RepositoryRead,
        ]);

        // O(1) 查找
        assert!(scopes.has(&Scope::ApiRead)); // ApiWrite 包含 ApiRead
        assert!(scopes.has(&Scope::RepositoryRead));
        assert!(!scopes.has(&Scope::UserWrite));
    }

    #[test]
    fn test_backward_compatibility() {
        // 旧格式应该能正确解析
        assert_eq!(Scope::from_str("read_api"), Some(Scope::ApiRead));
        assert_eq!(Scope::from_str("write_repository"), Some(Scope::RepositoryWrite));
        
        // 新格式
        assert_eq!(Scope::from_str("api:read"), Some(Scope::ApiRead));
    }

    #[test]
    fn test_token_masking() {
        let token = "gfpat_1234567890abcdef1234567890abcdef";
        let masked = TokenAuditInfo::mask_token(token);
        assert_eq!(masked, "gfpat_12...cdef");
        assert!(!masked.contains("567890abcdef1234567890ab"));
    }
}
