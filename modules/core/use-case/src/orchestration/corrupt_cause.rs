//! `CorruptCause` — 「行は読めたがドメインへ写せない」理由の分類 (entities.md)。

use std::fmt;

/// `Corrupt` の原因分類 (材料)。
///
/// [`JournalReadError`] と [`RepositoryError`] が共有する語彙である — 同じ状態を 2 つの面が
/// 別の名前で語らないようにするため、分類はここ 1 か所で定義する。
///
/// [`JournalReadError`]: super::journal_read_error::JournalReadError
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
