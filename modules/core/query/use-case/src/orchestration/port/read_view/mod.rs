//! 構造化リードモデル (`read_*` 表) の行を写すビュー型 — b43 の DTO 族。
//!
//! `workflow_view` / `execution_view` が配布 3 入力と状態ファイルという**ファイル面**の
//! リードモデルを写すのに対し、本モジュールは RMU が計算結果まで作った**非正規化リード
//! モデル**の行を写す (`coding-rules/cqrs-boundaries.md` 規則 6 の 2026-09-02 追記)。
//!
//! # 基本データ型で持つ
//!
//! フィールドは列そのままの `String` / `u32` / `bool` / `Option<..>` である。検証付きの
//! newtype (`StageSlugView` 等) を使わないのは、行を書いたのが**検証済みのドメイン型を
//! 持つ RMU** だからであり、読み直しで再検証する経路を作ると「引けなかった」以外の失敗が
//! ポート面に増える (読取エラーは [`ReadModelReadError`] 1 本 — 設計 §2)。JSON 配列の列も
//! 1 行 JSON の文字列のまま運ぶ — 配列へ開くのは描く側である。
//!
//! # `id` と FK 列は「たどる必要があるとき」に載せる
//!
//! View は 1 表 1 行の写しであり、関連は結合ではなく **FK 列**で表す (オーナー裁定
//! 2026-09-03 — `coding-rules/cqrs-boundaries.md` 規則 6)。したがって
//! [`RunStageView`] / [`SteeringPlanView`] は主キー `id` を運び ([`NextAnswerView`] と
//! [`RunStageView`] の FK が指す先だから)、[`NextAnswerView`] / [`RunStageView`] /
//! [`SteeringPartView`] は FK 列を運ぶ (ユースケースがそれを次の鍵にするから)。要求の鍵で
//! 直接引ける表 ([`JumpView`] / [`ScopeView`] など) は `id` を載せない — 誰も指さない
//! 代理キーは読み手の役に立たない。
//!
//! # 判断メソッドを持たない
//!
//! View は「行の写し」であって判断の場所ではない (設計 §0-5)。述語・導出・文言組立は
//! いずれもプレゼンタ側に置く。
//!
//! 型ファイルの mod も本モジュール自身も private。公開 API は親 (`port` → `orchestration`)
//! が中継する `pub use` が唯一の宣言である (`coding-rules/module-visibility.md`)。
//!
//! [`ReadModelReadError`]: super::ReadModelReadError

mod definition_summary_view;
mod execution_view;
mod jump_phase_view;
mod jump_view;
mod next_answer_view;
mod phase_entry_view;
mod run_stage_view;
mod scope_change_view;
mod scope_view;
mod steering_part_view;
mod steering_plan_view;

pub use definition_summary_view::DefinitionSummaryView;
pub use execution_view::ExecutionView;
pub use jump_phase_view::JumpPhaseView;
pub use jump_view::JumpView;
pub use next_answer_view::NextAnswerView;
pub use phase_entry_view::PhaseEntryView;
pub use run_stage_view::RunStageView;
pub use scope_change_view::ScopeChangeView;
pub use scope_view::ScopeView;
pub use steering_part_view::SteeringPartView;
pub use steering_plan_view::SteeringPlanView;
