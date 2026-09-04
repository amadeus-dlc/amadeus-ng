//! ルール本体 — 「ソース文字列 + 仮想パス → 所見リスト」の純粋関数として実装する。
//!
//! この形にしておくことで、各ルールの検査力 (= 実際に赤例を落とせること) を I/O 抜きの
//! 単体テストで固定できる。検出例の無いルールは追加しない (このリポジトリの Quint ゲートと
//! 同じ DoD)。
//!
//! 検出の哲学 (R1): getter の存在そのものは咎めない。**他オブジェクトから状態を抜き出して
//! 所有者の判断を呼出側で代行する**濫用パターンだけを検出する。R3 はこの前段 — アクセサを
//! 経由せず内部構造をそのまま公開する `pub` フィールドを禁じる。R4 はカプセル化の単位を
//! ファイル構成で支える — 1 ファイル 1 公開型 (「モジュール private ≒ struct private」の
//! 成立条件、abstract-data-type.md)。R5 は CQRS の読取側の規律 — クエリ側 DAO の SQL が
//! 1 文で 2 表以上を読んでいないか (1 表 1 引当、cqrs-boundaries.md 規則 6)。R6 は
//! アプリケーション境界の語彙 — use-case 層の公開ポート名がコマンド側 `XxxRepository` /
//! クエリ側 `XxxDao` に収まっているか (造語ポートの禁止、gateway-taxonomy.md §1/§3/§5)。
//! R7 はその裏返しで、コマンド側で外界に触ってよいのは Repository 実装だけ、という責務境界を
//! fs / 乱数 / プロセス / ネットワークの使用箇所で押さえる (gateway-taxonomy.md §1)。

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
/// R4: 1 ファイルに公開型 (`pub struct` / `pub enum` / `pub trait`) が 2 つ以上ある
/// (`aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/abstract-data-type.md` — 1 ファイル 1 公開型、
/// オーナー裁定 2026-09-01)。
///
/// 検出境界: **ファイルのトップレベル**の**無制限 `pub`** に限る。`pub(crate)` 以下
/// (package-private 相当)・private 補助型・`pub type` エイリアス・自由関数は同居可で、
/// 検出しない。深い `pub mod` の中の型は module-visibility 側の主題なので数えない。
/// 公開型ゼロの自由関数モジュール (`codec.rs` 等) は正当。
pub(crate) const RULE_ONE_PUBLIC_TYPE: &str = "one-public-type";
/// R5: クエリ側 DAO の SQL が 1 文で 2 つ以上の `read_*` 表を読んでいる
/// (`aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/cqrs-boundaries.md` 規則 6 — DAO は 1 表 1 引当、
/// オーナー裁定 2026-09-03)。
///
/// 検出境界: **クエリ側インターフェイスアダプタ** ([`QUERY_ADAPTER_SCOPE`]) の非テストコードに
/// 限る。RMU (投影核) は 15 表を 1 バッチで差し替えるのが仕事なので射程外。検査するのは
/// ソース上のリテラル SQL だけで、属性 (doc コメント) のリテラルは見ない — doc が「JOIN」と
/// 書いただけで鳴らないようにするため。
pub(crate) const RULE_DAO_SINGLE_TABLE: &str = "dao-single-table";
/// R6: use-case 層の公開ポート名が側の規約から外れている
/// (`aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/gateway-taxonomy.md` §1/§3/§5 —
/// コマンド側は `XxxRepository`、クエリ側は `XxxDao`。造語ポート (`Store` / `Reader` /
/// `Writer` / `Source` / `Provider`) の禁止、オーナー裁定 2026-08-22・改訂 2026-08-31)。
///
/// 検出境界: **use-case 層** ([`COMMAND_USE_CASE_SCOPE`] / [`QUERY_USE_CASE_SCOPE`]) の
/// **無制限 `pub` の trait** に限る (R4 と同じ可視性境界)。domain / interface-adapter /
/// RMU / app は射程外 — アダプタ層の機構 trait (`Clock` / `WorkspaceScanner`) はポートでは
/// なく注入シームであり (§1 の「機構は Gateway ではない」)、RMU の `JournalReader` は
/// §1c の永続化基盤ポートとして名指しで例外に置かれているから。外部システムクライアント
/// (`XxxClient`) のような正当な非 Repository ポートは理由付き allow で通す。
pub(crate) const RULE_PORT_NAMING: &str = "port-naming";
/// R7: コマンド側の Repository 実装以外に fs / 乱数 / プロセス / ネットワークの I/O がある
/// (`aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/gateway-taxonomy.md` §1d —
/// コマンド側で外界に触るのは Repository 実装だけ、GitHub #47・オーナー示唆 2026-08-30)。
///
/// 検出境界: **コマンド側 3 クレート** ([`COMMAND_SCOPE`]) の非テストコードのうち、
/// [`REPOSITORY_IMPL_SUFFIX`] で終わるファイルを除いた全部。検出するのは
/// [`IO_STD_MODULES`] と [`IO_RANDOM_CRATES`] への経路 (`use` ツリーの平坦化と完全修飾パス)
/// だけで、`std::io` 単独 (エラー型の写像)・`std::time` (Clock はアダプタ層の機構)・
/// `uuid` (集約内採番はオーナー裁定 2026-09-02 で正当) は鳴らない。
pub(crate) const RULE_COMMAND_SIDE_IO: &str = "command-side-io";

/// R1 の語彙所有者。この 1 ファイルだけは変種を列挙してよい (分類述語の実装本体)。
/// b32 の 1 ファイル 1 公開型分割で `checkbox.rs` から `checkbox_state.rs` へ移った
/// (`CheckboxState` の自ファイル) のに追随している。
const CHECKBOX_OWNER: &str = "modules/core/command/domain/src/workspace/checkbox_state.rs";

/// R5 の射程。クエリ側の DAO 実装だけが「1 表 1 引当」の対象である
/// (コマンド側・RMU・app は別の責務を持つ)。
const QUERY_ADAPTER_SCOPE: &str = "modules/core/query/interface-adapter/src/";

/// R6 の射程 (コマンド側)。ここの公開 trait はアプリケーション境界のポートである。
const COMMAND_USE_CASE_SCOPE: &str = "modules/core/command/use-case/src/";
/// R6 の射程 (クエリ側)。読む先が集約ではなくリードモデルなので `Dao` を名乗る。
const QUERY_USE_CASE_SCOPE: &str = "modules/core/query/use-case/src/";

/// R7 の射程。コマンド側 3 クレート (domain / use-case / interface-adapter) をまとめて覆う。
const COMMAND_SCOPE: &str = "modules/core/command/";
/// R7 の唯一の除外。Repository 実装だけが外界に触ってよい (gateway-taxonomy §1d)。
const REPOSITORY_IMPL_SUFFIX: &str = "_repository_impl.rs";
/// R7 が検出する `std::<name>` の I/O モジュール。
const IO_STD_MODULES: [&str; 3] = ["fs", "process", "net"];
/// R7 が検出する乱数クレート。ID 採番は `uuid` の責務なので、ここに `uuid` は入らない。
const IO_RANDOM_CRATES: [&str; 3] = ["getrandom", "rand", "rand_core"];

const CHECKBOX_HELP: &str = "CheckboxState の述語 (is_in_flight / is_finished / is_active) を使う。\
集約が所有する遷移前提集合 (I7 / I13 等) であれば \
`// amadeus-lint: allow(checkbox-vocabulary) — 理由` で理由を明示する";
const NO_PUBLIC_FIELDS_HELP: &str = "フィールドは private にし、アクセサ \
(as_str / message / フィールド名) と必要なら new() を公開する — aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/field-visibility.md";
const ONE_PUBLIC_TYPE_HELP: &str = "公開型ごとに型名の snake_case のファイルへ分け、\
ファサード (mod.rs) の pub use で公開する — \
aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/abstract-data-type.md (1 ファイル 1 公開型)";
const DAO_SINGLE_TABLE_HELP: &str = "表ごとに DAO を分け、FK はユースケースがたどる — \
aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/cqrs-boundaries.md (規則 6: DAO は 1 表 1 引当)";
const PORT_NAMING_HELP: &str = "ポートは集約名 + Repository (コマンド側) / リードモデル名 + Dao \
(クエリ側) で名付ける。外部システムクライアント (XxxClient) のような正当な非 Repository ポートは \
`// amadeus-lint: allow(port-naming) — 理由` で理由を明示する — \
aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/gateway-taxonomy.md";
const COMMAND_SIDE_IO_HELP: &str = "I/O は Repository 実装 (*_repository_impl.rs) へ移す。\
正当な例外は `// amadeus-lint: allow(command-side-io) — 理由` で理由を明示する — \
aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/gateway-taxonomy.md";

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
        dao_rule: path.starts_with(QUERY_ADAPTER_SCOPE),
        port_side: port_side_of(&path),
        io_rule: is_command_side_io_scope(&path),
        io_lines: BTreeSet::new(),
        findings: Vec::new(),
    };
    visitor.visit_file(&file);
    visitor.findings.extend(one_public_type_findings(&file));

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

/// R6 の射程判定 — このファイルがどちら側の use-case 層か (どちらでもなければ `None`)。
fn port_side_of(path: &str) -> Option<PortSide> {
    if path.starts_with(COMMAND_USE_CASE_SCOPE) {
        Some(PortSide::Command)
    } else if path.starts_with(QUERY_USE_CASE_SCOPE) {
        Some(PortSide::Query)
    } else {
        None
    }
}

/// R7 の射程判定 — コマンド側で、かつ Repository 実装ではないファイル。
fn is_command_side_io_scope(path: &str) -> bool {
    path.starts_with(COMMAND_SCOPE) && !path.ends_with(REPOSITORY_IMPL_SUFFIX)
}

/// R6 の側。ポートの接尾辞と、外れたときに示す期待をこの型が持つ。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PortSide {
    /// コマンド側 — 読む先は集約なので `XxxRepository`。
    Command,
    /// クエリ側 — 読む先はリードモデルなので `XxxDao`。
    Query,
}

impl PortSide {
    /// 公開ポート名に要求する接尾辞。
    const fn suffix(self) -> &'static str {
        match self {
            PortSide::Command => "Repository",
            PortSide::Query => "Dao",
        }
    }

    /// 所見に添える「本来こうである」の一言。
    const fn expectation(self) -> &'static str {
        match self {
            PortSide::Command => "コマンド側のポートは集約名 + Repository だけ",
            PortSide::Query => "クエリ側の読取ポートはリードモデル名 + Dao だけ",
        }
    }
}

/// R4: ファイルのトップレベルにある無制限 `pub` の型宣言を文書順に集め、
/// 2 つ目以降を所見にする。行は `pub` トークン (可視性) の行 — 抑制コメントを
/// doc コメントの後・宣言の直前行に置ける位置で報告する (R3 と同じ規約)。
fn one_public_type_findings(file: &syn::File) -> Vec<Finding> {
    let mut types: Vec<(usize, String)> = Vec::new();
    for item in &file.items {
        if has_cfg_test(item_attrs(item)) {
            continue;
        }
        let (vis, ident) = match item {
            syn::Item::Struct(i) => (&i.vis, &i.ident),
            syn::Item::Enum(i) => (&i.vis, &i.ident),
            syn::Item::Trait(i) => (&i.vis, &i.ident),
            _ => continue,
        };
        if !matches!(vis, syn::Visibility::Public(_)) {
            continue;
        }
        types.push((vis.span().start().line, ident.to_string()));
    }
    let Some(((_, first), surplus)) = types.split_first() else {
        return Vec::new();
    };
    surplus
        .iter()
        .map(|(line, name)| Finding {
            rule: RULE_ONE_PUBLIC_TYPE,
            line: *line,
            message: format!(
                "ファイル 2 つ目以降の公開型 `{name}` — 1 ファイル 1 公開型 (最初の公開型は `{first}`)"
            ),
            help: ONE_PUBLIC_TYPE_HELP,
        })
        .collect()
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
    dao_rule: bool,
    /// R6 の射程。`None` なら use-case 層ではないので検査しない。
    port_side: Option<PortSide>,
    io_rule: bool,
    /// R7 を報告済みの行。1 行に複数の I/O 経路が綴られても所見は 1 件に畳む。
    io_lines: BTreeSet<usize>,
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

    /// R5: SQL テキスト 1 つ分を検査し、違反なら所見を積む。
    fn check_dao_sql(&mut self, line: usize, sql: &str) {
        if !self.dao_rule {
            return;
        }
        if let Some(message) = dao_single_table_message(sql) {
            self.findings.push(Finding {
                rule: RULE_DAO_SINGLE_TABLE,
                line,
                message,
                help: DAO_SINGLE_TABLE_HELP,
            });
        }
    }

    /// R6: 射程内の無制限 `pub trait` 名を接尾辞で検査する。
    fn check_port_name(&mut self, line: usize, name: &str) {
        let Some(side) = self.port_side else {
            return;
        };
        if name.ends_with(side.suffix()) {
            return;
        }
        self.findings.push(Finding {
            rule: RULE_PORT_NAMING,
            line,
            message: format!(
                "use-case 層の pub trait `{name}` が `{}` で終わらない — {} (gateway-taxonomy 違反)",
                side.suffix(),
                side.expectation()
            ),
            help: PORT_NAMING_HELP,
        });
    }

    /// R7: I/O 経路 1 つ分を報告する。同じ行の 2 件目以降は捨てる。
    fn push_command_side_io(&mut self, line: usize, label: &str) {
        if !self.io_rule || !self.io_lines.insert(line) {
            return;
        }
        self.findings.push(Finding {
            rule: RULE_COMMAND_SIDE_IO,
            line,
            message: format!(
                "コマンド側の `{label}` — fs / 乱数 / プロセス / ネットワークの I/O は \
Repository 実装 (`*_repository_impl.rs`) だけに置く (gateway-taxonomy §1 違反)"
            ),
            help: COMMAND_SIDE_IO_HELP,
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
    ///
    /// R5 (b): `concat!` 1 呼出のリテラルを順に連結したものが 1 つの SQL テキスト。
    fn visit_expr_macro(&mut self, node: &'ast syn::ExprMacro) {
        if self.checkbox_rule && path_ends_with_ident(&node.mac.path, "matches") {
            let mut variants = BTreeSet::new();
            collect_checkbox_variants_in_tokens(&node.mac.tokens, &mut variants);
            if variants.len() >= 2 {
                let line = node.span().start().line;
                self.push_checkbox(line, &variants);
            }
        }
        if self.dao_rule && path_ends_with_ident(&node.mac.path, "concat") {
            let line = node.mac.path.span().start().line;
            self.check_dao_sql(line, &concat_literals(&node.mac.tokens));
        }
        syn::visit::visit_expr_macro(self, node);
    }

    /// R5 (c): `macro_rules!` 本体に埋め込まれた `concat!`。マクロ定義の中身は未解析の
    /// トークン列なので、`concat` `!` `(..)` の並びを直接探す。展開先 (`select_continuation!()`)
    /// にはリテラルが無いので、同じ SQL を二重に数えることはない。
    fn visit_item_macro(&mut self, node: &'ast syn::ItemMacro) {
        if self.dao_rule {
            let mut found = Vec::new();
            collect_concat_sql_in_tokens(&node.mac.tokens, &mut found);
            for (line, sql) in found {
                self.check_dao_sql(line, &sql);
            }
        }
        syn::visit::visit_item_macro(self, node);
    }

    /// R5 (a): 単独の文字列リテラル。
    fn visit_lit_str(&mut self, node: &'ast syn::LitStr) {
        self.check_dao_sql(node.span().start().line, &node.value());
        syn::visit::visit_lit_str(self, node);
    }

    /// 属性のリテラルは走査しない。doc コメントは `#[doc = "..."]` になるので、
    /// 「単独 SELECT でも JOIN でも同じ選択句を使える」と**説明した**だけの行が
    /// R5 の所見になってしまうのを防ぐ。
    fn visit_attribute(&mut self, _node: &'ast syn::Attribute) {}

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

    /// R6: 無制限 `pub` の trait 宣言。行は `pub` トークンの行 — 抑制コメントを
    /// doc コメントの後・宣言の直前行に置ける位置で報告する (R3 / R4 と同じ規約)。
    fn visit_item_trait(&mut self, node: &'ast syn::ItemTrait) {
        if matches!(node.vis, syn::Visibility::Public(_)) {
            let line = node.vis.span().start().line;
            self.check_port_name(line, &node.ident.to_string());
        }
        syn::visit::visit_item_trait(self, node);
    }

    /// R7 (a): `use` 文。ツリーを平坦化して 1 本ずつのパスに直してから判定する
    /// (`use std::{fs, io};` / `use std::fs::{self, File};` を取り違えないため)。
    fn visit_item_use(&mut self, node: &'ast syn::ItemUse) {
        if self.io_rule {
            let line = node.use_token.span().start().line;
            let mut prefix = Vec::new();
            let mut paths = Vec::new();
            flatten_use_tree(&node.tree, &mut prefix, &mut paths);
            for segments in &paths {
                if let Some(label) = io_label(segments, true) {
                    self.push_command_side_io(line, &label);
                }
            }
        }
        syn::visit::visit_item_use(self, node);
    }

    /// R7 (b): 式・型・パターン中の完全修飾パス (`std::fs::read_to_string` など)。
    /// `use` ツリーは [`syn::Path`] を持たないので、(a) と二重に数えることはない。
    fn visit_path(&mut self, node: &'ast syn::Path) {
        if self.io_rule
            && let Some(first) = node.segments.first()
        {
            let segments: Vec<String> = node
                .segments
                .iter()
                .map(|seg| seg.ident.to_string())
                .collect();
            if let Some(label) = io_label(&segments, false) {
                self.push_command_side_io(first.ident.span().start().line, &label);
            }
        }
        syn::visit::visit_path(self, node);
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

/// R5 の判定本体 — SQL テキスト 1 つが「1 表 1 引当」に収まっているか。
///
/// `SELECT` を語として含まないテキストは SQL とみなさない (DDL・`DELETE` の全差し替え
/// バッチ・普通の文字列は対象外)。違反の本則は**相異なる `read_*` 表が 2 つ以上**で、
/// これが `JOIN` もカンマ結合も `UNION` も `EXISTS` も一様に捉える。`JOIN` の語と
/// 副問合せ `(SELECT …)` は、表が 1 つしか綴られていない断片 (連結の一部) でも
/// 掴まえるための補助であり、報告する理由を具体的にする役目も持つ。
fn dao_single_table_message(sql: &str) -> Option<String> {
    if !contains_word(sql, "select") {
        return None;
    }
    let tables = read_table_names(sql);
    let listed: Vec<String> = tables.iter().map(|name| format!("`{name}`")).collect();
    let detail = if listed.is_empty() {
        String::new()
    } else {
        format!(" ({})", listed.join(", "))
    };
    let rule = "1 表 1 引当 (cqrs-boundaries 規則 6、2026-09-03)";
    if tables.len() >= 2 {
        return Some(format!(
            "DAO の SQL が {} 表を読んでいる{detail} — {rule}",
            tables.len()
        ));
    }
    if contains_word(sql, "join") {
        return Some(format!("DAO の SQL に JOIN がある{detail} — {rule}"));
    }
    if has_subquery(sql) {
        return Some(format!(
            "DAO の SQL に副問合せ ((SELECT …)) がある{detail} — {rule}"
        ));
    }
    None
}

/// SQL テキストに現れる `read_*` 表名 (重複を畳んだ集合)。
///
/// 識別子の連なりを取り出して `read_` で始まるものだけを拾う。列名は `read_` で始まらない
/// ので (リードモデルの DDL 実測)、これで表だけが残る。
fn read_table_names(sql: &str) -> BTreeSet<String> {
    let lowered = sql.to_ascii_lowercase();
    let mut names = BTreeSet::new();
    let mut current = String::new();
    for ch in lowered.chars() {
        if is_ident_char(ch) {
            current.push(ch);
            continue;
        }
        push_read_table(&mut names, &current);
        current.clear();
    }
    push_read_table(&mut names, &current);
    names
}

fn push_read_table(names: &mut BTreeSet<String>, ident: &str) {
    if let Some(suffix) = ident.strip_prefix("read_")
        && !suffix.is_empty()
    {
        names.insert(ident.to_string());
    }
}

/// 識別子の境界を守った語の検出 (大文字小文字を問わない)。`joined` は `join` に当たらない。
fn contains_word(text: &str, word: &str) -> bool {
    let lowered = text.to_ascii_lowercase();
    lowered.match_indices(word).any(|(index, _)| {
        let before = lowered[..index].chars().next_back();
        let after = lowered[index + word.len()..].chars().next();
        !before.is_some_and(is_ident_char) && !after.is_some_and(is_ident_char)
    })
}

/// `(` の直後に語としての `select` が来る = 副問合せ (`EXISTS (SELECT …)` 等)。
fn has_subquery(text: &str) -> bool {
    let lowered = text.to_ascii_lowercase();
    lowered.match_indices('(').any(|(index, _)| {
        let rest = lowered[index + 1..].trim_start();
        rest.strip_prefix("select")
            .is_some_and(|after| !after.starts_with(is_ident_char))
    })
}

fn is_ident_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

/// `concat!` の引数トークンから文字列リテラルだけを出現順に連結する。
///
/// リテラル以外 (入れ子のマクロ呼出・区切り) は素通しする — `run_stage_selection!()` の
/// ように展開が要るものは読めないので、**読めた分だけ**で判定する。表を隠す方向へ
/// 誤ることはあっても、無い表を見つけることはない。
fn concat_literals(tokens: &TokenStream) -> String {
    let mut text = String::new();
    push_string_literals(tokens, &mut text);
    text
}

fn push_string_literals(tokens: &TokenStream, out: &mut String) {
    for tree in tokens.clone() {
        match tree {
            TokenTree::Literal(literal) => {
                if let syn::Lit::Str(lit) = syn::Lit::new(literal) {
                    out.push_str(&lit.value());
                }
            }
            TokenTree::Group(group) => push_string_literals(&group.stream(), out),
            _ => {}
        }
    }
}

/// トークン列から `concat` `!` `(..)` の並びを探し、(行, 連結後のテキスト) を集める。
/// 見つけた `concat!` の引数は連結側に任せ、その中を再帰探索し直さない (二重計上の防止)。
fn collect_concat_sql_in_tokens(tokens: &TokenStream, out: &mut Vec<(usize, String)>) {
    let trees: Vec<TokenTree> = tokens.clone().into_iter().collect();
    let mut index = 0usize;
    while index < trees.len() {
        if let (
            Some(TokenTree::Ident(ident)),
            Some(TokenTree::Punct(bang)),
            Some(TokenTree::Group(group)),
        ) = (trees.get(index), trees.get(index + 1), trees.get(index + 2))
            && ident == "concat"
            && bang.as_char() == '!'
        {
            out.push((ident.span().start().line, concat_literals(&group.stream())));
            index += 3;
            continue;
        }
        if let Some(TokenTree::Group(group)) = trees.get(index) {
            collect_concat_sql_in_tokens(&group.stream(), out);
        }
        index += 1;
    }
}

/// `use` ツリーを 1 本ずつの完全パス (セグメント列) へ平坦化する。
///
/// `use std::fs::{self, File};` は `["std","fs","self"]` と `["std","fs","File"]` の 2 本に
/// なり、どちらも先頭 2 つが `std::fs` なので同じ判定に落ちる。`use std::fs::*;` の glob は
/// その時点の接頭辞 (`["std","fs"]`) を 1 本として積む。
fn flatten_use_tree(tree: &syn::UseTree, prefix: &mut Vec<String>, out: &mut Vec<Vec<String>>) {
    match tree {
        syn::UseTree::Path(node) => {
            prefix.push(node.ident.to_string());
            flatten_use_tree(&node.tree, prefix, out);
            prefix.pop();
        }
        syn::UseTree::Name(node) => {
            prefix.push(node.ident.to_string());
            out.push(prefix.clone());
            prefix.pop();
        }
        syn::UseTree::Rename(node) => {
            prefix.push(node.ident.to_string());
            out.push(prefix.clone());
            prefix.pop();
        }
        syn::UseTree::Glob(_) => out.push(prefix.clone()),
        syn::UseTree::Group(node) => {
            for item in &node.items {
                flatten_use_tree(item, prefix, out);
            }
        }
    }
}

/// R7 の判定本体 — セグメント列が I/O 経路なら、所見に載せる表示名を返す。
///
/// `bare_crate_ok` は `use` 文でだけ真にする。式の中の 1 セグメントは
/// **クレート名ではなく変数名でもありうる** (`let rand = ..; rand + 1`) ので、完全修飾パス側は
/// 2 セグメント以上 (`rand::thread_rng`) を要求して誤検出を避ける。
fn io_label(segments: &[String], bare_crate_ok: bool) -> Option<String> {
    let first = segments.first()?;
    if first == "std" {
        let second = segments.get(1)?;
        return IO_STD_MODULES
            .contains(&second.as_str())
            .then(|| format!("std::{second}"));
    }
    if !bare_crate_ok && segments.len() < 2 {
        return None;
    }
    IO_RANDOM_CRATES
        .contains(&first.as_str())
        .then(|| first.clone())
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

    const DOMAIN_PATH: &str = "modules/core/command/domain/src/orchestration/intent_execution.rs";
    const OWNER_PATH: &str = CHECKBOX_OWNER;
    const ADAPTER_PATH: &str = "modules/core/command/interface-adapter/src/orchestration/workflow_definition_repository_impl.rs";
    const ADAPTER_TEST_PATH: &str =
        "modules/core/command/interface-adapter/tests/workflow_definition_repository_impl_test.rs";
    /// R5 の射程 (クエリ側 DAO 実装)。
    const QUERY_ADAPTER_PATH: &str =
        "modules/core/query/interface-adapter/src/next_answer_dao_impl.rs";
    /// 射程内だが `/tests/` を含むパス (is_test_path の免除を単独で確かめる)。
    const QUERY_ADAPTER_TEST_PATH: &str =
        "modules/core/query/interface-adapter/src/tests/dao_fixtures.rs";
    /// 射程外 — RMU は 15 表を 1 バッチで差し替えるのが仕事。
    const RMU_PATH: &str = "modules/core/read-model-updater/src/read_tables/sql.rs";
    /// R6 の射程 (コマンド側 use-case)。
    const COMMAND_USE_CASE_PATH: &str =
        "modules/core/command/use-case/src/orchestration/port/intent_repository.rs";
    /// R6 の射程 (クエリ側 use-case)。
    const QUERY_USE_CASE_PATH: &str =
        "modules/core/query/use-case/src/orchestration/port/run_stage_dao.rs";
    /// 射程内だが `/tests/` を含むパス (コマンド側 use-case)。
    const COMMAND_USE_CASE_TEST_PATH: &str =
        "modules/core/command/use-case/src/tests/port_fixtures.rs";
    /// R7 の唯一の除外 — Repository 実装だけが外界に触ってよい。
    const REPOSITORY_IMPL_PATH: &str =
        "modules/core/command/interface-adapter/src/orchestration/intent_repository_impl.rs";
    /// R7 の射程 (コマンド側アダプタ層で Repository 実装ではないファイル)。
    const COMMAND_ADAPTER_PATH: &str =
        "modules/core/command/interface-adapter/src/workspace_scanner.rs";

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
                "modules/core/command/domain/tests/engine_loop_conformance.rs",
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
        // スイープ後の現行形 (R4 裁定後は 1 ファイル 1 公開型なので、ファイルごとに検査する)。
        let tuple = r#"
pub struct UnknownPhase(String);

impl UnknownPhase {
    pub fn new(phase: String) -> Self {
        Self(phase)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
"#;
        assert!(check(DOMAIN_PATH, tuple).is_empty());

        let named = r#"
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
        assert!(check(DOMAIN_PATH, named).is_empty());
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
        let named = r#"
pub struct Inner {
    pub(crate) shared: u32,
    pub(super) parent_only: u32,
    pub(in crate::workspace) scoped: u32,
}
"#;
        assert!(check(DOMAIN_PATH, named).is_empty());

        let tuple = r#"
pub struct Wrapper(pub(crate) String);
"#;
        assert!(check(DOMAIN_PATH, tuple).is_empty());
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

    // ---- R4 赤例 (2026-09-01 裁定時に実在した形) ---------------------------

    #[test]
    fn r4_detects_a_second_public_type_in_one_file() {
        // 裁定時の典型形: 値オブジェクトとそのエラー enum の同居 (stage_slug.rs ほか約 45 件)。
        let source = r#"
pub struct StageSlug(String);

pub enum StageSlugError {
    Empty,
}
"#;
        let findings = check(DOMAIN_PATH, source);
        assert_eq!(rules(&findings), vec![RULE_ONE_PUBLIC_TYPE]);
        assert_eq!(findings[0].line, 4, "2 つ目の公開型の pub 行を指すこと");
        assert!(
            findings[0].message.contains("`StageSlugError`"),
            "余剰の型名を message に含めること: {}",
            findings[0].message
        );
        assert!(
            findings[0].message.contains("`StageSlug`"),
            "最初の公開型名も message に含めること: {}",
            findings[0].message
        );
    }

    #[test]
    fn r4_reports_one_finding_per_surplus_public_type() {
        // 裁定時の最大例はイベント族 (enum + 変種ペイロード 12 型/1 ファイル)。
        let source = r#"
pub enum IntentExecutionEvent {
    Started(Started),
}

pub struct Started;

pub struct StageCompleted;

pub trait EventLike {}
"#;
        let findings = check(DOMAIN_PATH, source);
        assert_eq!(
            rules(&findings),
            vec![
                RULE_ONE_PUBLIC_TYPE,
                RULE_ONE_PUBLIC_TYPE,
                RULE_ONE_PUBLIC_TYPE
            ],
            "2 つ目以降の公開型ごとに 1 所見 (struct / enum / trait すべて対象)"
        );
        assert_eq!(findings[0].line, 6);
        assert_eq!(findings[1].line, 8);
        assert_eq!(findings[2].line, 10);
    }

    // ---- R4 緑例 ---------------------------------------------------------

    #[test]
    fn r4_allows_one_public_type_with_its_servants() {
        // 同居可: private 補助型・pub type エイリアス・主題型に仕える自由関数・pub(crate) 以下。
        let source = r#"
pub struct StageSlug(String);

struct Parser;

pub(crate) struct SharedDetail;

pub type SlugResult = Result<StageSlug, String>;

pub fn parse_all(raw: &str) -> Vec<StageSlug> {
    Vec::new()
}
"#;
        assert!(check(DOMAIN_PATH, source).is_empty());
    }

    #[test]
    fn r4_allows_zero_public_type_free_function_modules() {
        // 公開型ゼロの自由関数モジュール (codec.rs 等 — 裁定済みの free function 化) は正当。
        let source = r#"
pub fn encode(input: &str) -> String {
    input.to_string()
}

pub fn decode(input: &str) -> String {
    input.to_string()
}
"#;
        assert!(check(ADAPTER_PATH, source).is_empty());
    }

    #[test]
    fn r4_ignores_types_inside_cfg_test_and_test_paths() {
        let source = r#"
pub struct Subject;

#[cfg(test)]
pub struct Fixture;
"#;
        assert!(check(DOMAIN_PATH, source).is_empty());

        let bare = r#"
pub struct A;
pub struct B;
"#;
        assert!(check(ADAPTER_TEST_PATH, bare).is_empty());
    }

    #[test]
    fn r4_does_not_count_types_nested_in_modules() {
        // 検出はファイルのトップレベルに限る (深い pub mod は module-visibility 側の主題)。
        let source = r#"
pub struct Subject;

mod detail {
    pub(crate) struct Inner;
}
"#;
        assert!(check(DOMAIN_PATH, source).is_empty());
    }

    #[test]
    fn r4_applies_outside_domain_paths_too() {
        let source = r#"
pub struct DefinitionPaths;

pub struct WorkflowDefinitionDaoImpl;
"#;
        assert_eq!(
            rules(&check(ADAPTER_PATH, source)),
            vec![RULE_ONE_PUBLIC_TYPE]
        );
    }

    #[test]
    fn r4_is_suppressed_by_a_matching_allow_comment_with_reason() {
        let source = r#"
pub struct Subject;

// amadeus-lint: allow(one-public-type) — 移行途中の暫定同居 (次 Bolt で分離)
pub struct Companion;
"#;
        assert!(check(DOMAIN_PATH, source).is_empty());

        let bare = r#"
pub struct Subject;

// amadeus-lint: allow(one-public-type)
pub struct Companion;
"#;
        assert_eq!(
            rules(&check(DOMAIN_PATH, bare)),
            vec![RULE_ONE_PUBLIC_TYPE],
            "理由の無い裸の allow は抑制しない"
        );
    }

    // ---- R5 赤例 (2026-09-03 裁定時に実在した 4 形) -------------------------

    #[test]
    fn r5_detects_a_join_in_a_plain_string_literal() {
        // 実在形: execution_dao_impl.rs の SELECT_EXECUTION (read_execution × read_intent)。
        let source = r#"
//! `ExecutionDao` の実 Gateway — 実行の現在地を 2 表のキー結合で引く。

/// `read_intent` は実行の `intent_id` を主キー `id` に当てて結合する (定義識別子を運ぶため)。
const SELECT_EXECUTION: &str = "SELECT e.id, e.intent_id, i.definition_id, e.scope, \
e.status, e.cursor_slug, e.parked_at_slug, e.parked_active, e.state_binding \
FROM read_execution e \
JOIN read_intent i ON i.id = e.intent_id \
WHERE e.id = ?1";
"#;
        let findings = check(QUERY_ADAPTER_PATH, source);
        assert_eq!(rules(&findings), vec![RULE_DAO_SINGLE_TABLE]);
        assert_eq!(findings[0].line, 5, "リテラルの開始行を指すこと");
        assert!(
            findings[0].message.contains("`read_execution`")
                && findings[0].message.contains("`read_intent`"),
            "読んでいる表を message に列挙すること: {}",
            findings[0].message
        );
    }

    #[test]
    fn r5_detects_joins_assembled_by_concat_in_a_const() {
        // 実在形: next_answer_dao_impl.rs の SELECT_NEXT_ANSWER (6 表)。
        let source = r#"
const RUN_STAGE_OFFSET: usize = 11;

/// 結合はすべて**キー結合**である。
const SELECT_NEXT_ANSWER: &str = concat!(
    "SELECT a.decision_kind, a.stage_index, a.stage_slug, a.gated, a.checkbox, ",
    "e.scope, e.cursor_slug, e.parked_at_slug, e.status, e.state_binding, ",
    "i.definition_id, ",
    run_stage_selection!(),
    ", p.bundle_digest, p.part_count, p.delivered_paths, sp.rules_content ",
    "FROM read_next_answer a ",
    "JOIN read_execution e ON e.id = a.execution_id ",
    "JOIN read_intent i ON i.id = e.intent_id ",
    "LEFT JOIN read_run_stage r ",
    "  ON r.definition_id = i.definition_id AND r.scope = i.scope ",
    "     AND r.stage_slug = a.stage_slug ",
    "LEFT JOIN read_steering_plan p ON p.phase = r.phase ",
    "LEFT JOIN read_steering_part sp ON sp.phase = r.phase AND sp.part_index = 1 ",
    "WHERE a.execution_id = ?1 AND a.request_kind = ?2"
);
"#;
        let findings = check(QUERY_ADAPTER_PATH, source);
        assert_eq!(
            rules(&findings),
            vec![RULE_DAO_SINGLE_TABLE],
            "concat! 1 呼出を 1 つの SQL とみなし、所見も 1 件"
        );
        assert_eq!(findings[0].line, 5, "concat! 呼出の行を指すこと");
        assert!(
            findings[0].message.contains("6 表")
                && findings[0].message.contains("`read_steering_part`"),
            "連結後の全表を数えること: {}",
            findings[0].message
        );
    }

    #[test]
    fn r5_detects_joins_inside_a_macro_rules_body() {
        // 実在形: continuation_dao_impl.rs の select_continuation! (3 表)。
        let source = r#"
/// 束縛はすべて鍵の一部である — 照合ではなく引当である。
macro_rules! select_continuation {
    () => {
        concat!(
            "SELECT ",
            run_stage_selection!(),
            ", p.part_count, p.delivered_paths, sp.phase, sp.part_index, sp.rules_content ",
            "FROM read_run_stage r ",
            "JOIN read_steering_plan p ON p.phase = r.phase AND p.bundle_digest = ?3 ",
            "LEFT JOIN read_steering_part sp ON sp.phase = r.phase AND sp.part_index = ?4 ",
            "WHERE r.route_digest = ?1 AND r.directive_digest = ?2"
        )
    };
}

const SELECT_CONTINUATION: &str = select_continuation!();
"#;
        let findings = check(QUERY_ADAPTER_PATH, source);
        assert_eq!(
            rules(&findings),
            vec![RULE_DAO_SINGLE_TABLE],
            "マクロ定義本体の concat! も検査し、その展開先では二重に数えない"
        );
        assert_eq!(
            findings[0].line, 5,
            "マクロ本体の concat! 呼出の行を指すこと"
        );
        assert!(
            findings[0].message.contains("3 表"),
            "{}",
            findings[0].message
        );
    }

    #[test]
    fn r5_detects_a_subquery_that_reads_another_table() {
        // 実在形: continuation_dao_impl.rs の EXISTS 副問合せ。`JOIN` の綴りは無く、
        // 連結断片に現れる read_* は 1 つだけなので、副問合せの検出が要る。
        let source = r#"
/// state 束縛を持つ token 用 — その束縛の実行が在ることも鍵に加える。
const SELECT_CONTINUATION_BOUND_TO_STATE: &str = concat!(
    select_continuation!(),
    " AND EXISTS (SELECT 1 FROM read_execution x WHERE x.state_binding = ?5)"
);
"#;
        let findings = check(QUERY_ADAPTER_PATH, source);
        assert_eq!(rules(&findings), vec![RULE_DAO_SINGLE_TABLE]);
        assert_eq!(findings[0].line, 3, "concat! 呼出の行を指すこと");
        assert!(
            findings[0].message.contains("副問合せ"),
            "JOIN でも複数表でもない検出理由を message に書くこと: {}",
            findings[0].message
        );
    }

    // ---- R5 緑例 ---------------------------------------------------------

    #[test]
    fn r5_allows_a_single_table_select() {
        // 是正後の形: 1 文 1 表。concat! で組み立てても表が 1 つなら適法。
        let source = r#"
const SELECT_RUN_STAGE: &str = concat!(
    "SELECT ",
    run_stage_selection!(),
    " FROM read_run_stage r ",
    "WHERE r.definition_id = ?1 AND r.scope = ?2 AND r.stage_slug = ?3"
);

const SELECT_JUMP: &str = "SELECT target_index, target_slug, outcome, refusal \
FROM read_next_jump WHERE execution_id = ?1 AND target_slug = ?2";
"#;
        assert!(check(QUERY_ADAPTER_PATH, source).is_empty());
    }

    #[test]
    fn r5_ignores_sql_words_written_in_doc_comments() {
        // run_stage_columns.rs:7 の「単独 SELECT でも JOIN でも同じ選択句を使える」が
        // 鳴らないこと。doc は属性 (`#[doc = "..."]`) なのでリテラル走査から外す。
        let source = r#"
//! 表別名は `r` に固定する — 単独 SELECT でも JOIN でも同じ選択句を使えるようにするため。

/// `read_next_answer` と `read_execution` を JOIN する SELECT でも同じ並びで読める。
const SELECT_RUN_STAGE: &str = "SELECT r.phase FROM read_run_stage r WHERE r.id = ?1";
"#;
        assert!(check(QUERY_ADAPTER_PATH, source).is_empty());
    }

    #[test]
    fn r5_ignores_cfg_test_modules_and_test_paths() {
        let source = r#"
#[cfg(test)]
mod tests {
    const SELECT_JOINED: &str = "SELECT e.id FROM read_execution e \
JOIN read_intent i ON i.id = e.intent_id WHERE e.id = ?1";
}
"#;
        assert!(check(QUERY_ADAPTER_PATH, source).is_empty());

        let bare = r#"
const SELECT_JOINED: &str = "SELECT e.id FROM read_execution e \
JOIN read_intent i ON i.id = e.intent_id WHERE e.id = ?1";
"#;
        assert!(
            check(QUERY_ADAPTER_TEST_PATH, bare).is_empty(),
            "射程内でも /tests/ を含むパスは対象外"
        );
        assert_eq!(
            rules(&check(QUERY_ADAPTER_PATH, bare)),
            vec![RULE_DAO_SINGLE_TABLE],
            "同じソースが射程内の非テストパスでは鳴ること (緑例が空振りでない証明)"
        );
    }

    #[test]
    fn r5_is_scoped_to_the_query_side_interface_adapter() {
        // RMU は 15 表の全差し替えを 1 バッチで持つのが仕事なので射程外。
        let source = r#"
const SELECT_JOINED: &str = "SELECT e.id FROM read_execution e \
JOIN read_intent i ON i.id = e.intent_id WHERE e.id = ?1";

const DELETE_TABLES: &str = "
DELETE FROM read_execution;
DELETE FROM read_intent;
";
"#;
        assert!(check(RMU_PATH, source).is_empty());
        assert_eq!(
            rules(&check(QUERY_ADAPTER_PATH, source)),
            vec![RULE_DAO_SINGLE_TABLE],
            "同じソースが射程内では鳴ること (DELETE バッチは SELECT が無いので鳴らない)"
        );
    }

    #[test]
    fn r5_is_suppressed_by_a_matching_allow_comment_with_reason() {
        let source = r#"
// amadeus-lint: allow(dao-single-table) — 暫定: 次 Bolt で表ごとの DAO へ分ける
const SELECT_EXECUTION: &str = "SELECT e.id FROM read_execution e \
JOIN read_intent i ON i.id = e.intent_id WHERE e.id = ?1";
"#;
        assert!(check(QUERY_ADAPTER_PATH, source).is_empty());
    }

    #[test]
    fn r5_bare_allow_without_a_reason_does_not_suppress() {
        let source = r#"
// amadeus-lint: allow(dao-single-table)
const SELECT_EXECUTION: &str = "SELECT e.id FROM read_execution e \
JOIN read_intent i ON i.id = e.intent_id WHERE e.id = ?1";
"#;
        assert_eq!(
            rules(&check(QUERY_ADAPTER_PATH, source)),
            vec![RULE_DAO_SINGLE_TABLE],
            "理由の無い裸の allow は抑制しない"
        );
    }

    // ---- R6 赤例 (造語ポート — 2026-08-22 裁定時に実在した形と側の取り違え) ----

    #[test]
    fn r6_detects_a_reader_port_on_the_command_side() {
        // 実在形: 裁定前の `StageGraphReader` (現 `WorkflowDefinitionRepository`)。
        let source = r#"
/// 定義グラフを読む。
pub trait StageGraphReader {
    fn read(&self) -> String;
}
"#;
        let findings = check(COMMAND_USE_CASE_PATH, source);
        assert_eq!(rules(&findings), vec![RULE_PORT_NAMING]);
        assert_eq!(findings[0].line, 3, "pub トークンの行を指すこと");
        assert!(
            findings[0].message.contains("`StageGraphReader`")
                && findings[0].message.contains("`Repository`"),
            "trait 名と期待する接尾辞を message に含めること: {}",
            findings[0].message
        );
    }

    #[test]
    fn r6_detects_a_store_port_on_the_command_side() {
        // 実在形: 裁定で削除された `StateFileStore`。
        let source = r#"
pub trait WorkflowStore {
    fn write(&self);
}
"#;
        let findings = check(COMMAND_USE_CASE_PATH, source);
        assert_eq!(rules(&findings), vec![RULE_PORT_NAMING]);
        assert!(findings[0].message.contains("`WorkflowStore`"));
    }

    #[test]
    fn r6_detects_a_dao_port_on_the_command_side() {
        // 側の取り違え: `Dao` はクエリ側の語彙。コマンド側が読むのは集約である。
        let source = r#"
pub trait DefinitionDao {
    fn find(&self);
}
"#;
        let findings = check(COMMAND_USE_CASE_PATH, source);
        assert_eq!(rules(&findings), vec![RULE_PORT_NAMING]);
        assert!(
            findings[0].message.contains("集約名 + Repository"),
            "コマンド側の期待を message に書くこと: {}",
            findings[0].message
        );
    }

    #[test]
    fn r6_detects_a_repository_port_on_the_query_side() {
        // 側の取り違え: クエリ側が読むのはリードモデルなので Repository とは名乗らない。
        let source = r#"
pub trait ExecutionRepository {
    fn find(&self);
}
"#;
        let findings = check(QUERY_USE_CASE_PATH, source);
        assert_eq!(rules(&findings), vec![RULE_PORT_NAMING]);
        assert!(
            findings[0].message.contains("`Dao`")
                && findings[0].message.contains("リードモデル名 + Dao"),
            "クエリ側の期待を message に書くこと: {}",
            findings[0].message
        );
    }

    // ---- R6 緑例 ---------------------------------------------------------

    #[test]
    fn r6_allows_the_conforming_port_name_on_each_side() {
        let command = r#"
pub trait IntentRepository {
    fn find_by_id(&self);
    fn store(&self);
}
"#;
        assert!(check(COMMAND_USE_CASE_PATH, command).is_empty());

        let query = r#"
pub trait RunStageDao {
    fn find(&self);
}
"#;
        assert!(check(QUERY_USE_CASE_PATH, query).is_empty());
    }

    #[test]
    fn r6_ignores_restricted_and_private_traits() {
        // R4 と同じ可視性境界 — pub(crate) 以下はアプリケーション境界のポートではない。
        let restricted = r#"
pub(crate) trait Helper {
    fn help(&self);
}
"#;
        assert!(check(COMMAND_USE_CASE_PATH, restricted).is_empty());

        let private = r#"
trait Reader {
    fn read(&self);
}
"#;
        assert!(check(COMMAND_USE_CASE_PATH, private).is_empty());
    }

    #[test]
    fn r6_is_scoped_to_the_use_case_layers() {
        // 機構 trait (Clock / WorkspaceScanner) はアダプタ層に居るのでポートではない。
        let source = r#"
pub trait WorkspaceScanner {
    fn scan(&self);
}
"#;
        assert!(check(ADAPTER_PATH, source).is_empty());
        assert!(check(DOMAIN_PATH, source).is_empty());
        // §1c の永続化基盤ポート `JournalReader` は名指しの例外 — RMU も射程外。
        assert!(check(RMU_PATH, source).is_empty());
        assert_eq!(
            rules(&check(COMMAND_USE_CASE_PATH, source)),
            vec![RULE_PORT_NAMING],
            "同じソースが射程内では鳴ること (射程外の緑が空振りでない証明)"
        );
    }

    #[test]
    fn r6_ignores_cfg_test_modules_and_test_paths() {
        let cfg_test = r#"
#[cfg(test)]
mod tests {
    pub trait FakeReader {
        fn read(&self);
    }
}
"#;
        assert!(check(COMMAND_USE_CASE_PATH, cfg_test).is_empty());

        let bare = r#"
pub trait FakeReader {
    fn read(&self);
}
"#;
        assert!(check(COMMAND_USE_CASE_TEST_PATH, bare).is_empty());
        assert_eq!(
            rules(&check(COMMAND_USE_CASE_PATH, bare)),
            vec![RULE_PORT_NAMING],
            "同じソースが射程内の非テストパスでは鳴ること"
        );
    }

    #[test]
    fn r6_is_suppressed_by_a_matching_allow_comment_with_reason() {
        let source = r#"
// amadeus-lint: allow(port-naming) — 外部システムクライアント (GitHub API との RPC)
pub trait GitHubPullRequestClient {
    fn open(&self);
}
"#;
        assert!(check(COMMAND_USE_CASE_PATH, source).is_empty());

        let bare = r#"
// amadeus-lint: allow(port-naming)
pub trait GitHubPullRequestClient {
    fn open(&self);
}
"#;
        assert_eq!(
            rules(&check(COMMAND_USE_CASE_PATH, bare)),
            vec![RULE_PORT_NAMING],
            "理由の無い裸の allow は抑制しない"
        );
    }

    // ---- R7 赤例 (#47 が塞ぐ形) --------------------------------------------

    #[test]
    fn r7_detects_a_use_of_std_fs_in_the_use_case_layer() {
        let source = r#"
use std::fs;

fn read(path: &str) -> String {
    fs::read_to_string(path).unwrap_or_default()
}
"#;
        let findings = check(COMMAND_USE_CASE_PATH, source);
        assert_eq!(rules(&findings), vec![RULE_COMMAND_SIDE_IO]);
        assert_eq!(findings[0].line, 2, "use トークンの行を指すこと");
        assert!(
            findings[0].message.contains("`std::fs`"),
            "検出した経路を message に含めること: {}",
            findings[0].message
        );
    }

    #[test]
    fn r7_detects_a_fully_qualified_std_fs_call_in_the_domain() {
        // domain も射程 — 集約がディスクを読み始めたらそこで設計が壊れている。
        let source = r#"
fn read(path: &str) -> String {
    std::fs::read_to_string(path).unwrap_or_default()
}
"#;
        let findings = check(DOMAIN_PATH, source);
        assert_eq!(rules(&findings), vec![RULE_COMMAND_SIDE_IO]);
        assert_eq!(findings[0].line, 3, "パスの行を指すこと");
    }

    #[test]
    fn r7_detects_fs_hidden_in_a_use_group() {
        let source = r#"
use std::{fs, io};
"#;
        let findings = check(COMMAND_USE_CASE_PATH, source);
        assert_eq!(
            rules(&findings),
            vec![RULE_COMMAND_SIDE_IO],
            "グループを平坦化して fs を見つけ、io は見逃すこと"
        );
        assert!(findings[0].message.contains("`std::fs`"));
    }

    #[test]
    fn r7_detects_a_self_import_inside_a_use_group() {
        let source = r#"
use std::fs::{self, File};
"#;
        assert_eq!(
            rules(&check(COMMAND_USE_CASE_PATH, source)),
            vec![RULE_COMMAND_SIDE_IO],
            "self と File の 2 本に平坦化されても所見は 1 件"
        );
    }

    #[test]
    fn r7_detects_random_number_generation() {
        let imported = r#"
use rand::Rng;
"#;
        assert_eq!(
            rules(&check(DOMAIN_PATH, imported)),
            vec![RULE_COMMAND_SIDE_IO]
        );

        let qualified = r#"
fn fill(buf: &mut [u8]) {
    getrandom::getrandom(buf).ok();
}
"#;
        let findings = check(DOMAIN_PATH, qualified);
        assert_eq!(rules(&findings), vec![RULE_COMMAND_SIDE_IO]);
        assert!(findings[0].message.contains("`getrandom`"));
    }

    #[test]
    fn r7_detects_process_spawning_and_network_access() {
        let process = r#"
fn head() -> bool {
    std::process::Command::new("git").status().is_ok()
}
"#;
        let findings = check(COMMAND_ADAPTER_PATH, process);
        assert_eq!(rules(&findings), vec![RULE_COMMAND_SIDE_IO]);
        assert!(findings[0].message.contains("`std::process`"));

        let network = r#"
use std::net::TcpStream;
"#;
        let findings = check(COMMAND_ADAPTER_PATH, network);
        assert_eq!(rules(&findings), vec![RULE_COMMAND_SIDE_IO]);
        assert!(findings[0].message.contains("`std::net`"));
    }

    // ---- R7 緑例 ---------------------------------------------------------

    #[test]
    fn r7_exempts_repository_implementations() {
        // 実在形: compiled_definition_repository_impl.rs が配布 3 ファイルを読む。
        let source = r#"
use std::fs;

fn read(path: &str) -> String {
    fs::read_to_string(path).unwrap_or_default()
}
"#;
        assert!(
            check(REPOSITORY_IMPL_PATH, source).is_empty(),
            "Repository 実装だけが外界に触ってよい"
        );
        assert_eq!(
            rules(&check(COMMAND_ADAPTER_PATH, source)),
            vec![RULE_COMMAND_SIDE_IO],
            "同じソースが同じ層の非 Repository 実装では鳴ること"
        );
    }

    #[test]
    fn r7_ignores_cfg_test_modules_and_test_paths() {
        let cfg_test = r#"
#[cfg(test)]
mod tests {
    use std::fs;

    fn fixture() -> String {
        fs::read_to_string("f").unwrap_or_default()
    }
}
"#;
        assert!(check(COMMAND_USE_CASE_PATH, cfg_test).is_empty());

        let bare = r#"
use std::fs;
"#;
        assert!(check(COMMAND_USE_CASE_TEST_PATH, bare).is_empty());
        assert_eq!(
            rules(&check(COMMAND_USE_CASE_PATH, bare)),
            vec![RULE_COMMAND_SIDE_IO],
            "同じソースが射程内の非テストパスでは鳴ること"
        );
    }

    #[test]
    fn r7_is_scoped_to_the_command_side() {
        // クエリ側 DAO 実装と RMU は自分でリードモデル (SQLite) を読むのが仕事。
        let source = r#"
use std::fs;
"#;
        assert!(check(QUERY_ADAPTER_PATH, source).is_empty());
        assert!(check(QUERY_USE_CASE_PATH, source).is_empty());
        assert!(check(RMU_PATH, source).is_empty());
        assert_eq!(
            rules(&check(DOMAIN_PATH, source)),
            vec![RULE_COMMAND_SIDE_IO],
            "同じソースがコマンド側では鳴ること"
        );
    }

    #[test]
    fn r7_allows_error_mapping_clocks_and_id_generation() {
        // `std::io` 単独はエラー型の写像、`std::time` は Clock の材料 (アダプタ層の機構)、
        // `Uuid::now_v7()` は集約内採番 (オーナー裁定 2026-09-02) — いずれも正当。
        let source = r#"
use std::io;
use std::time::SystemTime;
use uuid::Uuid;

fn stamp() -> String {
    let _ = SystemTime::now();
    let _: Option<io::Error> = None;
    uuid::Uuid::now_v7().as_hyphenated().to_string()
}

fn generate() -> Uuid {
    Uuid::now_v7()
}
"#;
        assert!(check(DOMAIN_PATH, source).is_empty());
    }

    #[test]
    fn r7_does_not_mistake_a_local_binding_for_a_random_crate() {
        // 式の中の 1 セグメントは変数名でもありうる — 完全修飾パス側は 2 セグメント要求。
        let source = r#"
fn pick(rand: u8) -> u8 {
    rand + 1
}
"#;
        assert!(check(DOMAIN_PATH, source).is_empty());
    }

    #[test]
    fn r7_reports_one_finding_per_line() {
        let source = r#"
fn copy(a: &str, b: &str) { let _ = std::fs::read_to_string(a); let _ = std::fs::write(b, ""); }
"#;
        assert_eq!(
            rules(&check(DOMAIN_PATH, source)),
            vec![RULE_COMMAND_SIDE_IO],
            "同一行に複数の I/O があっても所見は 1 件"
        );
    }

    #[test]
    fn r7_ignores_io_paths_mentioned_in_doc_comments() {
        // doc は属性 (`#[doc = "..."]`) なので走査から外れる — R5 と同じ扱い。
        let source = r#"
//! この層では `std::fs` を使わない (gateway-taxonomy §1d)。

/// `std::process::Command` を直接叩かず Repository 実装へ寄せる。
fn note() {}
"#;
        assert!(check(DOMAIN_PATH, source).is_empty());
    }

    #[test]
    fn r7_is_suppressed_by_a_matching_allow_comment_with_reason() {
        let source = r#"
// amadeus-lint: allow(command-side-io) — 合成ルートへ移すまでの暫定 (次 Bolt で解消)
use std::fs;
"#;
        assert!(check(COMMAND_USE_CASE_PATH, source).is_empty());

        let bare = r#"
// amadeus-lint: allow(command-side-io)
use std::fs;
"#;
        assert_eq!(
            rules(&check(COMMAND_USE_CASE_PATH, bare)),
            vec![RULE_COMMAND_SIDE_IO],
            "理由の無い裸の allow は抑制しない"
        );
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
