//! `DiffCodekbScopeUseCase` — ストアの走査範囲と現在のツリーを突き合わせる (upstream の
//! status モード)。

use crate::orchestration::{
    CodekbScopeDao, CodekbScopeDiffView, CodekbSourceFingerprintDao, ReScopeParseView, ReScopeView,
    ReadModelReadError,
};

/// ストアが記録した走査範囲が、いまも現物と一致しているかを引く。
///
/// 2 つの読取源 — ストアの走査範囲ブロックと、作業ツリーの内容指紋 — を突き合わせて 1 つの
/// 判定を組む。組むのは規則 6 (2026-09-03) がクエリ側ユースケースに認めた組み立てである
/// (`coding-rules/cqrs-boundaries.md`)。判定の**綴り**は持たない — 出す側が描く。
///
/// バインディングはスタティックが既定なので DAO は型パラメータで保持する
/// (`coding-rules/use-case-rules.md` §2)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffCodekbScopeUseCase<S: CodekbScopeDao, F: CodekbSourceFingerprintDao> {
    store_dao: S,
    fingerprint_dao: F,
}

impl<S: CodekbScopeDao, F: CodekbSourceFingerprintDao> DiffCodekbScopeUseCase<S, F> {
    /// 2 つの引当の口を注入する (**この型の唯一の構築経路**)。
    #[must_use]
    pub const fn new(store_dao: S, fingerprint_dao: F) -> DiffCodekbScopeUseCase<S, F> {
        DiffCodekbScopeUseCase {
            store_dao,
            fingerprint_dao,
        }
    }

    /// 鮮度の判定を引く。
    ///
    /// # Errors
    ///
    /// リードモデルを引けない ([`ReadModelReadError`])。
    pub fn execute(&self) -> Result<CodekbScopeDiffView, ReadModelReadError> {
        let store = match read_store(&self.store_dao)? {
            Ok(store) => store,
            Err(short_circuit) => return Ok(short_circuit),
        };

        // 範囲が空のストアは指紋を取りに行かない (upstream も `analyzedPaths.length > 0` で
        // 判定してから計算する)。
        let current = if store.analyzed_paths().is_empty() {
            None
        } else {
            self.fingerprint_dao.find(store.analyzed_paths())
        };

        // ストアが指紋を持たない場合が先である — 両方欠けていても「記録していない」と報せる
        // (upstream の三項が `store.fingerprint === null` を先に見るのと同じ)。
        let Some(store_fingerprint) = store.fingerprint() else {
            return Ok(CodekbScopeDiffView::UnverifiedWithoutFingerprint {
                store_intent: store.intent().to_string(),
                kind: store.kind().to_string(),
                analyzed_paths: store.analyzed_paths().to_vec(),
            });
        };
        let Some(current_fingerprint) = current else {
            return Ok(CodekbScopeDiffView::UnverifiedNotComputable {
                store_intent: store.intent().to_string(),
                kind: store.kind().to_string(),
                analyzed_paths: store.analyzed_paths().to_vec(),
            });
        };

        let unchanged = store_fingerprint == current_fingerprint;
        let store_intent = store.intent().to_string();
        let kind = store.kind().to_string();
        let analyzed_paths = store.analyzed_paths().to_vec();
        let store_fingerprint = store_fingerprint.to_string();
        Ok(if unchanged {
            CodekbScopeDiffView::Current {
                store_intent,
                kind,
                analyzed_paths,
                store_fingerprint,
                current_fingerprint,
            }
        } else {
            CodekbScopeDiffView::Stale {
                store_intent,
                kind,
                analyzed_paths,
                store_fingerprint,
                current_fingerprint,
            }
        })
    }
}

/// ストア側の 3 つの短絡 (ストアが無い / ブロックが無い / 綴りが通らない) を先に片付ける。
///
/// upstream はモードを見る**前に**この 3 つを返すので、status と compare の双方がここを通る。
/// 読めた場合だけ `Ok(Ok(scope))` を返し、短絡した場合は `Ok(Err(判定))` を返す。
pub(crate) fn read_store<S: CodekbScopeDao>(
    store_dao: &S,
) -> Result<Result<ReScopeView, CodekbScopeDiffView>, ReadModelReadError> {
    let Some(parsed) = store_dao.find()? else {
        return Ok(Err(CodekbScopeDiffView::NoStore));
    };
    Ok(match parsed {
        ReScopeParseView::Parsed(scope) => Ok(scope),
        ReScopeParseView::Absent(detail) => Err(CodekbScopeDiffView::StoreScopeAbsent { detail }),
        ReScopeParseView::Malformed(detail) => {
            Err(CodekbScopeDiffView::StoreScopeMalformed { detail })
        }
    })
}
