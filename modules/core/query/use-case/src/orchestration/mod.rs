//! orchestration コンテキストの**クエリ側** — 蓄積されたリードモデルを鍵で引く口と、
//! 読み手が放出する出力語彙 (directive プロトコル)。
//!
//! # 読むことしか許されない
//!
//! `next` / `continue` は directive を放出するだけで**何も書かない**。「ただ読むための責務は
//! コマンド側では許容されない — それはクエリ側の実装である」(オーナー裁定 2026-08-30、
//! `coding-rules/cqrs-boundaries.md` 規則 5)。したがって本モジュールは Repository を 1 本も
//! 持たず、集約の再構成もしない (同規則 6)。
//!
//! さらに**判断・導出・選択・文言組立のどれも持たない** (オーナー裁定 2026-09-02)。
//! ユースケースの本体は `execute(鍵) = dao.find(鍵) → View` であり、複数の表にまたがる答えは
//! **FK をたどって表ごとに引く**ことで組む (同 2026-09-03 — DAO は 1 表 1 引当)。何を描くかを
//! 決めるのは行の綴り (`read_next_answer.decision_kind`) であり、逐語文言・directive の綴り・
//! token の封緘は出す側 (合成ルートのプレゼンタ) の仕事である。
//!
//! b26 / b27 が持っていた 21 分岐ラダー・スコープ解決・steering 分割・Markdown 逆パースは
//! b44 で**すべて撤去**した (`query-side-audit/read-model-spec.md` §7)。
//!
//! # 読取素材は 12 の DAO ポート
//!
//! リードモデルを読む **DTO/DAO ポート** (`port/`) 経由で取得する (オーナー裁定 2026-08-31)。
//! ポートは読取動詞 `find` 1 本だけを持ち、更新動詞が存在しないことが「リードモデルは
//! 更新できない」の型保証である。**読取元 (SQLite の `read_*` 表) は実装の内部詳細**であり、
//! ポート面が語るのは DTO だけである (同日追補裁定)。DTO 自身も DAO と同じ `port/` に住む。
//!
//! # 出力モデルであってビューではない
//!
//! directive・continue_token・束縛はリードモデルの**写し**ではなく、読み手が**放出する**
//! 出力モデルなので `View` 接尾辞を付けない。ワイヤに乗る値と綴りは公開言語 (B14) であり、
//! 1 バイトも変えられない。
//!
//! その出力モデルが使う小さな値 ([`StageSlugView`] / [`ScopeSlugView`] / [`PhaseView`] /
//! [`StageModeView`] / [`ReviewClassView`]) は `View` 接尾辞を保つ — 綴りの出所は行
//! (`read_run_stage.phase` 等) であり、検証済みの値だけを保持する読取側の語彙だからである。
//!
//! 型ファイルの mod は private。公開 API は以下の `pub use` が唯一の宣言であり、消費側の
//! パスは `core_query_use_case::orchestration::<型>` で安定する
//! (`coding-rules/module-visibility.md`)。

mod ask_directive;
mod ask_kind;
mod bindings;
mod blank_stage_name;
mod bundle_digest;
mod continuation_view;
mod continue_token;
mod directive;
mod directive_digest;
mod directive_schema;
mod engine_command;
mod find_continuation_use_case;
mod find_definition_use_case;
mod find_execution_use_case;
mod find_jump_use_case;
mod find_next_answer_use_case;
mod find_phase_entry_use_case;
mod find_run_stage_use_case;
mod find_scope_change_use_case;
mod find_scope_keyword_use_case;
mod find_scope_use_case;
mod find_steering_use_case;
mod gate_field;
mod load_steering_directive;
mod next_turn_input;
mod next_turn_view;
mod noun_family;
mod noun_token;
mod part_count;
mod part_index;
mod phase_view;
mod port;
mod read_only_verb;
mod review_class_view;
mod route_digest;
mod rule_content;
mod run_stage_directive;
mod scope_slug_error;
mod scope_slug_view;
mod stage_mode_view;
mod stage_name;
mod stage_slug_error;
mod stage_slug_view;
mod state_binding;
mod steering_delivery_view;
mod token_version;
mod unit_kind;
mod unit_name;
mod unit_name_error;
mod unit_ref;
mod unknown_unit_kind;
mod unknown_value;

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

// steering 連鎖 (継続トークン・束縛・部の番号)
pub use bindings::Bindings;
pub use bundle_digest::BundleDigest;
pub use continue_token::{ContinueToken, ContinueTokenBuilder};
pub use directive_digest::DirectiveDigest;
pub use part_count::PartCount;
pub use part_index::PartIndex;
pub use route_digest::RouteDigest;
pub use state_binding::StateBinding;
pub use token_version::TokenVersion;

// エンジンコマンドの概念と綴り
pub use engine_command::EngineCommand;
pub use read_only_verb::ReadOnlyVerb;

// 値オブジェクト — 出力モデルが使う検証済みの値
pub use phase_view::PhaseView;
pub use review_class_view::ReviewClassView;
pub use scope_slug_view::ScopeSlugView;
pub use stage_mode_view::StageModeView;
pub use stage_name::StageName;
pub use stage_slug_view::StageSlugView;
pub use unit_kind::UnitKind;
pub use unit_name::UnitName;
pub use unit_ref::UnitRef;

// 要求の観測 (合成ルートが argv から畳む — 判断は持たない)
pub use next_turn_input::NextTurnInput;
pub use noun_family::NounFamily;
pub use noun_token::NounToken;

// ポート (trait) — 構造化リードモデル (`read_*` 表) を引く 12 の DAO。**1 表 1 ポート**で、
// 動詞は読取 (`find`) だけ (`coding-rules/cqrs-boundaries.md` 規則 6 /
// `gateway-taxonomy.md` §3 の 2026-08-31 追記)。
pub use port::{
    DefinitionDao, ExecutionDao, JumpDao, JumpPhaseDao, NextAnswerDao, PhaseEntryDao, RunStageDao,
    ScopeChangeDao, ScopeDao, ScopeKeywordDao, SteeringPartDao, SteeringPlanDao,
};

// 行の写し (1 表 1 View)。
pub use port::{
    DefinitionSummaryView, ExecutionView, JumpPhaseView, JumpView, NextAnswerView, PhaseEntryView,
    RunStageView, ScopeChangeView, ScopeView, SteeringPartView, SteeringPlanView,
};

// 複数の表にまたがる答えの**組み立て View**。DAO が返す型ではない (ユースケースが FK を
// たどって組む) ので `port/` ではなくここに住む。
pub use continuation_view::ContinuationView;
pub use next_turn_view::NextTurnView;
pub use steering_delivery_view::SteeringDeliveryView;

// ユースケース (読取専用 — DAO ポートを保持し、`execute` は `&self` のクエリ)。
pub use find_continuation_use_case::FindContinuationUseCase;
pub use find_definition_use_case::FindDefinitionUseCase;
pub use find_execution_use_case::FindExecutionUseCase;
pub use find_jump_use_case::FindJumpUseCase;
pub use find_next_answer_use_case::FindNextAnswerUseCase;
pub use find_phase_entry_use_case::FindPhaseEntryUseCase;
pub use find_run_stage_use_case::FindRunStageUseCase;
pub use find_scope_change_use_case::FindScopeChangeUseCase;
pub use find_scope_keyword_use_case::FindScopeKeywordUseCase;
pub use find_scope_use_case::FindScopeUseCase;
pub use find_steering_use_case::FindSteeringUseCase;

// 拒否 (ポート面のエラーは材料のみ — 逐語文言は出す側が組む)
pub use port::ReadModelReadError;
// 拒否 (値の復号 — ビューではないので `View` 接尾辞を付けない)
pub use blank_stage_name::BlankStageName;
pub use scope_slug_error::ScopeSlugError;
pub use stage_slug_error::StageSlugError;
pub use unit_name_error::UnitNameError;
pub use unknown_unit_kind::UnknownUnitKind;
pub use unknown_value::UnknownValue;
