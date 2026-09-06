# logical-components — U2 ドメイン ES コア（`u2-domain-es-core`）

> NFR Design（Construction 3.3）成果物（Unit: U2、kind: library）。**2026-09-07 再走（Modify）** — 2026-08-23 の初版
> （`modules/core/domain/` 配置、`WorkflowExecution`、B3 の `find_by_id` 範囲拡張）を現行コード `modules/core/command/domain/`
> と 2026-09-05 是正・2026-09-07 再走後の機能設計（BR5.5 FCC 化、Q5 `IntentMismatch`、§9 引継ぎ）へ同期した。
> 出典: `../nfr-requirements/security-requirements.md`（NFR1.1〜NFR4.5、末尾レビュー R-01〜R-08）、`../nfr-requirements/tech-stack-decisions.md`
> （§1 FCC 行・§2 依存の差分・§3 未決）、`../functional-design/functional-spec.md`（§2 API、§9 引継ぎ、末尾レビュー R-01〜R-10）、
> `../functional-design/entities.md`（FCC 型の不変条件・操作）、`../functional-design/rules.md`（BR4.1 / BR5.1〜BR5.5）、
> `../../../inception/contract-design/contract-summary.md`（C3 / C5 / C6）、`../../../inception/domain-design/components.md`、
> `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/`（module-visibility / first-class-collections / error-handling）、
> 確認事項 `nfr-design-questions.md`（P5〜P11、Looks correct）。
>
> 本 Unit はインフラを持たないため、「論理コンポーネント」= `core-command-domain` クレート内のモジュール境界、新設 FCC の置き場、
> 兄弟クレートへ及ぶ追随の一覧、テスト支援の置き場。用語: **FCC** = ファーストクラスコレクション（配列を不変条件と操作を持つ専用型で
> 包んだもの）、**DTO** = アダプタ層が保存・復元に使う写し、**RMU** = read-model-updater（イベントからリードモデルを投影するクレート）。

## 1. コンポーネント一覧

現行配置は `modules/core/command/domain/src/` の 3 文脈（`orchestration/` 53 エントリ = 51 ファイル + `intent_event/` /
`intent_execution_event/` の 2 ディレクトリ、`workflow_definition/`、`workspace/`。質問票 P7 の「55 ファイル」は概数で、実測はこの値）。
型ファイル mod はすべて private、公開は各文脈直下 `mod.rs` の `pub use` 列挙のみ（module-visibility、`unreachable_pub` deny で漏れは
ビルドエラー）。「本再走の変更」列が空の行は U2 の code-generation 再走で触らない。

| コンポーネント | 置き場（`src/` 基準） | 責務 | 公開面 | 依存 | 本再走の変更 |
|---|---|---|---|---|---|
| `intent` | `orchestration/intent.rs` | `Intent` 集約（依頼・`definition_id` / `definition_revision` の来歴・静的計画）。`create(..) -> (Intent, IntentEvent)`（`IntentError`）、`replay(events, snapshot)`、`resolve_review_policy(&WorkflowDefinition, &StageSlug)`（`IntentReviewError::DefinitionMismatch`） | `Intent` | stage_entry, start_request, workspace_scan, `workflow_definition::{WorkflowDefinition, WorkflowDefinitionId, DefinitionRevision, ReviewPolicy, StageSlug}` | `stages()`（`:260`、現行 `&[StageEntry]`）→ `&StageEntries`。計画検査 `StageEntry::check_plan(&[StageEntry])`（`stage_entry.rs:100`）を `StageEntries` の構築検査へ移す |
| `intent_execution` | `orchestration/intent_execution.rs` | 実行 FSM の集約ルート — 16 コマンド、`apply_event(seq_nr, t, &event)`（`:1542`、`# Panics`）、`replay(snapshot, delta)`（`:385`、`# Panics`）、`new(..) -> Result<_, IntentExecutionError>`（`:283`、DTO 境界の検査点）、書込前ガード `jump_resolve` / `stale_report`、`next_decision`（`:1897`）、`state_binding`、`stage_index(usize) -> Option<StageIndex>`（`:464`）、PBT 同居 | `IntentExecution`, `Status` | intent, 16 イベント型, stage_index / stage_key, next_request / next_decision / engine_signal / gate_decision, report_*, review_attempt, エラー型, `workspace::{CheckboxState, HumanTurns, PracticesPromotion}`, `workflow_definition::{PlanAction, ReviewPolicy, StageSlug, PhaseId}` | 7 並列列（`stage_keys` / `overlay` / `checkbox` / `review_attempts` / `practices_affirmed` / `approved` / `revision_count`）→ `slots: StageSlots`。`stage_keys() -> &[StageKey]`（`:441`）を廃止し `slots()` / `stage_key(StageIndex)` へ。`open_gate(.., Vec<String>, ..)`（`:821`）→ `ArtifactPaths`、`recompose(.., &[StageIndex], ..)`（`:1060`）→ `StageIndexSet`、`apply_report(.., &[TransitionStep], ..)`（`:2024`）→ `&TransitionSteps`、`next_decision` → `Result<NextDecision, CommandError>`（`IntentMismatch`）。冒頭 doc（`:35`「`# Panics` を持つ公開 API は無い」「memento」）を実態へ修正 |
| `intent_event` / `intent_execution_event` | `orchestration/intent_event/`（`Created` 1 変種）、`orchestration/intent_execution_event/`（16 ファイル）+ 各 `*.rs` の enum | ドメインイベント（エンティティ — 自前の UUIDv7 ID と `aggregate_id`）。ペイロードは entities.md が正本 | `IntentEvent`, `Created`, `IntentExecutionEvent`, 16 ペイロード型 | stage_entry / stage_key, `workflow_definition::StageSlug`, `workspace::PromotedSection`, autonomy_mode, skeleton_stance, review_* | 列の FCC 化: `Created.stages`（`created.rs:87`）/ `Started.stages`（`started.rs:68`）→ `StageEntries`、`GateOpened.artifacts`（`gate_opened.rs:40`）→ `ArtifactPaths`、`Recomposed.skipped` / `added`（`recomposed.rs:37,43`）→ `StageSlugSet`、`PracticesAffirmed.sections` / `mandated` / `forbidden`（`practices_affirmed.rs:61,67,73`）→ `PromotedSections` / `RuleLines` |
| ID 型 | `orchestration/{intent_id, intent_execution_id, intent_event_id, intent_execution_event_id}.rs`（+ `*_error.rs`） | UUIDv7 の検査付き newtype。`generate()`（`Uuid::now_v7`）が時計を読む唯一の箇所（NFR3.1 の例外） | 4 型 + 4 エラー型 | uuid | — |
| 位置・計画の値型 | `orchestration/{stage_index, stage_entry, stage_key, stage_display}.rs` | `StageIndex`（範囲を型保証、`Copy` + `Ord`、構築は集約と DTO 境界の検査付き `new`）、`StageEntry`（slug / phase / plan_action / conditional / display）、`StageKey`（slug / phase）、`StageDisplay` | 4 型 | `workflow_definition::{StageSlug, PhaseId, PlanAction}` | `StageEntry::check_plan` の移設（上記）。`is_gated` は不変 |
| **新設 FCC（orchestration）** | `orchestration/{stage_entries, stage_slot, stage_slots, stage_index_set, artifact_paths, stage_slug_set, transition_steps}.rs`（private）+ 型ごとの `*_error.rs` | BR5.5。`StageEntries`（非空・slug 一意・文書順。`first_of(phase, action)` / `position_of(slug)`）、`StageSlot`（位置 1 つの記録: key / plan_action / checkbox / approved / revision_count / review_attempt / practices_affirmed）、`StageSlots`（位置ごとの `StageSlot`、長さ = stage_count、`at(StageIndex)` / 位置単位の置換 / 受領証の一括リセット）、`StageIndexSet`（位置集合、`range` / `combine` = 和集合 / `divide` = 差集合、空集合を単位元とする Monoid）、`ArtifactPaths`（成果物パスの列、素通し）、`StageSlugSet`（辞書順・重複なし — R-03 の是正）、`TransitionSteps`（`ReportDecision::Commit.steps` と `apply_report` の入力、`contains(TransitionStep)` 相当の業務操作 — 定義は R-01） | ファサードから `pub use`（`StageSlot` は `StageSlots.at` の戻り値として公開） | `core_infrastructure::collections::FirstClassCollection`（`len` / `at` / `fold_left` / `filter`）、要素型 | 新設。`filter` / `divide` の結果型（空可の型 — R-04）と `TransitionSteps` の不変条件（R-01）は functional-design ゲートの Request Changes で確定し、code-generation の計画に先行して載せる |
| **新設 FCC（workspace）** | `workspace/{promoted_sections, rule_lines}.rs`（private）+ `*_error.rs` | `PromotedSections`（`PromotedSection` の列、見出し一意）、`RuleLines`（規則行の列、素通し・順序保持）。`PracticesPromotion` の内部列（`practices_promotion.rs:113,119,125` の `&[..]` 公開）も同型へ | `workspace::{PromotedSections, RuleLines}` | `FirstClassCollection`、`PromotedSection` | 新設。要素型 `PromotedSection` を所有する文脈に置く。`orchestration` → `workspace` の参照方向は既存どおり（`workspace` は `orchestration` を参照しない — 実測 0 件） |
| `review_attempt` | `orchestration/review_attempt.rs` | ステージごとのレビュー会計（`requests` / `pending: BTreeSet<u32>` / `closed: Vec<ReviewClosure>`）。`is_pending` / `has_terminal` / `closed()`（`:66`、`&[ReviewClosure]`） | `ReviewAttempt`, `ReviewClosure`, `ReviewVerdict` | review_closure, `workflow_definition::ReviewPolicy` | 内部列の FCC 名・不変条件は機能設計レビュー R-01 の未決（functional-design ゲートで確定）。置き場は本ファイル隣接の private mod、型はファサード公開せず `ReviewAttempt` の操作経由とする（仮置き、P7）。`StageSlot.review_attempt` として `StageSlots` に畳み込む |
| 判断・決定型 | `orchestration/{next_request, next_decision, engine_signal, gate_decision, report_request, report_decision, report_no_op, report_refusal, state_binding, transition_step, verdict, review_verdict, skeleton_stance, autonomy_mode, jump_direction, phase_boundary, start_request, workspace_scan}.rs` | `NextRequest` / `NextDecision` / `EngineSignal` 導出、`ReportRequest`（`for_retry_at`）、`ReportDecision::{Commit{stage, steps, scope}, NoOp}`、`StateBinding`、`TransitionStep`、`Verdict` ほか | 各型 | stage_index, `workspace::CheckboxState`, `workflow_definition::StageSlug` | `ReportDecision::Commit.steps: Vec<TransitionStep>` → `TransitionSteps` |
| エラー型 | `orchestration/{command_error, intent_error, plan_error, intent_execution_error, report_commit_error, report_refusal, single_stage_run_refusal, skeleton_stance_refusal, intent_review_error, invalid_mode_arg, unknown_*}.rs`、`apply_error.rs`（`pub(crate)`） | 手実装 enum（`IntentExecutionError` は理由文字列の struct）+ `fmt::Display`（材料のみ）+ `std::error::Error`。`CommandError` は `IntentMismatch` を含む 17 変種 | 各型（`ApplyError` は非公開） | stage_index, `workspace::CheckboxState`, `workflow_definition::*` | FCC ごとの構築・`combine` / `map` 衝突用 Error を新設（house style、thiserror / anyhow 不使用）。既存型の変種は増やさない |
| `workflow_definition/` 文脈 | `workflow_definition/*.rs` | `WorkflowDefinition` / `CompiledDefinition` 集約、`PlanAction`、`StageSlug`、`PhaseId`、`WorkflowDefinitionId` / `DefinitionRevision`、`ReviewPolicy`、`StageGraph` / `ScopeGrid`（既存 FCC）、`LineageMismatch` | 既存どおり | — | — （U2 の再走で触らない。BR4.1 の `PlanAction` 所在はここ） |
| `workspace/` 文脈（既存） | `workspace/*.rs` | `CheckboxState` / `Checkboxes` / `HumanTurns` / `PracticesPromotion` / `PromotedSection` / `BoltRefs` / `AuditFields` / `OrderedAuditEvents` ほか | 既存どおり | — | `PracticesPromotion` の列を `PromotedSections` / `RuleLines` へ（上記） |
| ファサード | `orchestration/mod.rs` / `workspace/mod.rs` | 公開 API の `pub use` 列挙のみ（利便再エクスポート無し） | 上記の公開型 | — | 新設 FCC と Error 型の `pub use` 追加。`orchestration/mod.rs` 冒頭 doc の修正（§9 #4: 「ジャーナル全再生」→ 最新スナップショット + 差分、「`next_decision` はクエリ側が所有」→ `IntentExecution::next_decision`、`recompose(&[stage])` → `StageIndexSet`） |
| ITF 準拠テスト | `tests/engine_loop_conformance.rs` | Quint `engine_loop.qnt`（v2.7）トレースの再生（decide → apply）、射影表（rules.md 第 3 節）の突合せ、`EngineSignal` の照合 | テストのみ | core-command-domain, serde_json（dev） | 追随: `next_decision`（`:356`、Result）、`open_gate(.., Vec::new(), ..)`（`:449`）、`recompose(.., &[index], ..)`（`:488`）。モデルは不変 |
| 契約試験ハーネス | `tests/collection_contract_test.rs` | 全 FCC の共通契約（`len` / `is_empty` / `at` / `fold_left` / `filter`）の横展開漏れ検査（現行 7 型: BoltRefs / Checkboxes / OrderedAuditEvents / AuditFields / StageGraph / ScopeGrid + infrastructure 側） | テストのみ | core-command-domain, core-infrastructure | 新設 FCC を `check(..)` へ登録（NFR2.5）。登録対象の確定列挙は R-01 の定義確定が前提 |

### 兄弟クレートへ及ぶ追随（U2 の Bolt に含める — P8）

| クレート | 箇所（実測） | 追随の内容 |
|---|---|---|
| `core-command-interface-adapter` | `src/orchestration/dto/intent_dto.rs:85`、`created_dto.rs:47`、`intent_execution_dto.rs:142`、`intent_execution_event_dto.rs:113,121` | `stages().iter().map(..).collect()` / `stage_keys()` / `artifacts().to_vec()` の要素列挙を `fold_left` へ。DTO の列構造（`IntentExecutionDto` の `stages` / `overlay` / `checkbox` / `review_attempts` / `practices_affirmed` / `approved` / `revision_count` の 7 列）と正準 JSON のバイトは不変 — `StageSlots` → 7 列は `fold_left` で展開、7 列 → `StageSlots` は `new` の検査付き変換で畳む |
| `core-read-model-updater` | `src/read_tables.rs:239,284`、`src/read_tables/stage_lookup.rs:23`、`src/workspace/resolved_plan.rs:49`、`src/read_tables/next_answer_row.rs:58` | 列挙を `fold_left` / `at` へ、`next_decision` の Err を投影の束縛不整合として扱う（§9 #2）。RMU は FCC 型を定義・保持せず、読取専用の呼出だけを通す |
| `core-command-use-case` | `src/orchestration/commit_verdict_use_case.rs:212,218`、`src/orchestration/test_support.rs:114,856,889` | `steps.contains(&TransitionStep::Approve)` → `TransitionSteps` の業務操作、`apply_report(.., &steps, ..)` の型、`original.stages().to_vec()` → `StageEntries` の受け渡し |
| `core-command-domain`（テスト） | `tests/engine_loop_conformance.rs:356,449,488`、`tests/collection_contract_test.rs` | 上表のとおり |

## 2. 境界と隔離

- **クレート境界**: `core-command-domain` の依存は不変（runtime = `chrono` / `uuid` / `core-infrastructure`、dev = `proptest` / `serde_json`）。
  serde / canon-json をドメインに入れない — JSON 化と revision 計算は `core-command-interface-adapter`（U3）。`Cargo.lock` 不変が期待値。
- **モジュール境界**: 型ファイル mod はすべて private、公開はファサードの `pub use` のみ。文脈間の参照は `orchestration` → `workflow_definition`
  （`PlanAction` / `StageSlug` / `PhaseId` / `WorkflowDefinition` / `ReviewPolicy` / ID 型）と `orchestration` → `workspace`（`CheckboxState` /
  `HumanTurns` / `PracticesPromotion` / `PromotedSection`、新設 `PromotedSections` / `RuleLines`）の一方向。`workspace` / `workflow_definition`
  から `orchestration` への参照は作らない（実測 0 件を維持）。新設 FCC は要素型を所有する文脈に置く（P7）。
- **集約境界**: `IntentExecution` は `Intent` を保持せず `intent_id` で参照し、`&Intent` を受ける全 API が `matches(intent)`（`intent_execution.rs:417`）
  で照合して `CommandError::IntentMismatch` を返す（`next_decision` は Result 化で同列に並ぶ）。`Intent` は `WorkflowDefinition` を保持せず
  `definition_id` / `definition_revision` で参照し、`resolve_review_policy` が `IntentReviewError::DefinitionMismatch` で照合する。
- **リードモデル・DTO 境界**: FCC は集約の内側で完結し、境界では `fold_left` で平坦な表現（DTO の列、リードモデルの行）へ写す。RMU は
  FCC 型を定義・保持しない（読取専用の `fold_left` / `at` の呼出は除く）。DTO のバイト表現（正準 JSON）は変えないので、ゴールデン・往復
  テストは不変（NFR1.3 / NFR3.3）。
- **越境の裁定（U2 の Bolt が兄弟クレートを触る根拠）**: U2 の宣言境界（`unit-of-work.md`）は `core-command-domain` 内だが、公開 API の
  型変更は §1 末尾の 3 クレートの生産コードを壊す。PlanAction 完全移動の「呼出側パスの一斉修正を同 Unit に含む」先例と、機能設計
  §9 #3 の要約確認（2026-09-07、Looks correct）により、追随改修を U2 の同じ Bolt に含める。機能設計レビュー R-10 / NFR 要求レビュー
  R-03 の未解決はこの記載で閉じ、functional-design ゲートの Request Changes で §9 に同じ 1 行を折り戻す。

## 3. 障害ドメインとブラストラディウス

| 障害 | 影響範囲 | 手当て |
|---|---|---|
| ガード不成立（`CommandError` / `ReportRefusal` / `SingleStageRunRefusal` / `SkeletonStanceRefusal`） | 呼出側の 1 コマンド実行（状態不変、イベントなし） | Err で返す。文言はアダプタ層。ユースケースは中断して上位へ |
| 集約の取り違え（`IntentMismatch`）/ 定義の取り違え（`DefinitionMismatch`） | 1 コマンド / `next` 1 回 / 方針解決 1 回 | Err。別集約での駆動は契約違反として上位へ（U6 / RMU は投影の束縛不整合として扱う） |
| 境界の検査付き変換の失敗（`IntentExecutionError`、`IntentError`、FCC の構築 Err） | 再構成・生成の 1 回（集約は生成されない） | Err。U3 が `RepositoryError::Corrupt` に写す（C3）。DTO 復号の不整合と同じ経路 |
| 型変換後の壊れた歴史（`replay` / `apply_event` の通番飛び・未知ステージ・不変条件違反） | 再構成を行った 1 プロセス（ユースケース / RMU の 1 回）が panic で停止 | 回復しない（オーナー裁定 2026-08-30）。真実源の SQLite ジャーナルが破損している状態であり、進まないのが正。`# Panics` を明記し、復旧手段は U3 の責務 |
| 設計と Quint の乖離 | テスト失敗（リリース前に検出） | ITF 準拠テスト + PBT（実装を直す、モデル v2.7 は不変） |
| FCC の契約違反（共通契約・Monoid 則・連結衝突） | テスト失敗（リリース前に検出） | 契約試験ハーネス + 型ファイル同居の性質試験（NFR2.5） |
| 依存の脆弱性 | ビルド全体 | 依存追加なし。既存 `cargo audit`（U10） |

共有資源: なし（I/O・グローバル状態・時計を持たない。時計は `*EventId::generate` の UUIDv7 採番のみ）。

## 4. テストの配置（NFR2.x）

| 種別 | 置き場 | 内容 |
|---|---|---|
| ユニット（インライン `#[cfg(test)]`） | 各モジュール | ガード境界値（各 Err 変種: 正常系 + 異常系 2 件以上）、`IntentMismatch`（全コマンド + `next_decision` の新規）、`IntentExecutionError` の各検査、FCC の不変条件・境界値（空・重複・順序・範囲外 `at`）、`StageIndex` 構築、実グラフ索引 0〜2 非ゲート / 3 以降ゲート / initialization への jump = `InvalidTarget` |
| 壊れた歴史 | `intent_execution.rs` 同居 | `#[should_panic]` — 通番の飛び・逆行、未知ステージ、不変条件違反（NFR3.2 (2)） |
| PBT（proptest、`PROPTEST_RNG_SEED=20260823`） | `intent_execution.rs` 同居 | 5 性質（decide = 旧 + apply / `replay(snapshot, delta)` = 通常実行 / 通番単調 / Quint 不変条件 7 本 / Err 無副作用）、`new(fold_left(agg)) == agg` |
| FCC の性質試験 | 各 FCC 型ファイル同居（proptest） | 集合型（`StageIndexSet` / `StageSlugSet`）: 結合法則・左右単位元・冪等・交換、`A \ A = empty`、`A \ empty = A`。列（`StageEntries` / `StageSlots`）: `combine` の連結順序と slug 衝突の Err、`map` の衝突 Err。用途の無い型に `combine` / `divide` を足さない（NFR 要求レビュー R-06） |
| 共通契約 | `tests/collection_contract_test.rs` | 新設 FCC を `check(..)` へ登録（`len` / `is_empty` / `at` / `fold_left` / `filter`） |
| ITF 準拠（受け入れゲート） | `tests/engine_loop_conformance.rs` | Quint トレース再生 + 射影表 + `EngineSignal` 照合。モデル不変 |
| 横断適合（U3 / U4 所有） | interface-adapter / RMU のテスト | DTO 往復 `to_domain(to_dto(agg)) == agg`、ゴールデン（バイト不変） |

受入手順（U2 の code-generation 再走の PR）:

1. TDD のコミット順（テスト先行、または 1 コミット内で `tests` → `src`）が追える（NFR2.1）。
2. CI 3 ジョブ（check / quint / coverage）+ audit 緑。
3. カバレッジはクレート全体値と `orchestration/` 単独値の両方を記録し、全体値が 98.66% を下回らない（NFR2.3、単独値は希釈を避ける参考値 — NFR 要求レビュー R-07）:

```sh
PROPTEST_RNG_SEED=20260823 cargo llvm-cov --package core-command-domain --summary-only
PROPTEST_RNG_SEED=20260823 cargo llvm-cov --package core-command-domain --summary-only \
  --ignore-filename-regex 'modules/core/command/domain/src/(workflow_definition|workspace)/'
```

4. BR4.1 の判定式は 0 件、検出力の裏取りとして同じ式を `workflow_definition` へ流すと 1 件以上（NFR2.4、NFR 要求レビュー R-04）:

```sh
rg -n -e 'enum PlanAction' -e 'pub use .*PlanAction' modules/core/command/domain/src/orchestration   # 0 件が合格
rg -n -e 'enum PlanAction' -e 'pub use .*PlanAction' modules/core/command/domain/src/workflow_definition  # 1 件以上で検出力を確認
```

5. 生の `Vec` / `&[..]` の公開が集約・イベント・`PracticesPromotion` に残っていない（DTO 境界の理由付き例外を除く）。RMU が FCC 型を定義・保持していない（読取専用の呼出は除く）。

## 5. Infrastructure Design への橋渡し

infrastructure-design は本 intent でスコープ外（SKIP）。U2 はインフラ資源を持たないため引き渡し事項なし。CI（U10）側の関係:
`cargo audit` / `unsafe_code = "forbid"` / カバレッジ床 90% + `PROPTEST_RNG_SEED` 固定の対象に `core-command-domain` と、追随で触る
3 つの兄弟クレートが含まれること。
