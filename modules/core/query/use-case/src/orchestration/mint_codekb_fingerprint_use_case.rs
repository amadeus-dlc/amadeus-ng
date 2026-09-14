//! `MintCodekbFingerprintUseCase` — 走査範囲の内容指紋を鋳造する (upstream の `--mint`)。

use crate::orchestration::CodekbSourceFingerprintDao;

/// 与えたパスに限った作業ツリーの内容指紋を引く。
///
/// これは RE ステージが走査範囲ブロックの `fingerprint:` 行へ**そのまま書き写す**値である。
/// 本体は `execute(鍵) = dao.find(鍵)` だけで、判断・導出・文言組立のどれも持たない
/// (`coding-rules/cqrs-boundaries.md` 規則 6)。計算できないときに何と描くか (`unknown`) を
/// 決めるのは出す側である。
///
/// バインディングはスタティックが既定なので DAO は型パラメータで保持する
/// (`coding-rules/use-case-rules.md` §2)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MintCodekbFingerprintUseCase<F: CodekbSourceFingerprintDao> {
    fingerprint_dao: F,
}

impl<F: CodekbSourceFingerprintDao> MintCodekbFingerprintUseCase<F> {
    /// 引当の口を注入する (**この型の唯一の構築経路**)。
    #[must_use]
    pub const fn new(fingerprint_dao: F) -> MintCodekbFingerprintUseCase<F> {
        MintCodekbFingerprintUseCase { fingerprint_dao }
    }

    /// 与えたパスに限った内容指紋を引く (計算できなければ `None`)。
    #[must_use]
    pub fn execute(&self, paths: &[String]) -> Option<String> {
        self.fingerprint_dao.find(paths)
    }
}
