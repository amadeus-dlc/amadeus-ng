# tech-stack-decisions — U2 ドメイン ES コア（`u2-domain-es-core`）

> NFR Requirements（Construction 3.2）成果物（Unit: U2、kind: library）。**2026-09-07 再走（Modify）** — 現行コード
> （`modules/core/command/domain/Cargo.toml`、`src/orchestration/`、`src/workflow_definition/`、`tests/`）と 2026-09-05 是正・
> 2026-09-07 再走後の機能設計（BR5.5 FCC 化、Q5 `IntentMismatch`、§9 引継ぎ）へ同期した。出典: `security-requirements.md`
> （NFR1.1〜NFR4.5）、`../functional-design/functional-spec.md` / `rules.md` / `entities.md`、
> `../../../inception/requirements-analysis/requirements.md`（制約 C2 クリーンアーキテクチャ + coding-rules）、
> `../../../inception/contract-design/contract-summary.md`（C3 / C5 / C6）、`../../../inception/domain-design/decisions.md`
> （ADR-001〜010）、`aidlc/spaces/default/codekb/docs/technology-stack.md`、`rust-toolchain.toml`、確認事項
> `nfr-requirements-questions.md`（P7〜P12）。

## 1. 選定

| 領域 | 選定 | 理由 | 代替案（不採用の理由） |
|---|---|---|---|
| ツールチェーン | Rust 1.95.0（`rust-toolchain.toml`、components rustfmt / clippy / llvm-tools）、edition 2024 | U10 で固定済み。ローカルと CI の rustc を一致させる | floating stable: 上流更新で CI が突然赤になる（U10 で退けた） |
| クレート依存 | **追加なし**。`core-command-domain` の実測ベースラインは runtime = `chrono`（時刻の値）/ `uuid`（v7 — 識別子の検査とイベント ID 採番）/ `core-infrastructure`（言語拡張: canon_json、collections）、dev = `proptest` / `serde_json`（ITF JSON 読取のみ）。FCC 化・`next_decision` 改修で外部クレートを 1 つも足さない | ドメインを永続化知識から中立に保つ機械強制（serde / event-store-adapter-rs の不在 — domain-persistence-neutrality）。サプライチェーン面（NFR4.1）に影響を出さない | serde derive をドメインに入れて JSON 化まで担う: 層 = クレートの依存内向き規則に反し、ワイヤ形式の都合がドメイン型に漏れる。汎用コレクションクレート（im / indexmap 等）の追加: 既存 `core_infrastructure::collections` で足り、依存追加の理由がない |
| 集約とイベント | `Intent`（Created 1 変種）と `IntentExecution`（`IntentExecutionEvent` 16 変種、各変種は自前の UUIDv7 `IntentExecutionEventId` と `aggregate_id` を持つ）。1 コマンド 1 イベント、`apply_event(seq_nr, occurred_at, &event)` が通常実行と再生の同一経路。通番・時刻・`schema_version`・直列化はアダプタ封筒 | 変種の網羅 match で語彙 16 をコンパイル時固定（NFR1.3）。イベントはエンティティ（domain-object-kinds、aggregate-commands） | 複合イベント ID（集約 ID + 通番）: 2026-09-02 裁定で撤去。trait object / 動的イベント型: 網羅性が失われる |
| 再構成 | `replay(snapshot: IntentExecution, delta)` — 最新スナップショット（DTO → `IntentExecution::new` の検査付き変換）を基底に `seq_nr` より大きい差分だけを適用。壊れた歴史は panic、DTO・封筒の不整合は Err（`Corrupt`） | オーナー裁定 2026-09-05（差分再生）と 2026-08-30（壊れた歴史はクラッシュ）。memento 双子型を持たない（BR5.2） | ジャーナル全再生（2026-08-30 追記）: 2026-09-05 裁定で上書き。`Result` を返す `apply_event`: 回復不能な歴史を回復可能に見せる |
| ステージ位置 | `StageIndex` newtype（構築は `IntentExecution::stage_index(usize) -> Option<StageIndex>` と DTO 境界の検査付き `new`。`Copy` + `Ord`） | 範囲外を型で排除（BR5.1、NFR4.3） | 生 `usize`: 範囲検査が各メソッドに散る |
| ファーストクラスコレクション（新規） | コマンド側ドメインモデルの配列はすべて FCC（BR5.5）: `StageEntries`（静的計画）/ `StageSlots`（位置ごとの記録、旧 7 並列列の統合）/ `StageIndexSet`（位置集合）/ `ArtifactPaths` / `StageSlugSet`（辞書順・重複なし — レビュー R-03）/ `PromotedSections` / `RuleLines` / `TransitionSteps` ほか。共通契約は `core_infrastructure::collections::FirstClassCollection`（`len` / `at` / `fold_left` / `filter`）、`map` / `combine` / `divide` は型ごとの契約（集合は Monoid 則、列は連結 + 衝突拒否）。リードモデル側は FCC を使わず `fold_left` で平坦な表現へ写す | オーナー裁定 2026-09-06（規則）と 2026-09-07（Q4 / Q4a）。配列やイテレータを集約の外へ取り出さず、jump の位置集合の合成や受領証の一括リセットを集合演算で書く | 汎用 `Collection<T>` / `NonEmptyCollection<T>` をそのままフィールド型にする: 不変条件（slug 一意・文書順）と業務操作を持てない。共通 trait への `combine` / `divide` 追加: オーナーの最終方針だが、結果型と失敗条件が型ごとに異なるため今回は型ごとの契約に留める（積み残し） |
| 定義の識別子と ID 参照 | `WorkflowDefinitionId`（系譜 ID）+ `DefinitionRevision`（内容版、`CompiledDefinition` が `of_content` で導出 — ADR-008 改訂 2026-09-02）。`Intent` が `definition_id` / `definition_revision` を来歴として保持、`IntentExecution` は `intent_id` のみ。`&Intent` を受ける全 API（`next_decision` を含む）は `IntentMismatch` で拒否（Q5 = A） | 集約間は ID 参照（aggregate-references）。来歴がイベントに残る | `next_decision` の `DefinitionMismatch`（旧 NFR3.4）: 実行は定義を直接参照しないため失効。照合を呼出側の責務にする（Q5 = B）: 規則からの逸脱を常態化する |
| 非ゲート判定 | `StageEntry.phase` / `StageKey.phase` を保持し `gated = phase ≠ initialization` を集約が導く。誕生が initialization 全段を完了済みにする | 実グラフ（initialization 3 ステージ）で upstream のフェーズ単位ゲート判定と一致（NFR1.2）。初期化完了の二重記録を作らない | `gated: bool` のみ保持 / `complete_stage` ×3 の誕生手順: 失効 |
| エラー型 | 手実装 enum + `fmt::Display`（材料のみ、文言はアダプタ層）+ `std::error::Error` 手実装。`IntentError` / `PlanError` / `CommandError` / `IntentExecutionError` / `ReportRefusal` / `ReportCommitError` / `SingleStageRunRefusal` / `SkeletonStanceRefusal` | house style（coding-rules/error-handling、thiserror / anyhow 不使用） | thiserror: 依存追加 + 文言をドメインに持ち込む |
| PBT | proptest（既存）、`PROPTEST_RNG_SEED=20260823` 固定（`scripts/coverage.sh` / CI と同値）。コマンド列の生成器と性質 5 本（NFR2.2）+ FCC の Monoid 則・差集合則（NFR2.5） | 既存ツールで決定的に実行できる。集約本体に同居（team.md） | quickcheck: 既存依存と重複 |
| ITF 準拠 | Quint（既存、モデル `engine_loop.qnt` v2.7 は U2 の再走で不変）。`modules/core/command/domain/tests/engine_loop_conformance.rs` 1 本を改修後 API（`slots` / `stage_key` / `next_decision` の Result）へ追随 | NFR1.1。モデル側を触らないので Quint ゲートの検査力を維持 | モデルの同時改訂: 不変条件・witness の再検証が要り、本再走の範囲を超える |
| 契約試験ハーネス | `modules/core/command/domain/tests/collection_contract_test.rs`（既存 7 型の trait 適合）に新設 FCC を登録（NFR2.5） | 規則の適用例が定める検査場所を使う | 型ごとに散在するテストだけ: 共通契約の欠落が検出できない |
| 非同期 / ランタイム | なし（純粋・同期）。`async` は Repository trait（U3 / U5 / U6）側だけ | ドメインは I/O を持たない（BR5.2） | — |
| コーディング規則の機械強制 | `cargo fmt` / `cargo clippy -D warnings`（workspace lints 50 = rust 5 + rustdoc 1 + clippy 44）/ `cargo lint`（coding-rules 6 規則の機械強制分）。BR4.1 の判定式（現行配置）を code-generation の受入手順に含める | NFR2.4 | — |

## 2. 依存の差分（予定 — U2 code-generation 再走）

| 種別 | 追加・変更 | 備考 |
|---|---|---|
| Rust クレート（runtime） | なし | `core-command-domain` の `[dependencies]`（chrono / uuid / core-infrastructure）不変。`Cargo.lock` 不変が期待値 |
| Rust クレート（dev） | なし（proptest / serde_json 既存） | — |
| 他クレートへの波及（同じ Bolt で追随、functional-spec §9 #3 とレビュー R-02） | command interface-adapter の DTO（Started / Created / IntentExecution）の要素列挙を `fold_left` へ、read-model-updater（`ResolvedPlan::of`、`read_tables` の行生成・slug 引当、`NextAnswerRow::of` の Err 処理）、`core-command-use-case`（`commit_verdict_use_case.rs` の `steps.contains` → `TransitionSteps` の操作）、ITF 準拠テスト `engine_loop_conformance.rs` | PlanAction 完全移動の「呼出側一斉修正を同 Unit に含む」先例に倣う。リードモデル側は FCC を使わない |
| ファイル | `modules/core/command/domain/src/orchestration/` に FCC 型ファイル（型ファイル mod は private、ファサードで `pub use` — module-visibility）、`intent.rs` / `intent_execution.rs` / `intent_execution_event/*.rs` の改修、`mod.rs` 冒頭の旧説明の修正（§9 #4）、`tests/collection_contract_test.rs` への登録 | 配置の最終形は code-generation の計画で確定 |
| GitHub 設定 / CI | なし | U10 の設定をそのまま使う |

## 3. 未決（後続で確定）

- PBT のケース数とコマンド列の最大長（proptest 既定 256 ケース）— code-generation の計画で実行時間を見て確定（NFR2.2 / NFR2.5 は性質と決定性だけを要求し、量は定めない）。
- レビュー所見 R-01（TransitionSteps / ReviewAttempt の pending・closed / PromotedSections / RuleLines の型定義）は機能設計の凍結中のため、code-generation の計画で不変条件・操作・結果型を確定し、functional-design のゲート（Request Changes）で設計本文へ折り戻す。
- `combine` / `divide` / `map` の共通 trait への一律化（オーナーの最終方針、Q4a）— 着手時期は別途裁定。既存 7 型の改修を伴うため U2 の Bolt には含めない。
- 上流 `components.md` 冒頭注記と `contract-summary.md` C3 の B13 追記（「ジャーナル全再生」）の 2026-09-05 裁定への同期 — intent 記録の積み残し（Issue は起票しない）。
