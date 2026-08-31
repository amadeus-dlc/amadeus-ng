//! scope 解決ラダー — `state > --scope > positional > env > default` (01 §5.5 / 02 §2.1)。
//!
//! どの観測が勝つか・キーワード推論の抑止規則・デフォルト scope は**判断ポリシー**であり、
//! 有効 scope とキーワードを持つ [`DefinitionView`] が所有する (自由関数ではなく所有する型の
//! 関連メソッド — `coding-rules/domain-services.md`)。無効な明示 `--scope` は **state が勝つ
//! 場合でも無条件に検証**される (02 §5 `:2880-2896`)。キーワード推論は語境界一致・scope 名の
//! アルファベット順スキャン・**5 語超のテキストでは抑止** (`:5586-5594`)。デフォルトは
//! `classic` (`aidlc-lib.ts:8896`)。

use crate::workflow_view::{DefinitionView, ScopeSlugView};

/// デフォルト scope (`export const DEFAULT_SCOPE = "classic";`)。
const DEFAULT_SCOPE: &str = "classic";

/// 解決の出所 (観測可能な分類 — 逐語文言は出す側が組む)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopeSource {
    /// リードモデルの `Scope` (稼働中は常に勝つ)。
    State,
    /// 明示 `--scope`。
    Explicit,
    /// 位置引数のキーワード推論。
    Inferred,
    /// `AWS_AIDLC_DEFAULT_SCOPE`。
    Env,
    /// デフォルト定数 (自由記述含む)。
    Default,
}

/// 解決結果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedScope {
    name: ScopeSlugView,
    source: ScopeSource,
}

impl ResolvedScope {
    /// 解決された scope 名。
    #[must_use]
    pub const fn name(&self) -> &ScopeSlugView {
        &self.name
    }

    /// 解決の出所。
    #[must_use]
    pub const fn source(&self) -> ScopeSource {
        self.source
    }
}

/// 解決の失敗 (材料のみ — 逐語文言は出す側が組む)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScopeResolutionError {
    /// 無効な明示 `--scope` (分岐 3b — 無条件検証)。
    UnknownExplicit {
        /// 拒否された scope 名。
        scope: String,
    },
    /// 無効な `AWS_AIDLC_DEFAULT_SCOPE` (分岐 4)。
    UnknownEnv {
        /// 拒否された環境変数の値。
        value: String,
    },
    /// ラダーを通しても解決できない (state 由来値が定義に無い等)。
    Unresolvable {
        /// 拒否された scope 名。
        scope: String,
    },
}

impl DefinitionView {
    /// キーワード推論 (`inferScopeFromText`) — 語境界一致・アルファベット順で最初のマッチ・
    /// 5 語超は抑止。
    #[must_use]
    pub fn infer_scope_from_text(&self, text: &str) -> Option<ScopeSlugView> {
        let word_count = text.split_whitespace().count();
        if word_count > 5 {
            // "keyword + >5 words → likely a project description containing the keyword
            // incidentally"
            return None;
        }
        let lowered = text.to_lowercase();
        let words: Vec<&str> = lowered
            .split(|c: char| !c.is_ascii_alphanumeric() && c != '-')
            .filter(|w| !w.is_empty())
            .collect();
        // BTreeMap なので valid_scopes はアルファベット順 — 最初のマッチが決定的に勝つ。
        for scope in self.valid_scopes() {
            let Some(metadata) = self.scope_metadata(scope) else {
                continue;
            };
            for keyword in metadata.keywords() {
                // キーワードは空白区切りのトークン列。語境界一致 = 単語列の連続一致。
                let tokens: Vec<String> = keyword
                    .to_lowercase()
                    .split_whitespace()
                    .map(str::to_string)
                    .collect();
                if tokens.is_empty() {
                    continue;
                }
                let hit = words
                    .windows(tokens.len())
                    .any(|window| window.iter().zip(&tokens).all(|(w, t)| w == t));
                if hit && let Ok(slug) = ScopeSlugView::parse(scope) {
                    return Some(slug);
                }
            }
        }
        None
    }

    /// ラダー本体。`state_scope` は稼働中ワークフローの `Scope`、`explicit` は `--scope`、
    /// `positional` は位置引数テキスト、`env` は `AWS_AIDLC_DEFAULT_SCOPE` の生値。
    ///
    /// # Errors
    ///
    /// 無効な明示 `--scope` (`UnknownExplicit`)・無効な env 値 (`UnknownEnv`)・state 由来値が
    /// 定義に無い等の解決不能 (`Unresolvable`) を拒否する。
    pub fn resolve_scope(
        &self,
        state_scope: Option<&str>,
        explicit: Option<&str>,
        positional: Option<&str>,
        env: Option<&str>,
    ) -> Result<ResolvedScope, ScopeResolutionError> {
        // 分岐 3b — 無効な明示 --scope は state が勝つ場合でも無条件に検証される。
        if let Some(scope) = explicit
            && !self.is_valid_scope(scope)
        {
            return Err(ScopeResolutionError::UnknownExplicit {
                scope: scope.to_string(),
            });
        }
        if let Some(scope) = state_scope {
            if self.is_valid_scope(scope)
                && let Ok(name) = ScopeSlugView::parse(scope)
            {
                return Ok(ResolvedScope {
                    name,
                    source: ScopeSource::State,
                });
            }
            return Err(ScopeResolutionError::Unresolvable {
                scope: scope.to_string(),
            });
        }
        if let Some(scope) = explicit {
            // 有効性は上で検証済み — 文法違反だけが残る (定義に載る scope 名は文法適合が前提)。
            return match ScopeSlugView::parse(scope) {
                Ok(name) => Ok(ResolvedScope {
                    name,
                    source: ScopeSource::Explicit,
                }),
                Err(_) => Err(ScopeResolutionError::UnknownExplicit {
                    scope: scope.to_string(),
                }),
            };
        }
        if let Some(text) = positional
            && let Some(name) = self.infer_scope_from_text(text)
        {
            return Ok(ResolvedScope {
                name,
                source: ScopeSource::Inferred,
            });
        }
        if let Some(value) = env {
            if self.is_valid_scope(value)
                && let Ok(name) = ScopeSlugView::parse(value)
            {
                return Ok(ResolvedScope {
                    name,
                    source: ScopeSource::Env,
                });
            }
            // 分岐 4 — 無効な env 値は逐語で拒否する。
            return Err(ScopeResolutionError::UnknownEnv {
                value: value.to_string(),
            });
        }
        if let Ok(name) = ScopeSlugView::parse(DEFAULT_SCOPE) {
            return Ok(ResolvedScope {
                name,
                source: ScopeSource::Default,
            });
        }
        // 静的に到達しない防御枝 — DEFAULT_SCOPE は文法適合の定数。
        Err(ScopeResolutionError::Unresolvable {
            scope: DEFAULT_SCOPE.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestration::test_fixtures::definition;

    #[test]
    fn the_state_scope_wins_when_a_workflow_is_running() {
        let held = definition(2);
        let resolved = held
            .resolve_scope(Some("classic"), None, Some("fix the bug"), Some("mvp"))
            .unwrap();
        assert_eq!(resolved.name().as_str(), "classic");
        assert_eq!(resolved.source(), ScopeSource::State);
    }

    #[test]
    fn an_invalid_explicit_scope_is_checked_unconditionally() {
        let held = definition(2);
        assert_eq!(
            held.resolve_scope(Some("classic"), Some("ghost"), None, None),
            Err(ScopeResolutionError::UnknownExplicit {
                scope: "ghost".to_string()
            })
        );
    }

    #[test]
    fn a_state_scope_missing_from_the_definition_is_unresolvable() {
        let held = definition(2);
        assert_eq!(
            held.resolve_scope(Some("ghost"), None, None, None),
            Err(ScopeResolutionError::Unresolvable {
                scope: "ghost".to_string()
            })
        );
    }

    #[test]
    fn the_ladder_falls_through_explicit_then_inferred_then_env_then_default() {
        let held = definition(2);
        assert_eq!(
            held.resolve_scope(None, Some("bugfix"), None, None)
                .unwrap()
                .source(),
            ScopeSource::Explicit
        );
        let inferred = held
            .resolve_scope(None, None, Some("fix login"), None)
            .unwrap();
        assert_eq!(inferred.name().as_str(), "bugfix");
        assert_eq!(inferred.source(), ScopeSource::Inferred);
        let env = held
            .resolve_scope(None, None, None, Some("bugfix"))
            .unwrap();
        assert_eq!(env.source(), ScopeSource::Env);
        let default = held.resolve_scope(None, None, None, None).unwrap();
        assert_eq!(default.name().as_str(), "classic");
        assert_eq!(default.source(), ScopeSource::Default);
    }

    #[test]
    fn an_invalid_env_value_is_rejected_verbatim() {
        let held = definition(2);
        assert_eq!(
            held.resolve_scope(None, None, None, Some("ghost")),
            Err(ScopeResolutionError::UnknownEnv {
                value: "ghost".to_string()
            })
        );
    }

    #[test]
    fn the_keyword_inference_is_suppressed_beyond_five_words() {
        let held = definition(2);
        assert!(held.infer_scope_from_text("fix").is_some());
        assert!(
            held.infer_scope_from_text("we should fix the login crash today")
                .is_none(),
            "5 語超は偶発一致とみなして抑止する"
        );
        assert!(held.infer_scope_from_text("prefix").is_none(), "語境界一致");
    }
}
