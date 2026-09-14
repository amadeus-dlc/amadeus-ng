//! 自己診断 (`aidlc --doctor`) の観測入力 — C5 の `observed_configuration` の写し。
//!
//! 観測元 (環境・ファイル・ストア) ごとの事実を束ねる値オブジェクト。判定は持たない —
//! 「何が見えたか」だけを運び、成否・ラベル・修復案は集約 [`WorkspaceDoctor`] が決める。
//! 観測そのもの (ファイル・環境・ストアの読取) は合成ルートの仕事であり、ドメインは
//! serde・SQL・ファイル I/O を持たない。
//!
//! [`WorkspaceDoctor`]: super::WorkspaceDoctor

use super::{
    DefinitionAssets, HeartbeatObservation, HookWiring, NativeEntryPoints, RecordObservation,
    WorkspaceShell,
};

/// 1 回の診断で観測した事実の全部。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoctorObservation {
    bun_found: bool,
    entry_points: NativeEntryPoints,
    hook_wiring: HookWiring,
    heartbeat: HeartbeatObservation,
    shell: WorkspaceShell,
    definition: DefinitionAssets,
    record: Option<RecordObservation>,
}

impl DoctorObservation {
    /// 観測を束ねる。
    #[must_use]
    pub const fn new(
        bun_found: bool,
        entry_points: NativeEntryPoints,
        hook_wiring: HookWiring,
        heartbeat: HeartbeatObservation,
        shell: WorkspaceShell,
        definition: DefinitionAssets,
        record: Option<RecordObservation>,
    ) -> Self {
        Self {
            bun_found,
            entry_points,
            hook_wiring,
            heartbeat,
            shell,
            definition,
            record,
        }
    }

    /// Bun が PATH または `~/.bun/bin/bun` に在るか。
    #[must_use]
    pub const fn bun_found(&self) -> bool {
        self.bun_found
    }

    /// D1.b の観測。
    #[must_use]
    pub const fn entry_points(&self) -> &NativeEntryPoints {
        &self.entry_points
    }

    /// D2.a〜D2.e の観測。
    #[must_use]
    pub const fn hook_wiring(&self) -> &HookWiring {
        &self.hook_wiring
    }

    /// D2.f の観測。
    #[must_use]
    pub const fn heartbeat(&self) -> &HeartbeatObservation {
        &self.heartbeat
    }

    /// D3.a の観測。
    #[must_use]
    pub const fn shell(&self) -> &WorkspaceShell {
        &self.shell
    }

    /// D3.b〜D3.d の観測。
    #[must_use]
    pub const fn definition(&self) -> &DefinitionAssets {
        &self.definition
    }

    /// D4 / D5 の観測 (作業記録・ストアがまだ無い初回状態では `None`)。
    #[must_use]
    pub const fn record(&self) -> Option<&RecordObservation> {
        self.record.as_ref()
    }
}
