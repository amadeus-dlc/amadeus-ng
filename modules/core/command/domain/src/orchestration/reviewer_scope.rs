//! `ReviewerScope` — per-unit レビュアーの読み取り範囲 (`stage-protocol-reviewer.md` §12a)。
use super::reviewer_scope_paths::{
    canonical_suffix, contains_path, exempt_suffix_of, fold, has_wildcard,
    is_construction_component, normalize_resolved, normalized_comps, resolve_lexical,
    resolve_path_strings, unique_strings,
};
use super::{
    InspectedCommand, InspectedCommandStep, InspectedTool, ReviewerDispatch, ReviewerScopeBlock,
    ReviewerScopeCandidate, ReviewerScopeCandidates, ReviewerScopeVerdict, ScopeToken,
};
use core_infrastructure::collections::FirstClassCollection;
use std::collections::BTreeSet;

/// 候補の読み方。掃く根は「その下すべて」を触るので、`construction/` の**上**にあるだけで
/// 越境になる。開く対象はその経路自身しか触らない。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Reach {
    Target,
    SearchRoot,
}

/// 差し向け 1 件から組んだ、判定に使える形の読み取り範囲。
///
/// 規則は 1 つ — `construction/` の直後の成分が現在の Unit なら通し、ワイルドカードか
/// 不在 (掃く根) なら拒否し、具体的な兄弟なら許可経路と**完全一致**したときだけ通す。
/// 完全一致にするのは、許可されたファイルの親ディレクトリを覗くのは依然として兄弟の
/// 閲覧だからである。
///
/// `construction/` の外は常に許可する。判定は純粋で、時計も台帳も見ない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewerScope {
    dispatch: ReviewerDispatch,
    unit_folded: String,
    exempt_suffixes: BTreeSet<String>,
    exempt_paths: BTreeSet<String>,
    construction_root: Option<String>,
    bases: Vec<String>,
}
impl ReviewerScope {
    /// 差し向けと経路の基点から範囲を組む (**この型の唯一の構築経路**)。
    ///
    /// `record_root` は差し向け記録が置かれたディレクトリ、`cwd` はハーネスが渡す
    /// 作業ディレクトリである。どちらも無ければ字句だけの照合になる。
    #[must_use]
    pub fn of(
        dispatch: &ReviewerDispatch,
        record_root: Option<&str>,
        cwd: Option<&str>,
    ) -> ReviewerScope {
        let record_root = record_root.map(|root| resolve_lexical("", root));
        let construction_root = record_root
            .as_ref()
            .map(|root| resolve_lexical(root, "construction"));
        let unit_root = construction_root
            .as_ref()
            .map(|root| resolve_lexical(root, dispatch.unit().as_str()));
        let cwd = cwd.map(|dir| resolve_lexical("", dir));
        let bases = unique_strings(vec![
            cwd.unwrap_or_default(),
            record_root.unwrap_or_default(),
            unit_root.unwrap_or_default(),
        ]);
        let mut exempt_suffixes = BTreeSet::new();
        let mut exempt_paths = BTreeSet::new();
        dispatch.exempt().fold_left((), |(), entry| {
            if let Some(suffix) = exempt_suffix_of(entry.as_str()) {
                exempt_suffixes.insert(suffix);
            }
            for path in normalize_resolved(entry.as_str(), &bases) {
                exempt_paths.insert(path);
            }
        });
        ReviewerScope {
            unit_folded: fold(dispatch.unit().as_str()),
            exempt_suffixes,
            exempt_paths,
            construction_root: construction_root
                .as_deref()
                .map(super::reviewer_scope_paths::normalize_for_compare),
            bases,
            dispatch: dispatch.clone(),
        }
    }

    /// 呼出し 1 件を判定する。
    ///
    /// 最初に当たった越境で打ち切り、その綴りを拒否の材料にする (upstream と同じ順序)。
    #[must_use]
    pub fn judge(
        &self,
        tool: InspectedTool,
        candidates: &ReviewerScopeCandidates,
    ) -> ReviewerScopeVerdict {
        self.offending_token(tool, candidates)
            .and_then(|text| ScopeToken::parse(&text).ok())
            .map_or(ReviewerScopeVerdict::Allowed, |target| {
                ReviewerScopeVerdict::Blocked(ReviewerScopeBlock::new(
                    tool,
                    target,
                    self.dispatch.stage().clone(),
                    self.dispatch.unit().clone(),
                ))
            })
    }

    /// 越境した綴りを 1 つ返す。範囲の内側なら `None`。
    ///
    /// 空の綴りが越境することはない (空は基点自身へ解決し、`construction/` の下に来ない)。
    /// 仮に来ても [`ScopeToken`] を鋳造できないので呼出側は許可へ倒れる — フック全体の
    /// fail-open と同じ向きである。
    fn offending_token(
        &self,
        tool: InspectedTool,
        candidates: &ReviewerScopeCandidates,
    ) -> Option<String> {
        let mut glob_constrained = false;
        let mut saw_glob = false;
        let mut saw_search_root = false;
        for index in 0..candidates.len() {
            let found = match candidates.at(index)? {
                ReviewerScopeCandidate::Target(text) => {
                    self.judge_path_access(text.as_str(), Reach::Target, &self.bases)
                }
                ReviewerScopeCandidate::SearchRoot(text) => {
                    saw_search_root = true;
                    self.judge_path_access(text.as_str(), Reach::SearchRoot, &self.bases)
                }
                ReviewerScopeCandidate::Glob(text) => {
                    saw_glob = true;
                    let found = self.judge_path_access(text.as_str(), Reach::Target, &self.bases);
                    glob_constrained =
                        glob_constrained || self.pattern_limits_to_current_unit(text.as_str());
                    found
                }
                ReviewerScopeCandidate::Command(command) => self.judge_command(command),
            };
            if found.is_some() {
                return found;
            }
        }
        // 経路を名乗らない Grep は作業ディレクトリから再帰する。Glob も同じだが、
        // パターン自身が現在の Unit へ絞っているときだけ免れる。
        let sweeps_from_cwd = match tool {
            InspectedTool::Grep => !saw_search_root && !glob_constrained,
            InspectedTool::Glob => !saw_search_root && saw_glob && !glob_constrained,
            _ => false,
        };
        if sweeps_from_cwd {
            return self.judge_path_access(".", Reach::SearchRoot, &self.bases);
        }
        None
    }

    /// 字句のまま見て、解決してからも見る (upstream `judgePathAccess`)。
    fn judge_path_access(&self, text: &str, reach: Reach, bases: &[String]) -> Option<String> {
        self.judge_lexical_path(text)
            .or_else(|| self.judge_resolved_path(text, reach, bases))
    }

    /// 綴りに現れる `construction/` の出現位置ごとに判定する。
    fn judge_lexical_path(&self, text: &str) -> Option<String> {
        let comps = normalized_comps(text);
        for index in 0..comps.len() {
            let names_construction = comps
                .get(index)
                .is_some_and(|component| is_construction_component(component, true));
            if !names_construction {
                continue;
            }
            if comps
                .get(index..)
                .is_some_and(|suffix| self.crosses_at(suffix))
            {
                return Some(text.to_string());
            }
        }
        None
    }

    /// 基点に対して解決した経路が Unit の外へ届くか。
    fn judge_resolved_path(&self, text: &str, reach: Reach, bases: &[String]) -> Option<String> {
        let construction_root = self.construction_root.as_ref()?;
        for path in normalize_resolved(text, bases) {
            if contains_path(construction_root, &path) {
                let mut suffix = vec!["construction".to_string()];
                suffix.extend(rest_under(construction_root, &path));
                let exact_exempt = self.exempt_paths.contains(&path)
                    || self.exempt_suffixes.contains(&canonical_suffix(&suffix));
                if exact_exempt {
                    continue;
                }
                if self.crosses_at(&suffix) {
                    return Some(text.to_string());
                }
            }
            // `construction/` の**上**を掃く根は、綴りに construction が出なくても
            // 兄弟をすべて掃く (`rg X .` など)。
            if reach == Reach::SearchRoot && contains_path(&path, construction_root) {
                return Some(text.to_string());
            }
        }
        None
    }

    /// `construction/` の 1 つの出現が兄弟へ届くか (upstream `judgeOccurrence`)。
    fn crosses_at(&self, suffix: &[String]) -> bool {
        let Some(segment) = suffix.get(1) else {
            return true; // `construction/` そのものは全兄弟を掃く根である
        };
        if segment.is_empty() || has_wildcard(segment) {
            return true;
        }
        if fold(segment) == self.unit_folded {
            return false;
        }
        !self.exempt_suffixes.contains(&canonical_suffix(suffix))
    }

    /// パターン自身が現在の Unit (か許可経路) へ絞っているか。
    fn pattern_limits_to_current_unit(&self, text: &str) -> bool {
        let comps = normalized_comps(text);
        let mut saw_construction = false;
        for index in 0..comps.len() {
            let names_construction = comps
                .get(index)
                .is_some_and(|component| is_construction_component(component, true));
            if !names_construction {
                continue;
            }
            saw_construction = true;
            let Some(suffix) = comps.get(index..) else {
                return false;
            };
            let Some(segment) = suffix.get(1) else {
                return false;
            };
            if has_wildcard(segment) {
                return false;
            }
            if fold(segment) != self.unit_folded
                && !self.exempt_suffixes.contains(&canonical_suffix(suffix))
            {
                return false;
            }
        }
        saw_construction
    }

    /// シェルコマンド 1 本を実行位置ごとに読む (`cd` は続く実行位置の基点を動かす)。
    fn judge_command(&self, command: &InspectedCommand) -> Option<String> {
        let mut bases = self.bases.clone();
        for index in 0..command.len() {
            let step = command.at(index)?;
            let Some(name) = step.at(0) else {
                continue;
            };
            let program = command_basename(name.as_str());
            if program == "cd" {
                let Some(operand) = first_operand(step) else {
                    continue;
                };
                if operand == "-" {
                    continue;
                }
                if let Some(found) = self.judge_path_access(operand, Reach::SearchRoot, &bases) {
                    return Some(found);
                }
                bases = unique_strings(resolve_path_strings(operand, &bases));
                continue;
            }
            let found = match program.as_str() {
                "grep" | "egrep" | "fgrep" => self.judge_grep_like(step, &bases),
                "rg" | "ripgrep" => self.judge_ripgrep(step, &bases),
                "find" => self.judge_find(step, &bases),
                "ls" => self.judge_simple_file_command(step, Reach::SearchRoot, &bases),
                "cat" | "less" | "more" | "head" | "tail" => {
                    self.judge_simple_file_command(step, Reach::Target, &bases)
                }
                _ => self.judge_generic_command(step, &bases),
            };
            if found.is_some() {
                return found;
            }
        }
        None
    }

    /// `grep` 族 — 最初の被演算子は内容パターン、以降が探索の根。
    fn judge_grep_like(&self, step: &InspectedCommandStep, bases: &[String]) -> Option<String> {
        let mut pattern_seen = false;
        let mut root_seen = false;
        let mut index = 1;
        while index < step.len() {
            let word = step.at(index)?.as_str();
            if word == "-e" || word == "--regexp" {
                pattern_seen = true;
                index += 2;
                continue;
            }
            if word == "-f" || word == "--file" {
                index += 2;
                continue;
            }
            index += 1;
            if is_option(word) {
                continue;
            }
            if !pattern_seen {
                pattern_seen = true;
                continue;
            }
            root_seen = true;
            if let Some(found) = self.judge_path_access(word, Reach::SearchRoot, bases) {
                return Some(found);
            }
        }
        if root_seen {
            None
        } else {
            self.judge_path_access(".", Reach::SearchRoot, bases)
        }
    }

    /// `rg` — `--glob` は経路の形をした絞り込みなので、根の推定にも効く。
    fn judge_ripgrep(&self, step: &InspectedCommandStep, bases: &[String]) -> Option<String> {
        let mut pattern_seen = false;
        let mut root_seen = false;
        let mut constrained = false;
        let mut index = 1;
        while index < step.len() {
            let word = step.at(index)?.as_str();
            let glob = if word == "-g" || word == "--glob" {
                index += 1;
                Some(step.at(index).map_or("", ScopeToken::as_str).to_string())
            } else {
                word.strip_prefix("--glob=").map(str::to_string)
            };
            if let Some(glob) = glob {
                index += 1;
                if glob.is_empty() {
                    continue;
                }
                if let Some(found) = self.judge_path_access(&glob, Reach::Target, bases) {
                    return Some(found);
                }
                constrained = constrained || self.pattern_limits_to_current_unit(&glob);
                continue;
            }
            index += 1;
            if is_option(word) {
                continue;
            }
            if !pattern_seen {
                pattern_seen = true;
                continue;
            }
            root_seen = true;
            if let Some(found) = self.judge_path_access(word, Reach::SearchRoot, bases) {
                return Some(found);
            }
        }
        if root_seen || constrained {
            None
        } else {
            self.judge_path_access(".", Reach::SearchRoot, bases)
        }
    }

    /// `find` — 述語が始まるまでの被演算子が走査の起点。
    fn judge_find(&self, step: &InspectedCommandStep, bases: &[String]) -> Option<String> {
        let mut root_seen = false;
        for index in 1..step.len() {
            let word = step.at(index)?.as_str();
            if is_option(word) || word == "!" || word == "(" || word == ")" {
                break;
            }
            root_seen = true;
            if let Some(found) = self.judge_path_access(word, Reach::SearchRoot, bases) {
                return Some(found);
            }
        }
        if root_seen {
            None
        } else {
            self.judge_path_access(".", Reach::SearchRoot, bases)
        }
    }

    /// `ls` / `cat` 族 — スイッチ以外の被演算子すべてが対象。
    fn judge_simple_file_command(
        &self,
        step: &InspectedCommandStep,
        reach: Reach,
        bases: &[String],
    ) -> Option<String> {
        let mut operand_seen = false;
        for index in 1..step.len() {
            let word = step.at(index)?.as_str();
            if is_option(word) {
                continue;
            }
            operand_seen = true;
            if let Some(found) = self.judge_path_access(word, reach, bases) {
                return Some(found);
            }
        }
        if !operand_seen && reach == Reach::SearchRoot {
            return self.judge_path_access(".", Reach::SearchRoot, bases);
        }
        None
    }

    /// 知らないコマンド — 経路の形をした語だけを対象として見る。
    fn judge_generic_command(
        &self,
        step: &InspectedCommandStep,
        bases: &[String],
    ) -> Option<String> {
        for index in 1..step.len() {
            let word = step.at(index)?.as_str();
            if is_option(word) || !is_pathish(word) {
                continue;
            }
            if let Some(found) = self.judge_path_access(word, Reach::Target, bases) {
                return Some(found);
            }
        }
        None
    }
}

/// `construction/` 直下から数えた残りの成分。
fn rest_under(construction_root: &str, path: &str) -> Vec<String> {
    if path == construction_root {
        return Vec::new();
    }
    path.strip_prefix(construction_root)
        .and_then(|rest| rest.strip_prefix('/'))
        .map(|rest| rest.split('/').map(str::to_string).collect())
        .unwrap_or_default()
}

/// 実行ファイル名の葉を畳んだもの (upstream `commandBasename`)。
fn command_basename(word: &str) -> String {
    let comps = normalized_comps(word);
    fold(comps.last().map_or(word, String::as_str))
}

/// スイッチか (upstream `isOption` — 単体の `-` はスイッチではない)。
fn is_option(word: &str) -> bool {
    word.chars().count() > 1 && word.starts_with('-')
}

/// スイッチでない最初の被演算子 (upstream `firstOperand`)。
fn first_operand(step: &InspectedCommandStep) -> Option<&str> {
    (1..step.len())
        .filter_map(|index| step.at(index))
        .map(ScopeToken::as_str)
        .find(|word| !is_option(word))
}

/// 経路の形をした語か (upstream `isPathish`)。
fn is_pathish(word: &str) -> bool {
    word == "."
        || word == ".."
        || word.contains('/')
        || has_wildcard(word)
        || is_construction_component(word, true)
}
