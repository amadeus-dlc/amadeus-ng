//! 自己診断 (`aidlc --doctor`) を所有する集約 — 観測を評価し、診断の事実を吐く。
//!
//! 判断 (D1.a〜D5.b の成否・ラベル・修復案) はこの集約の側にある (オーナー裁定 2026-09-12:
//! 「doctor は、コマンド側集約が処理してイベントを吐き出し、RMU がイベントからリードモデルを
//! 作り、クエリ側がその結果を表示」)。観測 (ファイル・環境・ストアの読取) は合成ルートが
//! 行い、値オブジェクト [`DoctorObservation`] として渡す。診断は正本を修復・再初期化しない。

use super::{
    DoctorChecks, DoctorObservation, HookHealthTarget, WorkspaceDoctorError, WorkspaceDoctorEvent,
    WorkspaceDoctorEventId, WorkspaceDoctorId,
};
use chrono::{DateTime, Utc};

/// 診断対象 (space と record) ごとに、最新の診断結果を保持する。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceDoctor {
    id: WorkspaceDoctorId,
    target: HookHealthTarget,
    checks: DoctorChecks,
    seq_nr: usize,
    version: usize,
    diagnosed_at: DateTime<Utc>,
}

impl WorkspaceDoctor {
    /// 保存された全状態を検査して組む完全コンストラクタ。
    ///
    /// # Errors
    ///
    /// 識別子と対象が不一致、または通番が 0 の場合。
    pub fn new(
        id: WorkspaceDoctorId,
        target: HookHealthTarget,
        checks: DoctorChecks,
        seq_nr: usize,
        version: usize,
        diagnosed_at: DateTime<Utc>,
    ) -> Result<Self, WorkspaceDoctorError> {
        if id != WorkspaceDoctorId::for_target(&target) {
            return Err(WorkspaceDoctorError::TargetMismatch);
        }
        if seq_nr == 0 {
            return Err(WorkspaceDoctorError::InvalidHistory);
        }
        Ok(Self {
            id,
            target,
            checks,
            seq_nr,
            version,
            diagnosed_at,
        })
    }

    /// 診断対象から集約 ID を導く。
    #[must_use]
    pub fn id_for(target: &HookHealthTarget) -> WorkspaceDoctorId {
        WorkspaceDoctorId::for_target(target)
    }

    /// 最初の診断を実施し、集約と誕生事実を対で返す。
    ///
    /// # Errors
    ///
    /// 対象から有効な履歴を構築できない場合。
    pub fn start(
        target: HookHealthTarget,
        observation: &DoctorObservation,
        at: DateTime<Utc>,
    ) -> Result<(Self, WorkspaceDoctorEvent), WorkspaceDoctorError> {
        let id = Self::id_for(&target);
        let event = WorkspaceDoctorEvent::new(
            WorkspaceDoctorEventId::generate(),
            id.clone(),
            target.clone(),
            DoctorChecks::evaluate(observation),
        )?;
        let doctor = Self::new(id, target, event.checks().clone(), 1, 0, at)?;
        Ok((doctor, event))
    }

    /// 後続の診断を実施し、1 イベントとして記録する。
    ///
    /// # Errors
    ///
    /// 通番を増やせない場合。
    pub fn diagnose(
        &mut self,
        observation: &DoctorObservation,
        at: DateTime<Utc>,
    ) -> Result<WorkspaceDoctorEvent, WorkspaceDoctorError> {
        let sequence = self
            .seq_nr
            .checked_add(1)
            .ok_or(WorkspaceDoctorError::CounterExhausted)?;
        let event = WorkspaceDoctorEvent::new(
            WorkspaceDoctorEventId::generate(),
            self.id.clone(),
            self.target.clone(),
            DoctorChecks::evaluate(observation),
        )?;
        self.apply_event(&event, sequence, at);
        Ok(event)
    }

    /// 保存された事実を直接適用する。
    ///
    /// # Panics
    ///
    /// 壊れた対象 / 集約 ID / 通番を含む履歴 (再構成は失敗を返さない — オーナー裁定 2026-08-30)。
    pub fn apply_event(&mut self, event: &WorkspaceDoctorEvent, seq: usize, at: DateTime<Utc>) {
        assert!(
            event.aggregate_id() == &self.id
                && event.target() == &self.target
                && self.seq_nr.checked_add(1) == Some(seq),
            "invalid WorkspaceDoctor history"
        );
        self.checks = event.checks().clone();
        self.seq_nr = seq;
        self.diagnosed_at = at;
    }

    /// snapshot より後の保存事実だけを畳む。
    ///
    /// # Panics
    ///
    /// 保存履歴の不整合。
    #[must_use]
    pub fn replay(
        mut snapshot: Self,
        events: impl IntoIterator<Item = (WorkspaceDoctorEvent, usize, DateTime<Utc>)>,
    ) -> Self {
        for (event, seq, at) in events {
            snapshot.apply_event(&event, seq, at);
        }
        snapshot
    }

    /// 永続化境界が返す版を伴う新しい完成値。
    #[must_use]
    pub const fn with_version(mut self, version: usize) -> Self {
        self.version = version;
        self
    }

    /// 集約の識別子。
    #[must_use]
    pub const fn id(&self) -> &WorkspaceDoctorId {
        &self.id
    }

    /// 診断の対象。
    #[must_use]
    pub const fn target(&self) -> &HookHealthTarget {
        &self.target
    }

    /// 最新の診断の行 (表示順)。
    #[must_use]
    pub const fn checks(&self) -> &DoctorChecks {
        &self.checks
    }

    /// この集約の履歴通番。
    #[must_use]
    pub const fn seq_nr(&self) -> usize {
        self.seq_nr
    }

    /// 永続化境界の楽観ロック版。
    #[must_use]
    pub const fn version(&self) -> usize {
        self.version
    }

    /// 最新の診断の時刻。保存封筒の `occurred_at` へ渡す。
    #[must_use]
    pub const fn diagnosed_at(&self) -> DateTime<Utc> {
        self.diagnosed_at
    }
}
