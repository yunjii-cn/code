/// 多入口（API / Webhook / Cron）— M6.4 W16
///
/// 每个入口本质是一个“触发器”，可以接收外部请求并派发到指定 worker 执行。
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EntryKind {
    /// HTTP API endpoint
    Api,
    /// Webhook receiver (e.g. GitHub / Slack / DingTalk)
    Webhook,
    /// Periodic cron job
    Cron,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiEntry {
    /// Unique ID
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// API / Webhook / Cron
    pub kind: EntryKind,
    /// Target worker role (empty = any worker)
    pub target_role: Option<String>,
    /// For API: path like "/api/hello"
    /// For Webhook: URL pattern like "/webhook/github"
    /// For Cron: cron expression like "0 */6 * * *"
    pub route_or_expr: String,
    /// Optional secret/token for authentication
    pub auth_token: Option<String>,
    /// Whether this entry is active
    pub enabled: bool,
    /// Extra config (JSON map)
    pub config: HashMap<String, String>,
}

impl MultiEntry {
    pub fn new(id: impl Into<String>, name: impl Into<String>, kind: EntryKind, route_or_expr: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            kind,
            target_role: None,
            route_or_expr: route_or_expr.into(),
            auth_token: None,
            enabled: true,
            config: HashMap::new(),
        }
    }

    pub fn with_auth(mut self, token: impl Into<String>) -> Self {
        self.auth_token = Some(token.into());
        self
    }

    pub fn with_target(mut self, role: impl Into<String>) -> Self {
        self.target_role = Some(role.into());
        self
    }

    pub fn with_config(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.config.insert(key.into(), value.into());
        self
    }
}

/// Registry for all multi-entry triggers.
pub struct MultiEntryRegistry {
    entries: Mutex<Vec<MultiEntry>>,
}

impl MultiEntryRegistry {
    pub fn new() -> Self {
        Self {
            entries: Mutex::new(Vec::new()),
        }
    }

    pub fn register(&self, entry: MultiEntry) {
        self.entries.lock().unwrap().push(entry);
    }

    pub fn unregister(&self, id: &str) {
        let mut entries = self.entries.lock().unwrap();
        entries.retain(|e| e.id != id);
    }

    pub fn list(&self) -> Vec<MultiEntry> {
        self.entries.lock().unwrap().clone()
    }

    pub fn find_by_kind(&self, kind: &EntryKind) -> Vec<MultiEntry> {
        self.entries
            .lock()
            .unwrap()
            .iter()
            .filter(|e| &e.kind == kind)
            .cloned()
            .collect()
    }

    pub fn find_enabled(&self) -> Vec<MultiEntry> {
        self.entries
            .lock()
            .unwrap()
            .iter()
            .filter(|e| e.enabled)
            .cloned()
            .collect()
    }

    pub fn find_by_route(&self, route: &str) -> Option<MultiEntry> {
        self.entries
            .lock()
            .unwrap()
            .iter()
            .find(|e| e.route_or_expr == route)
            .cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_and_list() {
        let reg = MultiEntryRegistry::new();
        let entry = MultiEntry::new("e1", "Test API", EntryKind::Api, "/api/test");
        reg.register(entry);
        let list = reg.list();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, "e1");
    }

    #[test]
    fn test_unregister() {
        let reg = MultiEntryRegistry::new();
        reg.register(MultiEntry::new("e1", "Test", EntryKind::Api, "/api/test"));
        reg.unregister("e1");
        assert!(reg.list().is_empty());
    }

    #[test]
    fn test_find_by_kind() {
        let reg = MultiEntryRegistry::new();
        reg.register(MultiEntry::new("e1", "API", EntryKind::Api, "/api/test"));
        reg.register(MultiEntry::new("e2", "Webhook", EntryKind::Webhook, "/webhook/github"));
        reg.register(MultiEntry::new("e3", "Cron", EntryKind::Cron, "0 */6 * * *"));
        let apis = reg.find_by_kind(&EntryKind::Api);
        assert_eq!(apis.len(), 1);
        assert_eq!(apis[0].name, "API");
    }

    #[test]
    fn test_find_enabled() {
        let reg = MultiEntryRegistry::new();
        let mut e1 = MultiEntry::new("e1", "Enabled", EntryKind::Api, "/api/test");
        let mut e2 = MultiEntry::new("e2", "Disabled", EntryKind::Api, "/api/test2");
        e2.enabled = false;
        reg.register(e1);
        reg.register(e2);
        let enabled = reg.find_enabled();
        assert_eq!(enabled.len(), 1);
        assert_eq!(enabled[0].id, "e1");
    }
}