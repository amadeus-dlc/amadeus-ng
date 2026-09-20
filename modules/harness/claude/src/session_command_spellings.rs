//! SessionStartの案内文が名指すエンジンコマンドの綴り。
//!
//! 出典は 2.8.2 の `.claude/hooks/aidlc-session-start.ts` のグラフ乖離通知 (`:359`) と
//! 転送ループの規律 (`:377`・`:379`) である。綴りの規則はクエリ側が所有し、このクレートは
//! そこへ依存しない (`coding-rules/cqrs-boundaries.md`) ので、合成ルートが組んで渡す。

/// SessionStartの案内文が名指すエンジンコマンドの綴り。
///
/// 綴りの規則はクエリ側が所有し、このクレートはそこへ依存しない
/// (`coding-rules/cqrs-boundaries.md`) ので、合成ルートが組んで渡す。
/// 出典は 2.8.2 の `.claude/hooks/aidlc-session-start.ts:359`・`:377`・`:379`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionCommandSpellings {
    orchestrate: String,
    jump_execute: String,
    graph_compile: String,
}
impl SessionCommandSpellings {
    /// 合成ルートが組んだ全綴りから構築する。
    #[must_use]
    pub const fn new(orchestrate: String, jump_execute: String, graph_compile: String) -> Self {
        Self {
            orchestrate,
            jump_execute,
            graph_compile,
        }
    }
    /// 次の一手を決める唯一の経路として規律が名指すコマンド (`:377`)。
    #[must_use]
    pub fn orchestrate(&self) -> &str {
        &self.orchestrate
    }
    /// 印字指示が名指すコマンドの例として規律が挙げるジャンプ実行 (`:379`)。
    #[must_use]
    pub fn jump_execute(&self) -> &str {
        &self.jump_execute
    }
    /// グラフ乖離通知が案内する再コンパイル (`:359`)。
    #[must_use]
    pub fn graph_compile(&self) -> &str {
        &self.graph_compile
    }
}
