//! 自己診断の事実 1 件の読取レコード — ジャーナルの行を、我々が所有する型へ写したもの。

use chrono::{DateTime, Utc};
use core_command_domain::workspace::WorkspaceDoctorEvent;

use crate::orchestration::GlobalSeqNr;

/// ジャーナルから読んだ自己診断の事実 1 件。
///
/// 本家の封筒型はポートから出さない (`JournalEntry` と同じ方針)。運ぶのは、ジャーナル上の
/// 位置 (全集約横断の通番)・集約内の通番・発生時刻・事実の 4 つだけで、リードモデル側の
/// 型 (行・表) は知らない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceDoctorJournalEntry {
    position: GlobalSeqNr,
    seq_nr: usize,
    occurred_at: DateTime<Utc>,
    event: WorkspaceDoctorEvent,
}

impl WorkspaceDoctorJournalEntry {
    /// 検査済みの行を束ねる。
    #[must_use]
    pub const fn new(
        position: GlobalSeqNr,
        seq_nr: usize,
        occurred_at: DateTime<Utc>,
        event: WorkspaceDoctorEvent,
    ) -> Self {
        Self {
            position,
            seq_nr,
            occurred_at,
            event,
        }
    }

    /// ジャーナル上の位置。更新器はこれを処理したシーケンス番号として保存する。
    #[must_use]
    pub const fn position(&self) -> GlobalSeqNr {
        self.position
    }

    /// 集約内の通番 (集約の `replay` へ渡す)。
    #[must_use]
    pub const fn seq_nr(&self) -> usize {
        self.seq_nr
    }

    /// 元の発生時刻 (集約の `replay` へ渡す)。
    #[must_use]
    pub const fn occurred_at(&self) -> DateTime<Utc> {
        self.occurred_at
    }

    /// 記録された事実。
    #[must_use]
    pub const fn event(&self) -> &WorkspaceDoctorEvent {
        &self.event
    }
}
