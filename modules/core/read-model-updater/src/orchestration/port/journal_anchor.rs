//! `JournalAnchor` — ジャーナルの 1 行を名指す識別子 (集約 ID と集約内の通番)。

/// ジャーナルのある位置 (`rowid`) にある行の識別子 — 集約 ID (`aid`) と集約内の通番 (`seq_nr`)。
///
/// 処理したシーケンス番号 (ジャーナル上の位置) を保存するとき、その位置の行の識別子を
/// **アンカー**として併記する。読むときはアンカーをジャーナルの同じ位置の行と照らし合わせ、
/// 食い違えば (位置の振り直し・ジャーナルの改変の兆候) 静かに読み進めず止める。
///
/// ジャーナルの読み手 ([`super::StructuredJournalReader::anchor_at`]) が返し、チェックポイントの
/// 表の DAO ([`super::ProjectionCheckpointDao`]) が保存する。照らし合わせるのは更新器である
/// (ジャーナルとチェックポイントの表をまたぐ検査なので、DAO には置かない)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JournalAnchor {
    aggregate_id: String,
    seq_nr: usize,
}

impl JournalAnchor {
    /// 行の識別子を束ねる (**この型の唯一の構築経路**)。
    #[must_use]
    pub const fn new(aggregate_id: String, seq_nr: usize) -> Self {
        Self {
            aggregate_id,
            seq_nr,
        }
    }

    /// 行が名乗る集約 ID (生文字列 — 破損した行の識別子を運ぶこともある)。
    #[must_use]
    pub fn aggregate_id(&self) -> &str {
        &self.aggregate_id
    }

    /// 集約内の通番。
    #[must_use]
    pub const fn seq_nr(&self) -> usize {
        self.seq_nr
    }
}
