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
//! 読取素材 ([`ExecutionStateView`] / [`DefinitionView`] / [`MemoryRules`]) は、リードモデルを
//! 読む **DTO/DAO ポート** ([`ExecutionStateDao`] / [`WorkflowDefinitionDao`] /
//! [`MemoryRulesDao`]) 経由で取得する (オーナー裁定 2026-08-31)。ポートは読取動詞 `find`
//! 1 本だけを持ち、更新動詞が存在しないことが「リードモデルは更新できない」の型保証である。
//! **読取元 (ファイル / SQLite のテーブル) は実装の内部詳細**であり、ポート面が語るのは
//! DTO だけである (同日追補裁定)。
//!
//! その DTO 自身も DAO と同じ `port/` に住む — 「Port の Dao が依存する型も port/ にいれて。
//! `*View`」(オーナー裁定 2026-08-31)。DTO/DAO ポートは一つのパッケージであり、契約とその
//! 契約が返す型は同じ理由で変わるので、変更の単位を 1 ディレクトリに揃える。
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

mod ask_directive;
mod ask_kind;
mod bindings;
mod blank_stage_name;
mod bundle_digest;
mod continue_token;
mod continue_use_case;
mod directive;
mod directive_digest;
mod directive_schema;
mod engine_command;
mod engine_signal;
mod gate_field;
mod load_steering_directive;
mod next_decision;
mod next_request;
mod next_turn_input;
mod next_use_case;
mod noun_family;
mod noun_token;
mod part_count;
mod part_index;
mod port;
mod read_only_verb;
mod route_digest;
mod rule_content;
mod run_stage_directive;
mod scope_resolution;
mod scope_resolution_error;
mod scope_source;
mod stage_name;
mod state_binding;
mod steering_digest;
mod steering_part;
mod steering_plan;
#[cfg(test)]
mod test_fixtures;
mod token_version;
mod unit_kind;
mod unit_name;
mod unit_name_error;
mod unit_ref;
mod unknown_unit_kind;
mod unsplittable_section;
mod workspace_layout;

// directive プロトコル (10 種の閉集合と、構築できる部分集合の判別共用体)
pub use ask_directive::AskDirective;
pub use ask_kind::AskKind;
pub use directive::Directive;
pub use directive_schema::DirectiveKind;
pub use gate_field::GateField;
pub use load_steering_directive::LoadSteeringDirective;
pub use rule_content::RuleContent;
// ビルダーは対象型の所有サブツリー (`run_stage_directive/`) に住み、型ファイル自身が
// ファサード連鎖の一段を担う (`coding-rules/module-visibility.md` §追記 2026-09-01)。
pub use run_stage_directive::{RunStageDirective, RunStageDirectiveBuilder};

// steering 連鎖 (継続トークン・束縛・配信計画)
pub use bindings::Bindings;
pub use bundle_digest::BundleDigest;
pub use continue_token::{ContinueToken, ContinueTokenBuilder};
pub use directive_digest::DirectiveDigest;
pub use route_digest::RouteDigest;
pub use state_binding::StateBinding;
// steering ダイジェストの導出は所有する型の関連メソッド (steering_digest モジュールの impl —
// `coding-rules/domain-services.md`)。輸出する自由関数は無い。
pub use part_count::PartCount;
pub use part_index::PartIndex;
pub use steering_part::SteeringPart;
pub use steering_plan::SteeringPlan;
pub use token_version::TokenVersion;

// エンジンコマンドの概念と綴り
pub use engine_command::EngineCommand;
pub use read_only_verb::ReadOnlyVerb;

// 値オブジェクト
pub use stage_name::StageName;
pub use unit_kind::UnitKind;
pub use unit_name::UnitName;
pub use unit_ref::UnitRef;

// 判断の入出力
pub use engine_signal::EngineSignal;
pub use next_decision::NextDecision;
pub use next_request::NextRequest;
pub use scope_resolution::ResolvedScope;
pub use scope_source::ScopeSource;

// ポート (trait) — リードモデルを読む DAO。動詞は読取 (`find`) だけで、更新動詞は無い
// (`coding-rules/cqrs-boundaries.md` 規則 6 / `gateway-taxonomy.md` §3 の 2026-08-31 追記)。
pub use port::{ExecutionStateDao, MemoryRulesDao, WorkflowDefinitionDao};

// ポートの DTO — DAO が返すクエリモデル。DAO と同じ `port/` に同居する (オーナー裁定
// 2026-08-31 — DTO/DAO ポートは一つのパッケージ)。読む対象は 2 族あるが、消費側のパスは
// 本ファサードで平坦に揃う。
//
// ワークフロー定義リードモデル (3 入力) のビュー型
pub use port::{
    BrownfieldGreenfieldView, ConsumeDeclView, DefinitionIdView, DefinitionRevisionView,
    DefinitionView, ExecutionKindView, PhaseView, PlanActionView, ReviewCapValueView,
    ReviewClassView, RuleInContextView, RuleScopeView, ScopeGridView, ScopeMetadataView,
    ScopeSlugView, SensorRefView, SkeletonDefaultView, StageGraphView, StageModeView,
    StageNumberView, StageRouteView, StageSlugView, StageView, StageViewBuilder,
};
// 実行状態リードモデル (`aidlc-state.md`) のビュー型と、その上の判断 (BR3.1 の 8 分岐)
pub use port::{CheckboxState, ExecutionStateView, ExecutionStatus, StageIndex, StageProgressView};
// memory 層ルール束 (`MemoryRulesDao` の戻り値 — リードモデルの写しではないので `View` 無し)
pub use port::MemoryRules;

// ユースケース (読取専用 — DAO ポートを保持し、`execute` は `&self` のクエリ) と、その観測
pub use continue_use_case::ContinueUseCase;
pub use next_turn_input::NextTurnInput;
pub use next_use_case::NextUseCase;
pub use noun_family::NounFamily;
pub use noun_token::NounToken;
pub use workspace_layout::WorkspaceLayout;

// 拒否 (ポート面のエラーは材料のみ — 逐語文言は出す側のユースケースが組む)
pub use port::{ExecutionStateReadError, MemoryRulesReadError, WorkflowDefinitionReadError};
// 拒否 (DTO の復号 — ビューではないので `View` 接尾辞を付けない)
pub use blank_stage_name::BlankStageName;
pub use port::{
    DefinitionIdError, DefinitionRevisionError, ExecutionStateError, ScopeMetadataError,
    ScopeSlugError, StageGraphError, StageNumberError, StageSlugError, UnknownScope, UnknownValue,
};
pub use scope_resolution_error::ScopeResolutionError;
pub use unit_name_error::UnitNameError;
pub use unknown_unit_kind::UnknownUnitKind;
pub use unsplittable_section::UnsplittableSection;
