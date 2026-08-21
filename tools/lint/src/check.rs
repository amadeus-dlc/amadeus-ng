//! ルール本体 — 「ソース文字列 + 仮想パス → 所見リスト」の純粋関数として実装する。
//!
//! この形にしておくことで、各ルールの検査力 (= 実際に赤例を落とせること) を I/O 抜きの
//! 単体テストで固定できる。検出例の無いルールは追加しない (このリポジトリの Quint ゲートと
//! 同じ DoD)。
//!
//! 検出の哲学: getter の存在そのものは咎めない。**他オブジェクトから状態を抜き出して
//! 所有者の判断を呼出側で代行する**濫用パターンだけを検出する。

use std::collections::BTreeSet;

use proc_macro2::{Ident, TokenStream, TokenTree};
use syn::spanned::Spanned;
use syn::visit::Visit;

/// R1: 分類語彙 (`CheckboxState` の変種集合) を所有者の外で再実装している。
pub(crate) const RULE_CHECKBOX_VOCABULARY: &str = "checkbox-vocabulary";
/// R2: reap 適格判定の境界規約を interface-adapter で再実装している。
pub(crate) const RULE_REAP_DECISION_LOCALITY: &str = "reap-decision-locality";

/// R1 の語彙所有者。この 1 ファイルだけは変種を列挙してよい (分類述語の実装本体)。
const CHECKBOX_OWNER: &str = "modules/core/domain/src/workspace/checkbox.rs";
/// R2 の適用範囲。
const INTERFACE_ADAPTER_ROOT: &str = "modules/core/interface-adapter/";
/// R2 が監視する「所有者の内部状態」を表す識別子。
const REAP_IDENTS: [&str; 2] = ["stale_ms", "held_ticks"];

const CHECKBOX_HELP: &str = "CheckboxState の述語 (is_in_flight / is_finished / is_active) を使う。\
集約が所有する遷移前提集合 (I7 / I13 等) であれば \
`// amadeus-lint: allow(checkbox-vocabulary) — 理由` で理由を明示する";
const REAP_HELP: &str = "reap 適格判定は core_domain::workspace::lock_protocol::reap_eligible に\
委譲する (境界規約 `>` の単一実装)";

/// 1 件の所見。`line` は 1 始まり。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Finding {
    pub(crate) rule: &'static str,
    pub(crate) line: usize,
    pub(crate) message: String,
    pub(crate) help: &'static str,
}

/// 1 ファイル分の検査。`path` はリポジトリルートからの相対パス (区切りは `/`)。
///
/// # Errors
///
/// `source` が Rust ファイルとして構文解析できないとき `syn::Error` を返す。
pub(crate) fn check_source(path: &str, source: &str) -> Result<Vec<Finding>, syn::Error> {
    let path = path.replace('\\', "/");
    // テストコードは全ルールの対象外。Tell, Don't Ask はプロダクトコードの設計規律であり、
    // テストは意図的に内部状態を覗く (fixture 構築・網羅性検査) 場所だから。
    if is_test_path(&path) {
        return Ok(Vec::new());
    }

    let file = syn::parse_file(source)?;
    let mut visitor = Visitor {
        checkbox_rule: !path.ends_with(CHECKBOX_OWNER),
        reap_rule: path.contains(INTERFACE_ADAPTER_ROOT),
        findings: Vec::new(),
    };
    visitor.visit_file(&file);

    let lines: Vec<&str> = source.lines().collect();
    let mut findings: Vec<Finding> = visitor
        .findings
        .into_iter()
        .filter(|finding| !is_suppressed(&lines, finding))
        .collect();
    findings.sort_by_key(|finding| (finding.line, finding.rule));
    Ok(findings)
}

/// `/tests/` を含むパスは統合テスト。
fn is_test_path(path: &str) -> bool {
    path.contains("/tests/") || path.starts_with("tests/")
}

/// 所見の開始行の直前行が `// amadeus-lint: allow(<rule-id>)` で始まれば抑制する。
/// rule-id が一致しない allow は抑制しない (別ルールの許可で塗り潰さないため)。
fn is_suppressed(lines: &[&str], finding: &Finding) -> bool {
    if finding.line < 2 {
        return false;
    }
    let marker = format!("// amadeus-lint: allow({})", finding.rule);
    lines
        .get(finding.line - 2)
        .is_some_and(|prev| prev.trim().starts_with(&marker))
}

struct Visitor {
    checkbox_rule: bool,
    reap_rule: bool,
    findings: Vec<Finding>,
}

impl Visitor {
    fn push_checkbox(&mut self, line: usize, variants: &BTreeSet<String>) {
        let listed: Vec<&str> = variants.iter().map(String::as_str).collect();
        self.findings.push(Finding {
            rule: RULE_CHECKBOX_VOCABULARY,
            line,
            message: format!(
                "CheckboxState の変種を呼出側で列挙している ({}) — 分類語彙の再実装 (Tell, Don't Ask 違反)",
                listed.join(" | ")
            ),
            help: CHECKBOX_HELP,
        });
    }

    fn push_reap(&mut self, line: usize) {
        self.findings.push(Finding {
            rule: RULE_REAP_DECISION_LOCALITY,
            line,
            message: "ロック所有者の内部状態 (stale_ms / held_ticks) を取り出して \
reap 適格判定を adapter 側で再実装している (Tell, Don't Ask 違反)"
                .to_string(),
            help: REAP_HELP,
        });
    }
}

impl<'ast> Visit<'ast> for Visitor {
    fn visit_item(&mut self, node: &'ast syn::Item) {
        if has_cfg_test(item_attrs(node)) {
            return;
        }
        syn::visit::visit_item(self, node);
    }

    fn visit_impl_item(&mut self, node: &'ast syn::ImplItem) {
        if has_cfg_test(impl_item_attrs(node)) {
            return;
        }
        syn::visit::visit_impl_item(self, node);
    }

    /// R1 (a): `matches!` の引数トークン列に `CheckboxState::<Variant>` が 2 種類以上。
    fn visit_expr_macro(&mut self, node: &'ast syn::ExprMacro) {
        if self.checkbox_rule && path_ends_with_ident(&node.mac.path, "matches") {
            let mut variants = BTreeSet::new();
            collect_checkbox_variants_in_tokens(&node.mac.tokens, &mut variants);
            if variants.len() >= 2 {
                let line = node.span().start().line;
                self.push_checkbox(line, &variants);
            }
        }
        syn::visit::visit_expr_macro(self, node);
    }

    /// R1 (b): `match` 式の腕**パターン**に `CheckboxState` の変種が 2 種類以上。
    fn visit_expr_match(&mut self, node: &'ast syn::ExprMatch) {
        if self.checkbox_rule {
            let mut collector = PatternVariants {
                variants: BTreeSet::new(),
            };
            for arm in &node.arms {
                collector.visit_pat(&arm.pat);
            }
            if collector.variants.len() >= 2 {
                let line = node.match_token.span().start().line;
                self.push_checkbox(line, &collector.variants);
            }
        }
        syn::visit::visit_expr_match(self, node);
    }

    /// R2: 比較二項式のオペランドが `stale_ms` / `held_ticks` に触れている。
    fn visit_expr_binary(&mut self, node: &'ast syn::ExprBinary) {
        if self.reap_rule
            && is_ordering_op(&node.op)
            && (mentions_reap_state(&node.left) || mentions_reap_state(&node.right))
        {
            let line = node.span().start().line;
            self.push_reap(line);
        }
        syn::visit::visit_expr_binary(self, node);
    }
}

/// パターン中の `CheckboxState::<Variant>` を集める補助 visitor。
struct PatternVariants {
    variants: BTreeSet<String>,
}

impl<'ast> Visit<'ast> for PatternVariants {
    fn visit_path(&mut self, node: &'ast syn::Path) {
        if let Some(variant) = checkbox_variant_of_path(node) {
            self.variants.insert(variant);
        }
        syn::visit::visit_path(self, node);
    }
}

/// 特定の識別子が式中に現れるかを調べる補助 visitor (`self.stale_ms` の
/// `Member::Named` も `visit_member` 経由で到達する)。
struct IdentSearch<'a> {
    wanted: &'a [&'a str],
    hit: bool,
}

impl<'ast> Visit<'ast> for IdentSearch<'_> {
    fn visit_ident(&mut self, node: &'ast Ident) {
        if self.wanted.iter().any(|name| node == *name) {
            self.hit = true;
        }
    }
}

fn mentions_reap_state(expr: &syn::Expr) -> bool {
    let mut search = IdentSearch {
        wanted: &REAP_IDENTS,
        hit: false,
    };
    search.visit_expr(expr);
    search.hit
}

const fn is_ordering_op(op: &syn::BinOp) -> bool {
    matches!(
        op,
        syn::BinOp::Lt(_) | syn::BinOp::Le(_) | syn::BinOp::Gt(_) | syn::BinOp::Ge(_)
    )
}

fn path_ends_with_ident(path: &syn::Path, name: &str) -> bool {
    path.segments.last().is_some_and(|seg| seg.ident == name)
}

/// `..::CheckboxState::<Variant>` の `<Variant>` を取り出す。
fn checkbox_variant_of_path(path: &syn::Path) -> Option<String> {
    let idents: Vec<&Ident> = path.segments.iter().map(|seg| &seg.ident).collect();
    idents
        .windows(2)
        .find(|pair| pair[0] == "CheckboxState")
        .map(|pair| pair[1].to_string())
}

/// マクロ引数は未解析トークンなので、`CheckboxState :: <Ident>` の並びを直接走査する。
fn collect_checkbox_variants_in_tokens(tokens: &TokenStream, out: &mut BTreeSet<String>) {
    let trees: Vec<TokenTree> = tokens.clone().into_iter().collect();
    for (index, tree) in trees.iter().enumerate() {
        if let TokenTree::Group(group) = tree {
            collect_checkbox_variants_in_tokens(&group.stream(), out);
        }
        if !matches!(tree, TokenTree::Ident(ident) if ident == "CheckboxState") {
            continue;
        }
        let is_colon = |offset: usize| matches!(trees.get(index + offset), Some(TokenTree::Punct(p)) if p.as_char() == ':');
        if !(is_colon(1) && is_colon(2)) {
            continue;
        }
        if let Some(TokenTree::Ident(variant)) = trees.get(index + 3) {
            out.insert(variant.to_string());
        }
    }
}

/// `#[cfg(test)]` (`#[cfg(all(test, ..))]` を含む) が付いているか。
/// 属性のトークン列に裸の識別子 `test` が現れるかで判定する
/// (`cfg(feature = "test")` の `"test"` は Literal なので誤検出しない)。
fn has_cfg_test(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| {
        if !attr.path().is_ident("cfg") {
            return false;
        }
        match &attr.meta {
            syn::Meta::List(list) => token_stream_has_ident(&list.tokens, "test"),
            _ => false,
        }
    })
}

fn token_stream_has_ident(tokens: &TokenStream, name: &str) -> bool {
    tokens.clone().into_iter().any(|tree| match tree {
        TokenTree::Ident(ident) => ident == name,
        TokenTree::Group(group) => token_stream_has_ident(&group.stream(), name),
        _ => false,
    })
}

/// `syn::Item` は `#[non_exhaustive]` なので、属性を持つ既知の変種だけを列挙する。
fn item_attrs(item: &syn::Item) -> &[syn::Attribute] {
    match item {
        syn::Item::Const(i) => &i.attrs,
        syn::Item::Enum(i) => &i.attrs,
        syn::Item::ExternCrate(i) => &i.attrs,
        syn::Item::Fn(i) => &i.attrs,
        syn::Item::ForeignMod(i) => &i.attrs,
        syn::Item::Impl(i) => &i.attrs,
        syn::Item::Macro(i) => &i.attrs,
        syn::Item::Mod(i) => &i.attrs,
        syn::Item::Static(i) => &i.attrs,
        syn::Item::Struct(i) => &i.attrs,
        syn::Item::Trait(i) => &i.attrs,
        syn::Item::TraitAlias(i) => &i.attrs,
        syn::Item::Type(i) => &i.attrs,
        syn::Item::Union(i) => &i.attrs,
        syn::Item::Use(i) => &i.attrs,
        _ => &[],
    }
}

fn impl_item_attrs(item: &syn::ImplItem) -> &[syn::Attribute] {
    match item {
        syn::ImplItem::Const(i) => &i.attrs,
        syn::ImplItem::Fn(i) => &i.attrs,
        syn::ImplItem::Type(i) => &i.attrs,
        syn::ImplItem::Macro(i) => &i.attrs,
        _ => &[],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOMAIN_PATH: &str = "modules/core/domain/src/orchestration/workflow_execution.rs";
    const OWNER_PATH: &str = CHECKBOX_OWNER;
    const ADAPTER_PATH: &str = "modules/core/interface-adapter/src/workspace/fs_workspace_lock.rs";
    const ADAPTER_TEST_PATH: &str =
        "modules/core/interface-adapter/tests/fs_workspace_lock_test.rs";

    fn check(path: &str, source: &str) -> Vec<Finding> {
        check_source(path, source).expect("テストのソースは構文解析できること")
    }

    fn rules(findings: &[Finding]) -> Vec<&'static str> {
        findings.iter().map(|f| f.rule).collect()
    }

    // ---- R1 赤例 (修正前に実在した形) ------------------------------------

    #[test]
    fn r1_detects_matches_macro_enumerating_checkbox_variants() {
        // 実際に workflow_execution.rs にあった 4 変種の in-flight 判定。
        let source = r#"
fn report(cb: CheckboxState) -> bool {
    matches!(
        cb,
        CheckboxState::Pending
            | CheckboxState::InProgress
            | CheckboxState::AwaitingApproval
            | CheckboxState::Revising
    )
}
"#;
        let findings = check(DOMAIN_PATH, source);
        assert_eq!(rules(&findings), vec![RULE_CHECKBOX_VOCABULARY]);
        assert_eq!(findings[0].line, 3, "matches! の開始行を指すこと");
        assert!(
            findings[0].message.contains("AwaitingApproval"),
            "列挙した変種を message に含めること: {}",
            findings[0].message
        );
    }

    #[test]
    fn r1_detects_match_expression_enumerating_checkbox_variants_in_arm_patterns() {
        let source = r#"
fn classify(cb: CheckboxState) -> u8 {
    match cb {
        CheckboxState::Completed | CheckboxState::Skipped => 0,
        _ => 1,
    }
}
"#;
        let findings = check(DOMAIN_PATH, source);
        assert_eq!(rules(&findings), vec![RULE_CHECKBOX_VOCABULARY]);
        assert_eq!(findings[0].line, 3, "match キーワードの行を指すこと");
    }

    #[test]
    fn r1_detects_variants_nested_in_arm_patterns() {
        // Option 越しの列挙も分類語彙の再実装。
        let source = r#"
fn finished(cb: Option<CheckboxState>) -> bool {
    match cb {
        Some(CheckboxState::Completed | CheckboxState::Skipped) => true,
        _ => false,
    }
}
"#;
        assert_eq!(
            rules(&check(DOMAIN_PATH, source)),
            vec![RULE_CHECKBOX_VOCABULARY]
        );
    }

    // ---- R1 緑例 ---------------------------------------------------------

    #[test]
    fn r1_allows_delegating_to_the_owner_predicate() {
        let source = r#"
fn report(cb: CheckboxState) -> bool {
    cb.is_in_flight()
}
"#;
        assert!(check(DOMAIN_PATH, source).is_empty());
    }

    #[test]
    fn r1_allows_single_variant_comparison() {
        // 特定の問い (「いま pending か」) は分類語彙の再実装ではない。
        let source = r#"
fn started(cb: CheckboxState) -> bool {
    cb != CheckboxState::Pending
}

fn pending(cb: CheckboxState) -> bool {
    cb == CheckboxState::Pending
}
"#;
        assert!(check(DOMAIN_PATH, source).is_empty());
    }

    #[test]
    fn r1_allows_single_variant_matches_macro() {
        let source = r#"
fn pending(cb: CheckboxState) -> bool {
    matches!(cb, CheckboxState::Pending)
}
"#;
        assert!(check(DOMAIN_PATH, source).is_empty());
    }

    #[test]
    fn r1_ignores_code_inside_cfg_test_modules() {
        let source = r#"
#[cfg(test)]
mod tests {
    fn red(cb: CheckboxState) -> bool {
        matches!(cb, CheckboxState::InProgress | CheckboxState::AwaitingApproval)
    }
}
"#;
        assert!(check(DOMAIN_PATH, source).is_empty());
    }

    #[test]
    fn r1_ignores_nested_items_inside_cfg_test_modules() {
        let source = r#"
#[cfg(test)]
mod tests {
    mod inner {
        impl Thing {
            fn red(cb: CheckboxState) -> bool {
                matches!(cb, CheckboxState::InProgress | CheckboxState::Revising)
            }
        }
    }
}
"#;
        assert!(check(DOMAIN_PATH, source).is_empty());
    }

    #[test]
    fn r1_ignores_integration_test_paths() {
        let source = r#"
fn red(cb: CheckboxState) -> bool {
    matches!(cb, CheckboxState::InProgress | CheckboxState::AwaitingApproval)
}
"#;
        assert!(
            check(
                "modules/core/domain/tests/engine_loop_conformance.rs",
                source
            )
            .is_empty()
        );
    }

    #[test]
    fn r1_is_suppressed_by_a_matching_allow_comment() {
        let source = r#"
fn report(cb: CheckboxState) -> bool {
    // amadeus-lint: allow(checkbox-vocabulary) — I7 ゲート前提 (集約所有の遷移前提集合)
    matches!(cb, CheckboxState::InProgress | CheckboxState::AwaitingApproval)
}
"#;
        assert!(check(DOMAIN_PATH, source).is_empty());
    }

    #[test]
    fn allow_comment_for_another_rule_does_not_suppress() {
        let source = r#"
fn report(cb: CheckboxState) -> bool {
    // amadeus-lint: allow(other-rule) — 無関係な許可
    matches!(cb, CheckboxState::InProgress | CheckboxState::AwaitingApproval)
}
"#;
        assert_eq!(
            rules(&check(DOMAIN_PATH, source)),
            vec![RULE_CHECKBOX_VOCABULARY],
            "rule-id が一致しない allow は抑制しない"
        );
    }

    #[test]
    fn allow_comment_two_lines_above_does_not_suppress() {
        let source = r#"
fn report(cb: CheckboxState) -> bool {
    // amadeus-lint: allow(checkbox-vocabulary) — 位置がずれている

    matches!(cb, CheckboxState::InProgress | CheckboxState::AwaitingApproval)
}
"#;
        assert_eq!(
            rules(&check(DOMAIN_PATH, source)),
            vec![RULE_CHECKBOX_VOCABULARY]
        );
    }

    // ---- R1 境界 (語彙の所有者) -------------------------------------------

    #[test]
    fn r1_does_not_fire_inside_the_vocabulary_owner() {
        let source = r#"
impl CheckboxState {
    pub const fn is_finished(self) -> bool {
        matches!(self, CheckboxState::Completed | CheckboxState::Skipped)
    }

    pub const fn marker(self) -> char {
        match self {
            CheckboxState::Pending => ' ',
            CheckboxState::Completed => 'x',
            _ => '?',
        }
    }
}
"#;
        assert!(
            check(OWNER_PATH, source).is_empty(),
            "checkbox.rs は分類語彙の所有者なので列挙してよい"
        );
    }

    // ---- R2 赤例 (修正前に実在した形) ------------------------------------

    #[test]
    fn r2_detects_stale_ms_comparison_in_interface_adapter() {
        let source = r#"
impl FsWorkspaceLock {
    fn reapable(&self, alive: bool, age: u64) -> bool {
        let reapable = !alive || age > self.stale_ms;
        reapable
    }
}
"#;
        let findings = check(ADAPTER_PATH, source);
        assert_eq!(rules(&findings), vec![RULE_REAP_DECISION_LOCALITY]);
        assert_eq!(findings[0].line, 4);
    }

    #[test]
    fn r2_detects_held_ticks_comparison_in_interface_adapter() {
        let source = r#"
fn stale(held_ticks: u64, threshold: u64) -> bool {
    threshold < held_ticks
}
"#;
        assert_eq!(
            rules(&check(ADAPTER_PATH, source)),
            vec![RULE_REAP_DECISION_LOCALITY]
        );
    }

    // ---- R2 緑例 ---------------------------------------------------------

    #[test]
    fn r2_allows_delegating_to_the_domain_predicate() {
        // 関数引数は比較ではない — 判定はドメインに残っている。
        let source = r#"
impl FsWorkspaceLock {
    fn reapable(&self, alive: bool, age: u64) -> bool {
        reap_eligible(alive, age, self.stale_ms)
    }
}
"#;
        assert!(check(ADAPTER_PATH, source).is_empty());
    }

    #[test]
    fn r2_does_not_apply_to_domain_paths() {
        let source = r#"
pub const fn reap_eligible(owner_alive: bool, held_elapsed: u64, stale_ms: u64) -> bool {
    !owner_alive || held_elapsed > stale_ms
}
"#;
        assert!(
            check("modules/core/domain/src/workspace/lock_protocol.rs", source).is_empty(),
            "境界規約の単一実装はドメイン側に置かれる"
        );
    }

    #[test]
    fn r2_ignores_cfg_test_modules_and_integration_tests() {
        let source = r#"
#[cfg(test)]
mod tests {
    fn red(age: u64, stale_ms: u64) -> bool {
        age > stale_ms
    }
}
"#;
        assert!(check(ADAPTER_PATH, source).is_empty());

        let bare = r#"
fn red(age: u64, stale_ms: u64) -> bool {
    age > stale_ms
}
"#;
        assert!(check(ADAPTER_TEST_PATH, bare).is_empty());
    }

    #[test]
    fn r2_is_suppressed_by_a_matching_allow_comment() {
        let source = r#"
fn red(age: u64, stale_ms: u64) -> bool {
    // amadeus-lint: allow(reap-decision-locality) — 理由
    age > stale_ms
}
"#;
        assert!(check(ADAPTER_PATH, source).is_empty());
    }

    // ---- 共通 ------------------------------------------------------------

    #[test]
    fn findings_are_sorted_by_line() {
        let source = r#"
fn a(cb: CheckboxState) -> bool {
    matches!(cb, CheckboxState::Completed | CheckboxState::Skipped)
}

fn b(cb: CheckboxState) -> bool {
    matches!(cb, CheckboxState::InProgress | CheckboxState::Revising)
}
"#;
        let findings = check(DOMAIN_PATH, source);
        assert_eq!(findings.len(), 2);
        assert!(findings[0].line < findings[1].line);
    }

    #[test]
    fn unparsable_source_is_reported_as_an_error() {
        assert!(check_source(DOMAIN_PATH, "fn broken( {").is_err());
    }
}
