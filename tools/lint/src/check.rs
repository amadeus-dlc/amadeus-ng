//! ルール本体 — 「ソース文字列 + 仮想パス → 所見リスト」の純粋関数として実装する。
//!
//! この形にしておくことで、各ルールの検査力 (= 実際に赤例を落とせること) を I/O 抜きの
//! 単体テストで固定できる。検出例の無いルールは追加しない (このリポジトリの Quint ゲートと
//! 同じ DoD)。
//!
//! 検出の哲学 (R1): getter の存在そのものは咎めない。**他オブジェクトから状態を抜き出して
//! 所有者の判断を呼出側で代行する**濫用パターンだけを検出する。R3 はこの前段 — アクセサを
//! 経由せず内部構造をそのまま公開する `pub` フィールドを禁じる。

use std::collections::BTreeSet;

use proc_macro2::{Ident, TokenStream, TokenTree};
use syn::spanned::Spanned;
use syn::visit::Visit;

/// R1: 分類語彙 (`CheckboxState` の変種集合) を所有者の外で再実装している。
pub(crate) const RULE_CHECKBOX_VOCABULARY: &str = "checkbox-vocabulary";
/// R3: struct が内部構造を `pub` フィールドとしてそのまま公開している
/// (`aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/field-visibility.md`)。
///
/// 検出境界: **無制限の `pub` だけ**を検出する。`pub(crate)` / `pub(super)` /
/// `pub(in path)` といった制限付き可視性は検出しない — ルール文書が
/// 「`pub(crate)` は同一クレート内の実装詳細共有にのみ許す」と条件付きで認めており、
/// 無条件の検出は過剰だから。`enum` の変種フィールドは言語仕様上 private にできず
/// (syn 上も可視性を持たない) 対象外。
pub(crate) const RULE_NO_PUBLIC_FIELDS: &str = "no-public-fields";

/// R1 の語彙所有者。この 1 ファイルだけは変種を列挙してよい (分類述語の実装本体)。
const CHECKBOX_OWNER: &str = "modules/core/domain/src/workspace/checkbox.rs";

const CHECKBOX_HELP: &str = "CheckboxState の述語 (is_in_flight / is_finished / is_active) を使う。\
集約が所有する遷移前提集合 (I7 / I13 等) であれば \
`// amadeus-lint: allow(checkbox-vocabulary) — 理由` で理由を明示する";
const NO_PUBLIC_FIELDS_HELP: &str = "フィールドは private にし、アクセサ \
(as_str / message / フィールド名) と必要なら new() を公開する — aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/field-visibility.md";

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

/// 所見の開始行の直前行が `// amadeus-lint: allow(<rule-id>) <理由>` で始まれば抑制する。
///
/// 抑制には**理由の記述が必須**である。rule-id の閉じ括弧の後ろに実質的な文字が無い裸の
/// `allow` は抑制しない — 逃げ道は残すが、黙って使えないようにするための設計である
/// (coding-rules/factory-naming.md「機械化の候補」)。誤検出のあるルールでも、例外に理由を
/// 書かせれば `allow` の量産が根拠の蓄積に変わる。区切り記号 (`—` / `-` / `:` など) は
/// 問わず、何か書いてあることだけを見る — 理由の**質**は機械には測れないのでレビューの仕事。
///
/// rule-id が一致しない allow は抑制しない (別ルールの許可で塗り潰さないため)。
fn is_suppressed(lines: &[&str], finding: &Finding) -> bool {
    if finding.line < 2 {
        return false;
    }
    let marker = format!("// amadeus-lint: allow({})", finding.rule);
    lines.get(finding.line - 2).is_some_and(|prev| {
        let trimmed = prev.trim();
        let Some(rest) = trimmed.strip_prefix(&marker) else {
            return false;
        };
        has_reason(rest)
    })
}

/// `allow(<rule-id>)` の後ろに理由が書かれているか。
///
/// 区切り記号と空白だけを剥がし、残りに文字が 1 つでもあれば理由とみなす。
fn has_reason(rest: &str) -> bool {
    rest.trim_matches(|c: char| {
        c.is_whitespace() || matches!(c, '—' | '–' | '-' | ':' | '：' | '=' | '/' | '#')
    })
    .chars()
    .next()
    .is_some()
}

struct Visitor {
    checkbox_rule: bool,
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

    fn push_public_field(&mut self, line: usize, name: &str) {
        self.findings.push(Finding {
            rule: RULE_NO_PUBLIC_FIELDS,
            line,
            message: format!(
                "struct の pub フィールド `{name}` — 内部構造の直接公開 (field-visibility 違反)"
            ),
            help: NO_PUBLIC_FIELDS_HELP,
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

    /// R3: struct のフィールド (名前付き / tuple) が無制限の `pub` を持つ。
    /// 所見はフィールドごとに 1 件、行番号はそのフィールドの span を指す。
    fn visit_item_struct(&mut self, node: &'ast syn::ItemStruct) {
        for (index, field) in node.fields.iter().enumerate() {
            if !matches!(field.vis, syn::Visibility::Public(_)) {
                continue;
            }
            let name = field
                .ident
                .as_ref()
                .map_or_else(|| index.to_string(), ToString::to_string);
            // 属性 (doc コメント) を含む `field.span()` ではなく `pub` トークンの span を
            // 使う。抑制コメントは所見行の直前行に置く規約なので、行は `pub` を指す方がよい。
            let line = field.vis.span().start().line;
            self.push_public_field(line, &name);
        }
        syn::visit::visit_item_struct(self, node);
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
            syn::Meta::List(list) => stream_has_test_outside_not(&list.tokens),
            _ => false,
        }
    })
}

/// トークン列に「`not(...)` の外側の」裸の識別子 `test` が現れるか。
///
/// `#[cfg(not(test))]` は**非テストビルド専用のプロダクトコード**なので、`test` の出現だけで
/// テスト扱いにすると全ルールから誤免除される (PR #11 レビュー指摘)。`not` 直後の Group は
/// 丸ごと読み飛ばす (`cfg(all(test, not(unix)))` の `test` は正しく拾う)。
fn stream_has_test_outside_not(tokens: &TokenStream) -> bool {
    let mut prev_is_not = false;
    for tree in tokens.clone() {
        match &tree {
            TokenTree::Ident(ident) => {
                if ident == "test" {
                    return true;
                }
                prev_is_not = ident == "not";
                continue;
            }
            TokenTree::Group(group)
                if !prev_is_not && stream_has_test_outside_not(&group.stream()) =>
            {
                return true;
            }
            _ => {}
        }
        prev_is_not = false;
    }
    false
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
    const ADAPTER_PATH: &str =
        "modules/core/command/interface-adapter/src/orchestration/workflow_definition_repository_impl.rs";
    const ADAPTER_TEST_PATH: &str =
        "modules/core/command/interface-adapter/tests/workflow_definition_repository_impl_test.rs";

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
    fn cfg_not_test_modules_are_not_exempt() {
        // `#[cfg(not(test))]` は非テストビルド専用のプロダクトコード — 免除してはならない
        let source = r#"
#[cfg(not(test))]
mod wire {
    fn red(cb: CheckboxState) -> bool {
        matches!(cb, CheckboxState::InProgress | CheckboxState::AwaitingApproval)
    }
}
"#;
        assert_eq!(
            rules(&check(DOMAIN_PATH, source)),
            vec![RULE_CHECKBOX_VOCABULARY]
        );
    }

    #[test]
    fn cfg_all_test_modules_stay_exempt_and_not_inside_all_does_not_leak() {
        let exempt = r#"
#[cfg(all(test, unix))]
mod tests {
    fn red(cb: CheckboxState) -> bool {
        matches!(cb, CheckboxState::InProgress | CheckboxState::AwaitingApproval)
    }
}
"#;
        assert!(check(DOMAIN_PATH, exempt).is_empty());

        // all(not(test), unix) はテストではない — 免除されない
        let not_exempt = r#"
#[cfg(all(not(test), unix))]
mod wire {
    fn red(cb: CheckboxState) -> bool {
        matches!(cb, CheckboxState::InProgress | CheckboxState::AwaitingApproval)
    }
}
"#;
        assert_eq!(
            rules(&check(DOMAIN_PATH, not_exempt)),
            vec![RULE_CHECKBOX_VOCABULARY]
        );
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
    fn a_bare_allow_without_a_reason_does_not_suppress() {
        let source = r#"
fn report(cb: CheckboxState) -> bool {
    // amadeus-lint: allow(checkbox-vocabulary)
    matches!(cb, CheckboxState::InProgress | CheckboxState::AwaitingApproval)
}
"#;
        assert_eq!(
            rules(&check(DOMAIN_PATH, source)),
            vec![RULE_CHECKBOX_VOCABULARY],
            "理由の無い裸の allow は抑制しない"
        );
    }

    #[test]
    fn an_allow_whose_reason_is_only_punctuation_does_not_suppress() {
        let source = r#"
fn report(cb: CheckboxState) -> bool {
    // amadeus-lint: allow(checkbox-vocabulary) —
    matches!(cb, CheckboxState::InProgress | CheckboxState::AwaitingApproval)
}
"#;
        assert_eq!(
            rules(&check(DOMAIN_PATH, source)),
            vec![RULE_CHECKBOX_VOCABULARY],
            "区切り記号だけでは理由とみなさない"
        );
    }

    #[test]
    fn an_allow_with_a_reason_and_no_separator_suppresses() {
        let source = r#"
fn report(cb: CheckboxState) -> bool {
    // amadeus-lint: allow(checkbox-vocabulary) 集約が語彙を所有するため
    matches!(cb, CheckboxState::InProgress | CheckboxState::AwaitingApproval)
}
"#;
        assert!(
            check(DOMAIN_PATH, source).is_empty(),
            "区切り記号は問わない — 何か書いてあれば理由とみなす"
        );
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

    // ---- R3 赤例 (フィールド可視性スイープ前に実在した形) ------------------

    #[test]
    fn r3_detects_public_tuple_struct_field() {
        // スイープ前の `UnknownPhase` はエラー文字列を裸で公開していた。
        let source = r#"
pub struct UnknownPhase(pub String);
"#;
        let findings = check(DOMAIN_PATH, source);
        assert_eq!(rules(&findings), vec![RULE_NO_PUBLIC_FIELDS]);
        assert_eq!(findings[0].line, 2, "tuple フィールドの span を指すこと");
        assert!(
            findings[0].message.contains("`0`"),
            "tuple フィールドは index を message に含めること: {}",
            findings[0].message
        );
    }

    #[test]
    fn r3_reports_one_finding_per_named_public_field() {
        // スイープ前の `UnknownScope` は 2 フィールドとも pub だった。
        let source = r#"
pub struct UnknownScope {
    pub scope: String,
    pub valid_scopes: Vec<String>,
}
"#;
        let findings = check(DOMAIN_PATH, source);
        assert_eq!(
            rules(&findings),
            vec![RULE_NO_PUBLIC_FIELDS, RULE_NO_PUBLIC_FIELDS],
            "pub フィールドごとに 1 所見"
        );
        assert_eq!(findings[0].line, 3, "1 件目は scope のフィールド行");
        assert_eq!(findings[1].line, 4, "2 件目は valid_scopes のフィールド行");
        assert!(findings[0].message.contains("`scope`"));
        assert!(findings[1].message.contains("`valid_scopes`"));
    }

    #[test]
    fn r3_applies_outside_domain_paths_too() {
        // R1 と違い語彙の所有者による免除が無く、適用範囲はリポジトリ全体
        // (modules/ 配下の非テストコード)。
        let source = r#"
pub struct Snapshot(pub Vec<u8>);
"#;
        assert_eq!(
            rules(&check(ADAPTER_PATH, source)),
            vec![RULE_NO_PUBLIC_FIELDS]
        );
    }

    // ---- R3 緑例 ---------------------------------------------------------

    #[test]
    fn r3_allows_private_fields_with_accessors() {
        // スイープ後の現行形。
        let source = r#"
pub struct UnknownPhase(String);

impl UnknownPhase {
    pub fn new(phase: String) -> Self {
        Self(phase)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

pub struct UnknownScope {
    scope: String,
    valid_scopes: Vec<String>,
}

impl UnknownScope {
    pub fn scope(&self) -> &str {
        &self.scope
    }

    pub fn valid_scopes(&self) -> &[String] {
        &self.valid_scopes
    }
}
"#;
        assert!(check(DOMAIN_PATH, source).is_empty());
    }

    #[test]
    fn r3_ignores_enum_variant_fields() {
        // enum の変種フィールドは言語仕様上 private にできないので対象外。
        let source = r#"
pub enum E {
    V { x: u32 },
    W(String),
}
"#;
        assert!(check(DOMAIN_PATH, source).is_empty());
    }

    #[test]
    fn r3_ignores_restricted_visibility() {
        // pub(crate) は同一クレート内の実装詳細共有として条件付きで許される。
        let source = r#"
pub struct Inner {
    pub(crate) shared: u32,
    pub(super) parent_only: u32,
    pub(in crate::workspace) scoped: u32,
}

pub struct Wrapper(pub(crate) String);
"#;
        assert!(check(DOMAIN_PATH, source).is_empty());
    }

    #[test]
    fn r3_ignores_cfg_test_modules_and_integration_tests() {
        let source = r#"
#[cfg(test)]
mod tests {
    pub struct Fixture {
        pub scope: String,
    }
}
"#;
        assert!(check(DOMAIN_PATH, source).is_empty());

        let bare = r#"
pub struct Fixture(pub String);
"#;
        assert!(check(ADAPTER_TEST_PATH, bare).is_empty());
    }

    #[test]
    fn r3_is_suppressed_by_a_matching_allow_comment() {
        let source = r#"
pub struct Wire {
    // amadeus-lint: allow(no-public-fields) — serde の外部表現 (境界の DTO)
    pub scope: String,
}
"#;
        assert!(check(DOMAIN_PATH, source).is_empty());
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
