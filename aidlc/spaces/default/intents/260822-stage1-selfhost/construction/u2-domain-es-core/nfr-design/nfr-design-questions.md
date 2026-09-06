# nfr-design-questions — U2 ドメイン ES コア（`u2-domain-es-core`）

> NFR Design（Construction 3.3）の質問票（Unit: U2、kind: library）。出典: `../nfr-requirements/security-requirements.md`
> （NFR1.1〜1.3 / NFR2.1〜2.5 / NFR3.1〜3.4 / NFR4.1〜4.5、STRIDE）、`../nfr-requirements/tech-stack-decisions.md`（依存追加なし、
> FCC、定義の識別子、PBT / ITF、契約試験ハーネス、エラー型）、`../functional-design/functional-spec.md`（§2 API、W1〜W7、§5 エラー、
> §9 引継ぎ、末尾レビュー R-01〜R-10）、`../functional-design/rules.md`（BR1.x〜BR5.5）、`../functional-design/entities.md`、
> `../../../inception/contract-design/contract-summary.md`（C3 / C5 / C6）、`../../../inception/domain-design/decisions.md`（ADR-001〜010）、
> `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/`（module-visibility / field-visibility / tell-dont-ask / domain-equality /
> first-class-collections / error-handling）、実コード `modules/core/command/domain/`（`src/orchestration/`、`src/workspace/`、`tests/`）。
> performance / scalability / reliability / observability の要求・設計は kind = library のため存在せず、本ステージの成果物は
> `security-design.md` / `logical-components.md` / `traceability.json` の 3 つ。
>
> **2026-09-07 再走（Modify）。** 2026-08-23 の初版は旧世界（`WorkflowExecution`・12 変種・`from_snapshot` / `SnapshotError` /
> `ApplyError` 公開・「panic なし」・`next_decision` の `DefinitionMismatch`・`modules/core/domain/` 配置）の記述であり、2026-09-05 是正・
> 2026-09-07 再走後の機能設計と現行コードに合わない。以下「以前の前提」「以前に確認済みのまとめ」は確認済みバイトとして残し、
> 本再走の前提 P5〜P11 で上書きする。**質問なし。** セキュリティ設計（検査点の二層・ID 照合・FCC の構築検査・依存ゼロ）と
> 論理コンポーネント分割（現行の文脈配置・新設 FCC の置き場・兄弟クレートへの追随・テスト配置）は、NFR 要求・技術選定・機能設計・
> オーナー裁定（Q4 / Q4a / Q5、2026-09-05 / 2026-08-30）・coding-rules から一意に決まる。

## 以前の前提（2026-08-23 の記録 — 旧世界、P5〜P11 で上書き）

- P1. セキュリティ設計 = **不変条件の検査点を 3 か所に集約**する: (a) decide（各コマンドのガード — BR1.x、Err は状態不変）、
  (b) `apply_event`（封筒 seq_nr の連続性 — BR2.1、未知ステージ — UnknownStage）、(c) `from_snapshot`（長さ一致 / cursor in-scope /
  active ≤ 1 / gated Completed ⇒ approved / parked_at = cursor / definition_id の存在 — SnapshotError）。`next_decision` は
  definition_id の一致検査（BR2.6）。どの検査も panic せず Err（NFR3.2 / NFR4.3）。
- P2. ペイロード・秘密情報: 人間入力は `String` の素通し（加工・切詰め・ログなし — NFR4.4）。集約はログ基盤・時計・乱数・環境変数を
  持たない（NFR3.1）。`DefinitionRevision` の計算（canon-json）はアダプタ層、ドメインは値を運ぶだけ（NFR4.1 / NFR4.5）。
- P3. 論理コンポーネント（`core-domain` 内、module-visibility 準拠 — 型ファイル mod は private、公開はコンテキスト直下の
  mod.rs の `pub use` 列挙のみ）:
  - `orchestration/`: `workflow_execution`（集約 — 状態・decide・apply・クエリ）/ `workflow_execution_event`（封筒 + 12 変種）/
    `workflow_execution_snapshot` / `stage_index` / `stage_entry` / `next_decision`（NextRequest / NextDecision / EngineSignal 導出）/
    `command_error`・`apply_error`・`snapshot_error`・`start_error`（手実装 enum + Display + Error）。既存の `checkbox` /
    `autonomy_mode` / `jump_direction` / `status` は残す。
  - `workflow_definition/`: `plan_action`（完全移動 — FR8.3）/ `workflow_definition_id` / `definition_revision`（新設、Domain Primitive）、
    `workflow_definition` に `id()` / `revision()` を追加。`effective_plan_action` / `next_in_scope_stage` は削除（FR8.4）。
  - 公開面: `core_domain::orchestration::{WorkflowExecution, WorkflowExecutionEvent, WorkflowExecutionSnapshot, StageIndex,
    StageEntry, NextRequest, NextDecision, EngineSignal, CommandError, ApplyError, SnapshotError, StartError, …}`、
    `core_domain::workflow_definition::{PlanAction, WorkflowDefinitionId, DefinitionRevision, …}`。利便再エクスポート無し。
- P4. 障害ドメインとテスト配置: 障害は「呼出側へ返す `Err`」の 1 ドメイン（ブラストラディウスは 1 コマンド実行）。ユニットは
  各モジュールのインライン `#[cfg(test)]`、PBT（5 性質 — NFR2.2）は `workflow_execution` 同居、ITF 準拠は
  `tests/engine_loop_conformance.rs`（書き換え、合成 WorkflowDefinitionId / 合成計画）。infrastructure-design は SKIP（引き渡し
  なし）。

## 以前に確認済みのまとめ（2026-08-23）

- U2 に固有の NFR 設計質問はなし。耐障害・スケール・キャッシュ・観測のパターンは純粋な集約に不要
- セキュリティ設計（P1 / P2）: 不変条件の検査点は decide / apply_event / from_snapshot の 3 か所（+ next_decision の definition_id 検査）、すべて Err で panic なし。人間入力は素通し、時計・乱数・環境・ログなし、revision 計算はアダプタ層
- 論理コンポーネント（P3）: `orchestration/` に集約・イベント・スナップショット・StageIndex / StageEntry・NextDecision・エラー 4 型の private mod、`workflow_definition/` に PlanAction（移動）と WorkflowDefinitionId / DefinitionRevision（新設）、公開はファサードの `pub use` 列挙のみ
- 障害ドメインとテスト配置（P4）: Err の 1 ドメイン、ユニット同居 + PBT 同居 + ITF は engine_loop_conformance.rs、infrastructure-design は SKIP

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct

## 2026-09-07 再走（Modify）— 前提の更新

用語の注釈: **FCC** = ファーストクラスコレクション（配列をそのまま持たず、不変条件と操作を持つ専用の型で包んだもの）。
**DTO** = データ転送オブジェクト（アダプタ層が保存・復元のために使う写し）。**RMU** = read-model-updater（イベントから
リードモデルを投影するクレート）。**ITF 準拠テスト** = Quint モデルのトレースを集約に再生して突き合わせるテスト。

- P5. **検査点は二層**（NFR3.2、旧 P1 を上書き）: (1) **境界の検査付き変換** — `IntentExecution::new`（DTO → 集約基底。
  空計画・列の長さ不一致・cursor 範囲外・parked_at ≠ cursor・seq_nr = 0・slug 重複・状態不変条件違反を `IntentExecutionError`
  （理由文字列を持つ 1 型）の Err で返し、U3 が `RepositoryError::Corrupt` に写す）、`Intent::create`（`IntentError`）、
  BR5.5 で新設する FCC の構築検査（非空・slug 一意・辞書順などの不変条件違反を型ごとの Err で拒否）、コマンド入口のガード
  （`CommandError` / `ReportRefusal` / `SingleStageRunRefusal` / `SkeletonStanceRefusal`、Err は状態不変）。
  (2) **型変換後の壊れた歴史** — `replay` / `apply_event` は通番の飛び・未知ステージ・不変条件違反を回復せず panic する
  （オーナー裁定 2026-08-30、`# Panics` を明記、`ApplyError` は `pub(crate)`）。旧世界の `from_snapshot` / `SnapshotError` /
  公開 `ApplyError` / 「どの検査も panic なし」は失効。
- P6. **集約参照の照合**（NFR3.4、旧 P1 の definition_id 検査を上書き）: `&Intent` を受ける全コマンド・書込前ガード
  （`jump_resolve` / `stale_report`）・`next_decision`（Result 化、Q5 = A）は `intent_id` の不一致を `CommandError::IntentMismatch`
  で拒否する。定義 ID の照合は `Intent::resolve_review_policy(&WorkflowDefinition, ..)` が担い、不一致は
  `IntentReviewError::DefinitionMismatch`（現行実装名）。`Intent` は `definition_id` / `definition_revision` を来歴として持ち、
  revision の差は Err にしない。
- P7. **新設 FCC の置き場と公開面**（module-visibility、BR5.5、旧 P3 を上書き）: 現行配置は `modules/core/command/domain/src/`
  の 3 文脈（`orchestration/` 55 ファイル・`workflow_definition/`・`workspace/`）で、型ファイル mod は private、公開はファサード
  `mod.rs` の `pub use` 列挙のみ。新設 FCC は次のとおり置く —
  `orchestration/` に `stage_entries` / `stage_slot` / `stage_slots` / `stage_index_set` / `artifact_paths` / `stage_slug_set` /
  `transition_steps`（要素型 `StageEntry` / `StageKey` / `StageIndex` / `TransitionStep` を所有する文脈）;
  `workspace/` に `promoted_sections` / `rule_lines`（要素型 `PromotedSection` と `PracticesPromotion` を所有する文脈。
  `orchestration` → `workspace` の参照は既存方向: `CheckboxState` / `HumanTurns` / `PracticesPromotion`）。
  `ReviewAttempt` の内部列（`pending: BTreeSet<u32>` / `closed: Vec<ReviewClosure>`）の FCC 名・不変条件は機能設計レビュー R-01 の
  未決であり、functional-design ゲートの Request Changes で確定する — 置き場は `orchestration/` の `review_attempt` 隣接 private mod
  とし、公開は `ReviewAttempt` の操作（`is_pending` / `has_terminal` / `closed`）経由で型自体はファサード公開しない方針を
  仮置きする。各 FCC は構築・`combine` / `map` の衝突を型ごとの手実装 Error（enum + `Display` + `std::error::Error`）で拒否する。
- P8. **兄弟クレートへの追随（越境の裁定）**（機能設計 §9 #3、レビュー R-02 / R-10、NFR 要求レビュー R-02 / R-03）: U2 の Bolt に
  次の追随を含める — command interface-adapter の DTO（`intent_dto.rs:85` / `created_dto.rs:47` / `intent_execution_dto.rs:142` /
  `intent_execution_event_dto.rs:113,121` の要素列挙を `fold_left` へ）、read-model-updater（`read_tables.rs:239,284` /
  `read_tables/stage_lookup.rs:23` / `workspace/resolved_plan.rs:49` の列挙を `fold_left` / `at` へ、`read_tables/next_answer_row.rs:58`
  の `next_decision` 呼出を Result 処理へ）、core-command-use-case（`commit_verdict_use_case.rs:212,218` の `steps.contains` →
  `TransitionSteps` の業務操作、`test_support.rs:114,856,889` の `stages().to_vec()`）、ITF 準拠テスト
  `tests/engine_loop_conformance.rs:356,449,488`。根拠は PlanAction 完全移動の「呼出側一斉修正を同 Unit に含む」先例と、
  機能設計の要約確認（2026-09-07、Looks correct）。リードモデル側は FCC 型を**定義・保持しない**（境界での読取専用の
  `fold_left` / `at` の呼出は除く — NFR 要求レビュー R-01 の是正）。DTO の列表現（正準 JSON のバイト）は変えない。
- P9. **テスト配置と受入手順**（NFR2.x、旧 P4 を上書き）: ユニットは各モジュールのインライン `#[cfg(test)]`; PBT 5 性質は
  `intent_execution.rs` 同居; FCC の Monoid 則・差集合則（集合型: StageIndexSet / StageSlugSet）と連結の衝突拒否（列: StageEntries /
  StageSlots）は各 FCC 型ファイル同居の proptest; 共通契約（`len` / `at` / `fold_left` / `filter`）は既存ハーネス
  `tests/collection_contract_test.rs` へ登録（登録対象の確定列挙は R-01 の定義確定が前提 — NFR 要求レビュー R-05）;
  ITF は `tests/engine_loop_conformance.rs`（モデル v2.7 不変）。受入手順は (a) `cargo llvm-cov --package core-command-domain`
  のクレート全体値（床 98.66%）に加え `--ignore-filename-regex` で `orchestration/` 単独値も記録する（NFR 要求レビュー R-07）、
  (b) BR4.1 判定式はコードブロックで `rg -n -e 'enum PlanAction' -e 'pub use .*PlanAction' modules/core/command/domain/src/orchestration`
  0 件、検出力の裏取りとして同じ式を `workflow_definition` へ流すと 1 件以上（NFR 要求レビュー R-04）。
- P10. **障害ドメイン**（旧 P4 を上書き）: (1) 呼出側へ返す Err — ブラストラディウスは 1 コマンド実行（状態不変、イベントなし）。
  (2) 壊れた歴史の panic — ブラストラディウスは再構成を行った 1 プロセス（ユースケース / RMU の 1 回）。真実源（SQLite ジャーナル）が
  破損している状態なので進まないのが正で、復旧は U3 の責務。infrastructure-design は SKIP（引き渡しなし）。
- P11. **旧レビュー節と改訂案の退避**: 2026-08-23 の READY レビュー節（Major 2 / Minor 2）と `pending-revision.md`（改訂案 1〜6）は
  旧世界（`find_by_id` の `GraphReadError::NotFound`・ADR-008 の `start` 検査・`checkbox.rs` / `status.rs` の所在・`NotStale` の帰属）
  に対するもので、現行コードでは対象自体が失効（`GraphReadError` は現行に存在しない）または本再走の本文で解消する（`NotStale` は
  `stale_report` の行に置く）。両者を `security-design-review-history-2026-08-23.md` へ逐語退避し、`pending-revision.md` は削除する。

## Consolidated Summary Confirmation

- U2 に固有の NFR 設計質問はなし。2026-08-23 の前提 P1〜P4 は旧世界の記述であり、P5〜P11 で上書きする
- 検査点は二層（P5）: 境界の検査付き変換（`IntentExecution::new` の `IntentExecutionError`、`Intent::create`、FCC の構築検査、コマンド入口のガード）は Err で状態不変。型変換後の壊れた歴史は `replay` / `apply_event` が panic（2026-08-30 裁定、`# Panics` 明記）。`from_snapshot` / `SnapshotError` / 公開 `ApplyError` は失効
- 集約参照の照合（P6）: `&Intent` を受ける全コマンド・ガード・`next_decision`（Result 化）は `IntentMismatch`。定義 ID は `Intent::resolve_review_policy` の `DefinitionMismatch`。revision の差は Err にしない
- 新設 FCC の置き場（P7）: `orchestration/` に StageEntries / StageSlot / StageSlots / StageIndexSet / ArtifactPaths / StageSlugSet / TransitionSteps、`workspace/` に PromotedSections / RuleLines。ReviewAttempt の内部列は R-01 の未決として functional-design ゲートで確定（置き場と非公開の方針は仮置き）。衝突は型ごとの手実装 Error で拒否
- 兄弟クレートへの追随（P8）: interface-adapter DTO 4 ファイル・read-model-updater 4 ファイル・core-command-use-case 2 ファイル・ITF テストを U2 の Bolt に含める（先例 + 2026-09-07 の要約確認が根拠）。リードモデル側は FCC 型を定義・保持しない（読取専用の呼出は除く）。DTO のバイト表現は不変
- テスト配置と受入手順（P9）: ユニット同居・PBT 同居・FCC の性質試験は型ファイル同居・共通契約は collection_contract_test.rs・ITF は engine_loop_conformance.rs。受入はクレート全体 + orchestration 単独のカバレッジ、BR4.1 判定式は `-e` 2 本 + 検出力の裏取り
- 障害ドメイン（P10）: Err = 1 コマンド、panic = 1 プロセス（真実源の破損）。infrastructure-design は SKIP
- 退避（P11）: 旧レビュー節と pending-revision.md は履歴ファイルへ逐語退避し、pending-revision.md は削除

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct
