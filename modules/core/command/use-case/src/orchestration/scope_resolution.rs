//! scope 解決ラダー — `state > --scope > positional > env > default` (01 §5.5 / 02 §2.1)。
//!
//! `next` の分岐 3b / 4 / 「解決不能」を支える純関数。無効な明示 `--scope` は **state が
//! 勝つ場合でも無条件に検証**される (02 §5 `:2880-2896`)。キーワード推論は語境界一致・
//! scope 名のアルファベット順スキャン・**5 語超のテキストでは抑止** (`:5586-5594`)。
//! デフォルトは `classic` (`aidlc-lib.ts:8896`)。

use core_command_domain::workflow_definition::WorkflowDefinition;

/// デフォルト scope (`export const DEFAULT_SCOPE = "classic";`)。
pub(crate) const DEFAULT_SCOPE: &str = "classic";

/// 解決の出所 (観測可能な分類 — 逐語文言はここから組む)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScopeSource {
    /// state ファイルの `Scope` (稼働中は常に勝つ)。
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
pub(crate) struct ResolvedScope {
    pub(crate) name: String,
    pub(crate) source: ScopeSource,
}

/// 解決の失敗 (材料のみ — 逐語文言は wording が組む)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ScopeResolutionError {
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

/// キーワード推論 (`inferScopeFromText`) — 語境界一致・アルファベット順で最初のマッチ・
/// 5 語超は抑止。
pub(crate) fn infer_scope_from_text(definition: &WorkflowDefinition, text: &str) -> Option<String> {
    let word_count = text.split_whitespace().count();
    if word_count > 5 {
        // "keyword + >5 words → likely a project description containing the keyword incidentally"
        return None;
    }
    let lowered = text.to_lowercase();
    let words: Vec<&str> = lowered
        .split(|c: char| !c.is_ascii_alphanumeric() && c != '-')
        .filter(|w| !w.is_empty())
        .collect();
    // BTreeMap なので valid_scopes はアルファベット順 — 最初のマッチが決定的に勝つ。
    for scope in definition.valid_scopes() {
        let Some(metadata) = definition.scope_metadata(scope) else {
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
            if hit {
                return Some(scope.to_string());
            }
        }
    }
    None
}

/// ラダー本体。`state_scope` は稼働中ワークフローの `Scope`、`explicit` は `--scope`、
/// `positional` は位置引数テキスト、`env` は `AWS_AIDLC_DEFAULT_SCOPE` の生値。
pub(crate) fn resolve_scope(
    definition: &WorkflowDefinition,
    state_scope: Option<&str>,
    explicit: Option<&str>,
    positional: Option<&str>,
    env: Option<&str>,
) -> Result<ResolvedScope, ScopeResolutionError> {
    // 分岐 3b — 無効な明示 --scope は state が勝つ場合でも無条件に検証される。
    if let Some(scope) = explicit
        && !definition.is_valid_scope(scope)
    {
        return Err(ScopeResolutionError::UnknownExplicit {
            scope: scope.to_string(),
        });
    }
    if let Some(scope) = state_scope {
        if definition.is_valid_scope(scope) {
            return Ok(ResolvedScope {
                name: scope.to_string(),
                source: ScopeSource::State,
            });
        }
        return Err(ScopeResolutionError::Unresolvable {
            scope: scope.to_string(),
        });
    }
    if let Some(scope) = explicit {
        return Ok(ResolvedScope {
            name: scope.to_string(),
            source: ScopeSource::Explicit,
        });
    }
    if let Some(text) = positional
        && let Some(scope) = infer_scope_from_text(definition, text)
    {
        return Ok(ResolvedScope {
            name: scope,
            source: ScopeSource::Inferred,
        });
    }
    if let Some(value) = env {
        if definition.is_valid_scope(value) {
            return Ok(ResolvedScope {
                name: value.to_string(),
                source: ScopeSource::Env,
            });
        }
        // 分岐 4 — 無効な env 値は逐語で拒否する。
        return Err(ScopeResolutionError::UnknownEnv {
            value: value.to_string(),
        });
    }
    Ok(ResolvedScope {
        name: DEFAULT_SCOPE.to_string(),
        source: ScopeSource::Default,
    })
}
