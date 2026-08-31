//! orchestration コンテキストの**クエリ側** — 「次に何が起こるか」を読むだけの動詞
//! (`next` / `continue`) と、その出力語彙 (directive プロトコル)。
//!
//! # なぜクエリ側にあるのか
//!
//! `next` / `continue` は directive を放出するだけで**何も書かない**。「ただ読むための責務は
//! コマンド側では許容されない — それはクエリ側の実装である」(オーナー裁定 2026-08-30、
//! `coding-rules/cqrs-boundaries.md` 規則 5)。したがって本モジュールは Repository を 1 本も
//! 持たず、集約の再構成もしない (同規則 6)。
//!
//! 読取素材 ([`crate::execution_view::ExecutionStateView`] /
//! [`crate::workflow_view::DefinitionView`] / [`MemoryRules`]) は、リードモデルを読む
//! **DTO/DAO ポート** ([`ExecutionStateDao`] / [`WorkflowDefinitionDao`] /
//! [`MemoryRulesDao`]) 経由で取得する (オーナー裁定 2026-08-31)。ポートは読取動詞 `find`
//! 1 本だけを持ち、更新動詞が存在しないことが「リードモデルは更新できない」の型保証である。
//! **読取元 (ファイル / SQLite のテーブル) は実装の内部詳細**であり、ポート面が語るのは
//! DTO だけである (同日追補裁定)。
//!
//! # 出力モデルであってビューではない
//!
//! ここの型 (directive・continue_token・steering 束縛) はリードモデルの**写し**ではなく、
//! 読み手が**放出する**出力モデルなので `View` 接尾辞を付けない (`workflow_view` /
//! `execution_view` の命名方針と対をなす)。ワイヤに乗る値と綴りは公開言語 (B14) であり、
//! 1 バイトも変えられない。
//!
//! 型ファイルの mod は private。公開 API は以下の `pub use` が唯一の宣言であり、消費側の
//! パスは `core_query_use_case::orchestration::<型>` で安定する
//! (`coding-rules/module-visibility.md`)。

mod continue_token;
mod continue_use_case;
mod directive;
mod directive_schema;
mod engine_command;
mod memory_rules;
mod next_decision;
mod next_turn_input;
mod next_use_case;
mod port;
mod scope_resolution;
mod stage_name;
mod steering_binding;
mod steering_digest;
mod steering_plan;
#[cfg(test)]
mod test_fixtures;
mod token_version;
mod unit_ref;

// directive プロトコル (10 種の閉集合と、構築できる部分集合の判別共用体)
pub use directive::{
    AskDirective, AskKind, Directive, GateField, LoadSteeringDirective, RuleContent,
    RunStageDirective, RunStageDirectiveBuilder,
};
pub use directive_schema::DirectiveKind;

// steering 連鎖 (継続トークン・束縛・配信計画)
pub use continue_token::{ContinueToken, ContinueTokenBuilder};
pub use steering_binding::{Bindings, BundleDigest, DirectiveDigest, RouteDigest, StateBinding};
// steering ダイジェストの導出は所有する型の関連メソッド (steering_digest モジュールの impl —
// `coding-rules/domain-services.md`)。輸出する自由関数は無い。
pub use memory_rules::MemoryRules;
pub use steering_plan::{PartCount, PartIndex, SteeringPart, SteeringPlan};
pub use token_version::TokenVersion;

// エンジンコマンドの概念と綴り
pub use engine_command::{ConfigField, EngineCommand, ReadOnlyVerb};

// 値オブジェクト
pub use stage_name::StageName;
pub use unit_ref::{UnitKind, UnitName, UnitRef};

// 判断の入出力
pub use next_decision::{EngineSignal, NextDecision, NextRequest};
pub use scope_resolution::{ResolvedScope, ScopeSource};

// ポート (trait) — リードモデルを読む DAO。動詞は読取 (`find`) だけで、更新動詞は無い
// (`coding-rules/cqrs-boundaries.md` 規則 6 / `gateway-taxonomy.md` §3 の 2026-08-31 追記)。
pub use port::{ExecutionStateDao, MemoryRulesDao, WorkflowDefinitionDao};

// ユースケース (読取専用 — DAO ポートを保持し、`execute` は `&self` のクエリ)
pub use continue_use_case::ContinueUseCase;
pub use next_turn_input::{NextTurnInput, NounFamily, NounToken, WorkspaceLayout};
pub use next_use_case::NextUseCase;

// 拒否 (ポート面のエラーは材料のみ — 逐語文言は出す側のユースケースが組む)
pub use port::{ExecutionStateReadError, MemoryRulesReadError, WorkflowDefinitionReadError};
pub use scope_resolution::ScopeResolutionError;
pub use stage_name::BlankStageName;
pub use steering_plan::UnsplittableSection;
pub use unit_ref::{UnitNameError, UnknownUnitKind};
