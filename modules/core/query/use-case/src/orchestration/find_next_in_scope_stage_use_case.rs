//! `FindNextInScopeStageUseCase` — ある slug の次に来る in-scope の EXECUTE ステージを引く
//! (upstream `nextInScopeStage`)。

use crate::orchestration::{ReadModelReadError, ScopeGridDao, StageGraphDao, StateFileDao};

/// 次の in-scope ステージを引く。
///
/// これは 3 つのリードモデル — ステージグラフ (順序)・scope グリッド (EXECUTE/SKIP)・状態
/// ファイル (承認済み計画の per-stage 上書きと済/skip のチェックボックス) — を **slug という
/// 鍵で突合して 1 つの答えを組む**ものである。答えを組むのは規則 6 (2026-09-03) が
/// クエリ側ユースケースに認めた「FK をたどって表ごとに引き、View を組む」に当たる。ここは
/// ライフサイクルの状態依存の判断 (`next` の 21 分岐ラダー — b26/b27 が誤ってクエリ側に置き
/// b44 で撤去したもの) ではなく、**静的なコンパイル済みグラフの走査**である
/// (`coding-rules/cqrs-boundaries.md` — 規則 7 「クエリサイドはこれを読むだけ」)。
///
/// 状態ファイルは在れば尊重する: per-stage の `EXECUTE`/`SKIP` 上書きはグリッドに優先し、
/// 済/skip のチェックボックスはその slug を飛ばす。状態ファイルが無いワークスペースでも
/// 静的グリッドだけで答える (どちらも読取専用)。
///
/// バインディングはスタティックが既定なので DAO は型パラメータで保持する
/// (`coding-rules/use-case-rules.md` §2)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FindNextInScopeStageUseCase<G: StageGraphDao, S: ScopeGridDao, F: StateFileDao> {
    stage_graph_dao: G,
    scope_grid_dao: S,
    state_file_dao: F,
}

impl<G: StageGraphDao, S: ScopeGridDao, F: StateFileDao> FindNextInScopeStageUseCase<G, S, F> {
    /// 3 つの引当の口を注入する (**この型の唯一の構築経路**)。
    #[must_use]
    pub const fn new(
        stage_graph_dao: G,
        scope_grid_dao: S,
        state_file_dao: F,
    ) -> FindNextInScopeStageUseCase<G, S, F> {
        FindNextInScopeStageUseCase {
            stage_graph_dao,
            scope_grid_dao,
            state_file_dao,
        }
    }

    /// `after_slug` の次に来る in-scope の EXECUTE ステージの slug を引く。
    ///
    /// scope がグリッドに無い / `after_slug` がグラフに無い / それ以降に in-scope の EXECUTE が
    /// 無い、のいずれも「次は無い」= `Ok(None)` で返す (upstream の `null`)。
    ///
    /// # Errors
    ///
    /// リードモデルを引けない ([`ReadModelReadError`])。
    pub fn execute(
        &self,
        after_slug: &str,
        scope: &str,
    ) -> Result<Option<String>, ReadModelReadError> {
        let Some(actions) = self.scope_grid_dao.find(scope)? else {
            return Ok(None);
        };
        let graph = self.stage_graph_dao.find_all()?;
        let Some(start) = graph.iter().position(|stage| stage.slug() == after_slug) else {
            return Ok(None);
        };
        let state = self.state_file_dao.find()?;
        let overrides = state.as_deref().map(parse_stage_suffixes);
        let done = state.as_deref().map(parse_done_or_skipped);

        for stage in graph.iter().skip(start + 1) {
            let slug = stage.slug();
            if done
                .as_ref()
                .is_some_and(|set| set.iter().any(|s| s == slug))
            {
                continue;
            }
            let effective = overrides
                .as_ref()
                .and_then(|map| map.iter().find(|(s, _)| s == slug).map(|(_, a)| a.as_str()))
                .or_else(|| actions.action_of(slug));
            if effective == Some("EXECUTE") {
                return Ok(Some(slug.to_string()));
            }
        }
        Ok(None)
    }
}

/// 状態ファイルの各ステージ行から承認済み計画の `EXECUTE`/`SKIP` 上書きを読む
/// (upstream `parseStateStageSuffixes` — 正規表現 `^- \[[ xSR?-]\] (\S+)\s*—\s*(EXECUTE|SKIP)\b`)。
fn parse_stage_suffixes(content: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for line in content.lines() {
        let Some((slug, suffix)) = parse_checkbox_line(line) else {
            continue;
        };
        let keyword: String = suffix
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();
        if keyword == "EXECUTE" || keyword == "SKIP" {
            out.push((slug, keyword));
        }
    }
    out
}

/// 状態ファイルから済 (`x`) / skip (`S`) のステージ slug を読む
/// (upstream `parseCheckboxes` の `completed` / `skipped` 相当)。
fn parse_done_or_skipped(content: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in content.lines() {
        let Some(marker) = checkbox_marker(line) else {
            continue;
        };
        if marker != 'x' && marker != 'S' {
            continue;
        }
        if let Some((slug, _)) = parse_checkbox_line(line) {
            out.push(slug);
        }
    }
    out
}

/// `- [<marker>] ` で始まる行の marker 文字 (`[ xSR?-]` のいずれか)。
fn checkbox_marker(line: &str) -> Option<char> {
    let bytes = line.as_bytes();
    if bytes.first() != Some(&b'-')
        || bytes.get(1) != Some(&b' ')
        || bytes.get(2) != Some(&b'[')
        || bytes.get(4) != Some(&b']')
    {
        return None;
    }
    let marker = char::from(*bytes.get(3)?);
    matches!(marker, ' ' | 'x' | 'S' | 'R' | '?' | '-').then_some(marker)
}

/// `- [<marker>] <slug> — <suffix>` を (slug, suffix) に割る。
///
/// upstream の `(\S+)\s*—\s*(.*)` に合わせ、slug は空白で終わる最初の非空白トークン、
/// suffix はダッシュ以降を trim したものである。
fn parse_checkbox_line(line: &str) -> Option<(String, String)> {
    checkbox_marker(line)?;
    let after = line.get(5..)?.trim_start();
    let (before, suffix) = after.split_once('—')?;
    let slug = before.split_whitespace().next()?.to_string();
    Some((slug, suffix.trim().to_string()))
}
