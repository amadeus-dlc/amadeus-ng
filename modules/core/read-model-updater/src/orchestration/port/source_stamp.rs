//! `SourceStamp` — 参照入力由来の行が「どの入力から・どの履歴位置で」作られたかの読取レコード。

use crate::orchestration::GlobalSeqNr;

/// 保存済みの行が名乗る出所 — `source_digest` 列と `as_of` 列の組。
///
/// 参照入力由来の面 (テスト契約・計画指紋・Code Generation 開始可否) は、ジャーナル上の位置
/// ではなく入力のダイジェストで冪等を取る。更新器はこの組を表の DAO で読み、いま組んだ行と
/// 比べて書くかどうかを決める (判断は更新器の側にあり、この型と DAO は値を運ぶだけである)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceStamp {
    source_digest: String,
    as_of: GlobalSeqNr,
}

impl SourceStamp {
    /// 列の値を束ねる。
    #[must_use]
    pub const fn new(source_digest: String, as_of: GlobalSeqNr) -> Self {
        Self {
            source_digest,
            as_of,
        }
    }

    /// 行を作った参照入力の照合子。
    #[must_use]
    pub fn source_digest(&self) -> &str {
        &self.source_digest
    }

    /// 行を作ったときの履歴位置。
    #[must_use]
    pub const fn as_of(&self) -> GlobalSeqNr {
        self.as_of
    }
}
