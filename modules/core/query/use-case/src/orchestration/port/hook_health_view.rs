//! HookHealth read_hook_health 行のQuery DTO。
/// ドメイン型を持たない、投影済みの稼働観測行。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HookHealthView {
    id: String,
    target: String,
    hook: String,
    heartbeat: Option<String>,
    seq_nr: u64,
    drops: u64,
    latest_drop: Option<String>,
}
impl HookHealthView {
    /// 完全な投影行を組む。
    #[must_use]
    pub const fn new(
        id: String,
        target: String,
        hook: String,
        heartbeat: Option<String>,
        seq_nr: u64,
        drops: u64,
        latest_drop: Option<String>,
    ) -> Self {
        Self {
            id,
            target,
            hook,
            heartbeat,
            seq_nr,
            drops,
            latest_drop,
        }
    }
    /// 集約ID。
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }
    /// 観測領域。
    #[must_use]
    pub fn target(&self) -> &str {
        &self.target
    }
    /// フック名。
    #[must_use]
    pub fn hook(&self) -> &str {
        &self.hook
    }
    /// 最終heartbeat。
    #[must_use]
    pub fn heartbeat(&self) -> Option<&str> {
        self.heartbeat.as_deref()
    }
    /// 履歴通番。
    #[must_use]
    pub const fn seq_nr(&self) -> u64 {
        self.seq_nr
    }
    /// drop件数。
    #[must_use]
    pub const fn drops(&self) -> u64 {
        self.drops
    }
    /// 最新drop理由。
    #[must_use]
    pub fn latest_drop(&self) -> Option<&str> {
        self.latest_drop.as_deref()
    }
}
