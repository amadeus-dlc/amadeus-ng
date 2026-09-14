//! `CompareCodekbScopeUseCase` — 取込側の走査がストアの主張を覆うかを突き合わせる
//! (upstream の `--compare` モード)。

use crate::orchestration::{
    CodekbScopeDao, CodekbScopeDiffView, CompareCodekbScopeError, ReScopeParseView, ReScopeView,
};

use super::diff_codekb_scope_use_case::read_store;

/// 取込側の走査記録が、ストアが主張する検証済みの範囲を覆うかを引く。
///
/// これは**上書きで知識を失わないための関門**である。狭い走査で丸ごと置き換えると、前の
/// intent が深く読んで得た主張が黙って消える。だから覆えていない分を名指して返し、人間が
/// 再利用・再走査・置換をゲートで選べるようにする。
///
/// ストア側の 3 つの短絡は status モードと共有する ([`read_store`]) — upstream もモードを
/// 見る前にそれを返すからである。
///
/// バインディングはスタティックが既定なので DAO は型パラメータで保持する
/// (`coding-rules/use-case-rules.md` §2)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompareCodekbScopeUseCase<S: CodekbScopeDao, I: CodekbScopeDao> {
    store_dao: S,
    incoming_dao: I,
}

impl<S: CodekbScopeDao, I: CodekbScopeDao> CompareCodekbScopeUseCase<S, I> {
    /// 2 つの引当の口を注入する (**この型の唯一の構築経路**)。
    #[must_use]
    pub const fn new(store_dao: S, incoming_dao: I) -> CompareCodekbScopeUseCase<S, I> {
        CompareCodekbScopeUseCase {
            store_dao,
            incoming_dao,
        }
    }

    /// 突合の判定を引く。
    ///
    /// # Errors
    ///
    /// リードモデルを引けない、または名指された突合相手が無い
    /// ([`CompareCodekbScopeError`])。相手が無いのは**判定ではなく拒否**である — ストア側の
    /// 短絡と違い、呼び出しそのものが成立していない。
    pub fn execute(&self) -> Result<CodekbScopeDiffView, CompareCodekbScopeError> {
        let store =
            match read_store(&self.store_dao).map_err(CompareCodekbScopeError::Unreadable)? {
                Ok(store) => store,
                Err(short_circuit) => return Ok(short_circuit),
            };

        let incoming = match self
            .incoming_dao
            .find()
            .map_err(CompareCodekbScopeError::Unreadable)?
            .ok_or(CompareCodekbScopeError::IncomingMissing)?
        {
            ReScopeParseView::Parsed(scope) => scope,
            ReScopeParseView::Absent(detail) => {
                return Ok(CodekbScopeDiffView::IncomingScopeAbsent { detail });
            }
            ReScopeParseView::Malformed(detail) => {
                return Ok(CodekbScopeDiffView::IncomingScopeMalformed { detail });
            }
        };

        Ok(compare(&store, &incoming))
    }
}

/// 2 つの走査範囲を突き合わせて判定を組む。
fn compare(store: &ReScopeView, incoming: &ReScopeView) -> CodekbScopeDiffView {
    let incoming_is_full = incoming.kind() == "full";
    // 全体を読んだ取込側は何も捨てない。逆にストアが全体で取込側が部分なら、`./` の主張
    // そのものが失われる (個々のパスの包含では表せない格下げ)。
    let full_scope_downgrade = store.kind() == "full" && !incoming_is_full;
    let discarded_paths: Vec<String> = if incoming_is_full {
        Vec::new()
    } else if full_scope_downgrade {
        store.analyzed_paths().to_vec()
    } else {
        store
            .analyzed_paths()
            .iter()
            .filter(|path| !covered(incoming.analyzed_paths(), path))
            .cloned()
            .collect()
    };
    let discarded_components: Vec<String> = if incoming_is_full {
        Vec::new()
    } else {
        store
            .analyzed_components()
            .iter()
            .filter(|component| !incoming.analyzed_components().contains(component))
            .cloned()
            .collect()
    };

    let store_intent = store.intent().to_string();
    let incoming_intent = incoming.intent().to_string();
    if discarded_paths.is_empty() && discarded_components.is_empty() {
        CodekbScopeDiffView::Covers {
            store_intent,
            incoming_intent,
        }
    } else {
        CodekbScopeDiffView::Narrower {
            store_intent,
            incoming_intent,
            discarded_paths,
            discarded_components,
        }
    }
}

/// 取込側の一覧がストアの 1 パスを覆うか (upstream `scopePathCovered`)。
///
/// 逐語一致か、**ディレクトリの接頭辞** (`/` で終わる項) が飲み込む場合だけである。走査範囲の
/// パスは glob ではなくリポジトリ相対のディレクトリ・ファイルとして書かれるので、接頭辞
/// 判定だけに意図的に絞ってある。
fn covered(incoming: &[String], store_path: &str) -> bool {
    incoming
        .iter()
        .any(|path| path == store_path || (path.ends_with('/') && store_path.starts_with(path)))
}
