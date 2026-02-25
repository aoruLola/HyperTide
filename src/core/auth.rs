//! Authentication Manager
//! Handles API key validation and permission checking

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// 权限级别
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Permission {
    /// 锁定/解锁文件
    Lock,
    /// 上传文件
    Upload,
    /// 下载文件
    Download,
    /// 管理员权限 (强制解锁、生成/撤销 Key)
    Admin,
}

/// API Key 信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    /// Key 值 (用于认证)
    pub key: String,
    /// 所有者 ID
    pub owner_id: String,
    /// 权限列表
    pub permissions: Vec<Permission>,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 过期时间 (None 表示永不过期)
    pub expires_at: Option<DateTime<Utc>>,
    /// 是否已撤销
    pub revoked: bool,
}

impl ApiKey {
    /// 检查 Key 是否有效 (未过期且未撤销)
    pub fn is_valid(&self) -> bool {
        if self.revoked {
            return false;
        }
        if let Some(expires) = self.expires_at {
            return Utc::now() < expires;
        }
        true
    }

    /// 检查是否拥有指定权限
    pub fn has_permission(&self, perm: Permission) -> bool {
        // Admin 拥有所有权限
        if self.permissions.contains(&Permission::Admin) {
            return true;
        }
        self.permissions.contains(&perm)
    }
}

/// 认证管理器
#[derive(Clone)]
pub struct AuthManager {
    /// Key -> ApiKey 映射
    keys: Arc<DashMap<String, ApiKey>>,
    /// 开发模式 Master Key (仅用于开发)
    dev_master_key: Option<String>,
}

impl AuthManager {
    /// 创建新的认证管理器
    pub fn new() -> Self {
        Self {
            keys: Arc::new(DashMap::new()),
            dev_master_key: None,
        }
    }

    /// 创建带开发 Master Key 的认证管理器
    pub fn with_dev_key(master_key: impl Into<String>) -> Self {
        let master_key = master_key.into();
        let manager = Self {
            keys: Arc::new(DashMap::new()),
            dev_master_key: Some(master_key.clone()),
        };
        
        // 注册开发 Master Key (拥有所有权限)
        let dev_api_key = ApiKey {
            key: master_key,
            owner_id: "dev-admin".to_string(),
            permissions: vec![Permission::Lock, Permission::Upload, Permission::Download, Permission::Admin],
            created_at: Utc::now(),
            expires_at: None,
            revoked: false,
        };
        manager.keys.insert(dev_api_key.key.clone(), dev_api_key);
        
        manager
    }

    /// 验证 API Key，返回 Key 信息 (如果有效)
    pub fn validate_key(&self, key: &str) -> Option<ApiKey> {
        self.keys.get(key).and_then(|api_key| {
            if api_key.is_valid() {
                Some(api_key.clone())
            } else {
                None
            }
        })
    }

    /// 检查 Key 是否拥有指定权限
    pub fn has_permission(&self, key: &str, perm: Permission) -> bool {
        self.validate_key(key)
            .map(|api_key| api_key.has_permission(perm))
            .unwrap_or(false)
    }

    /// 生成新的 API Key
    pub fn generate_key(
        &self,
        owner_id: &str,
        permissions: Vec<Permission>,
        expires_in_days: Option<i64>,
    ) -> ApiKey {
        let key = format!("ht_{}", Uuid::new_v4().to_string().replace("-", ""));
        let expires_at = expires_in_days.map(|days| {
            Utc::now() + chrono::Duration::days(days)
        });

        let api_key = ApiKey {
            key: key.clone(),
            owner_id: owner_id.to_string(),
            permissions,
            created_at: Utc::now(),
            expires_at,
            revoked: false,
        };

        self.keys.insert(key, api_key.clone());
        api_key
    }

    /// 撤销 API Key
    pub fn revoke_key(&self, key: &str) -> bool {
        if let Some(mut api_key) = self.keys.get_mut(key) {
            api_key.revoked = true;
            true
        } else {
            false
        }
    }

    /// 列出所有 API Keys (管理员功能)
    pub fn list_keys(&self) -> Vec<ApiKey> {
        self.keys.iter().map(|kv| kv.value().clone()).collect()
    }

    /// 检查是否为开发 Master Key
    pub fn is_dev_master_key(&self, key: &str) -> bool {
        self.dev_master_key.as_ref().map(|k| k == key).unwrap_or(false)
    }
}

impl Default for AuthManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dev_master_key() {
        let manager = AuthManager::with_dev_key("test-master-key");
        
        // Master Key 应该有效
        assert!(manager.validate_key("test-master-key").is_some());
        
        // Master Key 应该拥有所有权限
        assert!(manager.has_permission("test-master-key", Permission::Admin));
        assert!(manager.has_permission("test-master-key", Permission::Lock));
        assert!(manager.has_permission("test-master-key", Permission::Upload));
    }

    #[test]
    fn test_generate_and_validate_key() {
        let manager = AuthManager::new();
        
        let api_key = manager.generate_key("alice", vec![Permission::Lock, Permission::Download], None);
        
        // 新生成的 Key 应该有效
        assert!(manager.validate_key(&api_key.key).is_some());
        
        // 权限检查
        assert!(manager.has_permission(&api_key.key, Permission::Lock));
        assert!(manager.has_permission(&api_key.key, Permission::Download));
        assert!(!manager.has_permission(&api_key.key, Permission::Admin));
    }

    #[test]
    fn test_revoke_key() {
        let manager = AuthManager::new();
        
        let api_key = manager.generate_key("bob", vec![Permission::Upload], None);
        assert!(manager.validate_key(&api_key.key).is_some());
        
        // 撤销 Key
        assert!(manager.revoke_key(&api_key.key));
        
        // 撤销后应该无效
        assert!(manager.validate_key(&api_key.key).is_none());
    }

    #[test]
    fn test_invalid_key() {
        let manager = AuthManager::new();
        
        // 不存在的 Key 应该返回 None
        assert!(manager.validate_key("invalid-key").is_none());
        assert!(!manager.has_permission("invalid-key", Permission::Lock));
    }
}
