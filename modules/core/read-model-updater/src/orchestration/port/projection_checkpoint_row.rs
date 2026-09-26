//! `ProjectionCheckpointRow` — `amadeus_projection_checkpoint` の 1 行 (投影 1 つぶんの処理した
//! シーケンス番号)。

use super::JournalAnchor;
use crate::orchestration::{GlobalSeqNr, ProjectionName};

/// `amadeus_projection_checkpoint` の 1 行。主キーは投影名。
///
/// **処理したシーケンス番号はリードモデル側の状態である** (オーナー裁定 2026-09-26)。値は
/// ジャーナル上の位置 (全集約横断の通番) であって、集約内の通番ではない。正の位置には、その
/// 位置のジャーナル行の識別子 ([`JournalAnchor`]) を併記する (`ZERO` は「まだ何も投影して
/// いない」なので対応する行が無く、アンカーも無い)。
///
/// 行は値を運ぶだけである。前進の単調性とアンカーの照合は更新器が持つ。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionCheckpointRow {
    projection: ProjectionName,
    position: GlobalSeqNr,
    anchor: Option<JournalAnchor>,
}

impl ProjectionCheckpointRow {
    /// 行の値を束ねる (**この型の唯一の構築経路**)。
    #[must_use]
    pub const fn new(
        projection: ProjectionName,
        position: GlobalSeqNr,
        anchor: Option<JournalAnchor>,
    ) -> Self {
        Self {
            projection,
            position,
            anchor,
        }
    }

    /// 主キー — 投影名。
    #[must_use]
    pub const fn projection(&self) -> &ProjectionName {
        &self.projection
    }

    /// 処理したシーケンス番号 (ジャーナル上の位置)。
    #[must_use]
    pub const fn position(&self) -> GlobalSeqNr {
        self.position
    }

    /// 位置の行の識別子 (保存されていなければ `None`)。
    #[must_use]
    pub const fn anchor(&self) -> Option<&JournalAnchor> {
        self.anchor.as_ref()
    }
}
