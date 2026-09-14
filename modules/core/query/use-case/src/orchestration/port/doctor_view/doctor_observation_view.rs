//! 自己診断 (`aidlc --doctor`) の観測入力 — C5 の `observed_configuration` の写し。
//!
//! 観測元 (環境・ファイル・ストア) ごとの事実を束ねる。判定は持たない — 「何が見えたか」
//! だけを運び、成否・ラベル・修復案はコマンド側の集約 (`WorkspaceDoctor`) が決める。

use super::{
    DefinitionAssetsView, HeartbeatView, HookWiringView, NativeEntryPointsView,
    RecordObservationView, WorkspaceShellView,
};

/// 1 回の診断で観測した事実の全部。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoctorObservationView {
    bun_found: bool,
    entry_points: NativeEntryPointsView,
    hook_wiring: HookWiringView,
    heartbeat: HeartbeatView,
    shell: WorkspaceShellView,
    definition: DefinitionAssetsView,
    record: Option<RecordObservationView>,
}

impl DoctorObservationView {
    /// 観測を束ねる。
    #[must_use]
    pub const fn new(
        bun_found: bool,
        entry_points: NativeEntryPointsView,
        hook_wiring: HookWiringView,
        heartbeat: HeartbeatView,
        shell: WorkspaceShellView,
        definition: DefinitionAssetsView,
        record: Option<RecordObservationView>,
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
    pub const fn entry_points(&self) -> &NativeEntryPointsView {
        &self.entry_points
    }

    /// D2.a〜D2.e の観測。
    #[must_use]
    pub const fn hook_wiring(&self) -> &HookWiringView {
        &self.hook_wiring
    }

    /// D2.f の観測。
    #[must_use]
    pub const fn heartbeat(&self) -> &HeartbeatView {
        &self.heartbeat
    }

    /// D3.a の観測。
    #[must_use]
    pub const fn shell(&self) -> &WorkspaceShellView {
        &self.shell
    }

    /// D3.b〜D3.d の観測。
    #[must_use]
    pub const fn definition(&self) -> &DefinitionAssetsView {
        &self.definition
    }

    /// D4 / D5 の観測 (作業記録・ストアがまだ無い初回状態では `None`)。
    #[must_use]
    pub const fn record(&self) -> Option<&RecordObservationView> {
        self.record.as_ref()
    }
}
