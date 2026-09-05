//! `CorruptCause` — 「行は読めたがドメインへ写せない」理由の分類、**読取面の閉集合**。

use std::fmt;

/// `Corrupt` の原因分類 (材料) — ジャーナルの横断読取とチェックポイントの面で起きうるもの。
///
/// コマンド側 (`core_command_use_case::orchestration::CorruptCause`) と**同じ名前の別の型**で
/// ある。RMU は中間なのでコマンド側を `Cargo.toml` に書くこと自体は許されるが、それでも
/// 共有しない — **実際に起きうる変種だけを各面が持つ**ためである (無用な変種は「この面では
/// ありえない」という情報を消してしまう)。正本も「エラー分類・I/O 写像は側ごとに専用化」と
/// している (`coding-rules/cqrs-boundaries.md` / 構成案 §3)。DRY より面の正確さを採る。
///
/// 読取面に無いのはコマンド側の再水和にしか意味の無い 2 つ — スナップショット行の欠落
/// (`MissingSnapshot`) と集約内通番の飛び (`SequenceGap`) である。横断読取は行を 1 本ずつ
/// 写すだけで、集約を組み立てないので、その 2 つは構成不能である。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CorruptCause {
    /// 行のペイロードをドメイン型へ復号できない (JSON の破損・未知の変種・型判別子の不一致)。
    UndecodablePayload,
    /// 列の値をドメインへ運べない (識別子が `IntentExecutionId` でない・通番が範囲外)。
    InvariantViolation,
    /// 保存済みチェックポイントのアンカー (aid, seq_nr) が journal の同位置と一致しない。
    ///
    /// rowid の振り直し・ジャーナルの改変の兆候であり、このまま差分読取を続けると欠落・
    /// 重複が起きるため、静かな破損ではなく明示エラーで止める材料 (BR1.4)。
    CheckpointAnchorMismatch,
    /// 同じ公開位置を名乗る構造化候補と現在の行集合が一致しない。
    ProjectionSnapshotMismatch,
}

impl fmt::Display for CorruptCause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            CorruptCause::UndecodablePayload => "undecodable payload",
            CorruptCause::InvariantViolation => "invariant violation",
            CorruptCause::CheckpointAnchorMismatch => "checkpoint anchor mismatch",
            CorruptCause::ProjectionSnapshotMismatch => "projection snapshot mismatch",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_corrupt_cause_renders_its_material() {
        assert_eq!(
            CorruptCause::ProjectionSnapshotMismatch.to_string(),
            "projection snapshot mismatch"
        );
        assert_eq!(
            CorruptCause::UndecodablePayload.to_string(),
            "undecodable payload"
        );
        assert_eq!(
            CorruptCause::InvariantViolation.to_string(),
            "invariant violation"
        );
        assert_eq!(
            CorruptCause::CheckpointAnchorMismatch.to_string(),
            "checkpoint anchor mismatch"
        );
    }

    #[test]
    fn causes_compare_by_value() {
        assert_eq!(
            CorruptCause::UndecodablePayload,
            CorruptCause::UndecodablePayload
        );
        assert_ne!(
            CorruptCause::UndecodablePayload,
            CorruptCause::CheckpointAnchorMismatch
        );
    }
}
