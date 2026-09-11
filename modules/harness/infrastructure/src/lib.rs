//! harness 文脈の infrastructure 層 — **言語拡張の受け皿**（憲章のみ。実体は U7 以降）。
//!
//! # 何を置くクレートか
//!
//! harness（Claude Code などの実行ハーネス）側で必要になる**汎用機構**だけを置く
//! (`coding-rules/infrastructure-layer.md`)。判定基準は 1 つ —
//! **その部品は相手方システムの契約を知るか**。知らずに標準ライブラリを汎用に延長するだけ
//! なら infrastructure、知るなら gateway であり interface-adapter 層に属する。
//!
//! | 置く（言語拡張） | 置かない（gateway → interface-adapter） |
//! |---|---|
//! | プロセス起動・stdio 配線の薄いプリミティブ | ハーネスの JSON プロトコルを組み立てる Presenter |
//! | 環境変数・設定の読取機構 | フックの発火条件を知る Controller |
//! | 計測・ロギングの配管 | RPC クライアント・DB アクセス（オーナー明言で禁止） |
//!
//! # なぜ空のまま置くのか
//!
//! `core-infrastructure` と対になる**文脈ごとの infrastructure** という配置規則を、実体が
//! 生まれる前に固定しておくためである（2026-08-29 オーナー裁定）。置き場が無いと、最初に
//! 必要になった機構が `harness-claude`（アダプタ層）へ紛れ込み、後から剥がすことになる。
//!
//! 依存方向: infrastructure は domain / use-case / interface-adapter を**知らない**。逆は
//! どの層から依存してもよい。依存は言語拡張のcore-infrastructureと正規表現ライブラリに限定する。

#![forbid(unsafe_code)]

mod shell_parse_error;
mod shell_text;
pub use shell_parse_error::ShellParseError;
pub use shell_text::ShellText;

mod shell_pattern;
pub use shell_pattern::compile_shell_pattern;

mod shell_segments;
mod shell_words;
pub use shell_segments::split_shell_segments;
pub use shell_words::ShellWords;

mod shell_invocation;
pub use shell_invocation::ShellInvocation;

// シェルコマンドが書き換えうるファイルの抽出。字句解析の 4 部品は crate 内部に閉じ、
// 公開するのは結果の列だけである。上の `ShellWords` / `split_shell_segments` /
// `ShellInvocation` とは**別の上流関数の移植で字句規則も違う**ので、混ぜて使わないこと
// (差分は各型の doc に書いてある)。
mod command_segments;
mod redirection_free_words;
mod shell_mutation;
mod shell_options;
mod shell_write_targets;
pub use shell_write_targets::ShellWriteTargets;

// reviewer-scope (§12a の読み取り範囲) が読むシェルコマンドの字句。上の 2 系統とは
// **また別の上流関数の移植で字句規則も違う** (差は `ReviewerScopeWords` の doc の表)。
mod reviewer_scope_segments;
mod reviewer_scope_words;
pub use reviewer_scope_segments::ReviewerScopeSegments;
pub use reviewer_scope_words::ReviewerScopeWords;

mod heredoc_substitutions;
mod shell_substitutions;
pub use heredoc_substitutions::heredoc_substitution_bodies;
pub use shell_substitutions::ShellSubstitutions;
