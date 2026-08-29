//! `CorruptCause` — 「行は読めたがドメインへ写せない」理由の分類、**コマンド側の閉集合**。

use std::fmt;

/// `Corrupt` の原因分類 (材料) — 集約の永続化と再水和の面で起きうるもの。
///
/// RMU (`core_read_model_updater::orchestration::CorruptCause`) と**同じ名前の別の型**で
/// ある。分類を 1 つの enum で共有すると、コマンド側が RMU を `Cargo.toml` に書くことになり
/// **こちら側では違反である** (`coding-rules/cqrs-boundaries.md` — コマンド側の依存に RMU が
/// 現れたら違反)。加えて、共有しないほうが正確でもある: **実際に起きうる変種だけを各面が
/// 持つ** (無用な変種は「この面ではありえない」という情報を消してしまう)。
///
/// コマンド側に無いのは投影チェックポイントのアンカー不一致
/// (`CheckpointAnchorMismatch`) である — チェックポイントは RMU だけが持つ表であり、
/// Repository は触れないので構成不能である。
///
/// [`RepositoryError`] が運ぶ材料である。
///
/// [`RepositoryError`]: super::repository_error::RepositoryError
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CorruptCause {
    /// ジャーナル行はあるのにスナップショット行が無い (BR1.2)。
    MissingSnapshot,
    /// 行のペイロードをドメイン型へ復号できない (JSON の破損・未知の変種・不変条件違反)。
    UndecodablePayload,
    /// 復元・適用の結果が集約不変条件を破る (`from_state` / `apply_event` の `Err`)。
    InvariantViolation,
    /// 集約内の `seq_nr` が連続していない (呼出側の不整合、またはジャーナルの欠損)。
    SequenceGap,
}

impl fmt::Display for CorruptCause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            CorruptCause::MissingSnapshot => "missing snapshot",
            CorruptCause::UndecodablePayload => "undecodable payload",
            CorruptCause::InvariantViolation => "invariant violation",
            CorruptCause::SequenceGap => "sequence gap",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_corrupt_cause_renders_its_material() {
        assert_eq!(
            CorruptCause::MissingSnapshot.to_string(),
            "missing snapshot"
        );
        assert_eq!(
            CorruptCause::UndecodablePayload.to_string(),
            "undecodable payload"
        );
        assert_eq!(
            CorruptCause::InvariantViolation.to_string(),
            "invariant violation"
        );
        assert_eq!(CorruptCause::SequenceGap.to_string(), "sequence gap");
    }

    #[test]
    fn causes_compare_by_value() {
        assert_eq!(CorruptCause::SequenceGap, CorruptCause::SequenceGap);
        assert_ne!(CorruptCause::SequenceGap, CorruptCause::MissingSnapshot);
    }
}
