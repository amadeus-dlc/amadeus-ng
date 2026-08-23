# tech-stack-decisions — U2 ドメイン ES コア（`u2-domain-es-core`）

> NFR Requirements（Construction 3.2）成果物（Unit: U2、kind: library）。出典: `security-requirements.md`（NFR1.1〜NFR4.5）、
> `../functional-design/functional-spec.md`（§2 インターフェイス、W1〜W7）、`../functional-design/rules.md`（BR2.x / BR5.x）、
> `../../../inception/requirements-analysis/requirements.md`（NFR1〜NFR5、制約 C2 クリーンアーキテクチャ + coding-rules）、
> `../../../inception/contract-design/contract-summary.md`（C3 / C5 / C6）、`../../../inception/domain-design/decisions.md`
> （ADR-001〜007）、`aidlc/spaces/default/codekb/docs/technology-stack.md`（既存依存の実測）、確認事項 `nfr-requirements-questions.md`
> （前提 P1〜P6）。

## 1. 選定

| 領域 | 選定 | 理由 | 代替案（不採用の理由） |
|---|---|---|---|
| クレート依存 | **追加なし**。`core-domain` の実測ベースラインは runtime = workspace 内部クレート `audit-events` / `directive-schema` / `message-catalog`（`autonomy_mode.rs` が `message_catalog::bolt` を使用）、dev = `proptest` 1.11 + `serde_json`（ITF 準拠テストの ITF JSON 読取）。外部クレートは 1 つも足さない | ドメインを純粋に保つ規約（technology-stack.md「ドメインは serde 非依存が規約」、BR5.2）。サプライチェーン面（NFR4.1）に影響を出さない | serde derive をドメインに入れて JSON 化まで担う: 層 = クレートの依存内向き規則に反し、ワイヤ形式の都合（フィールド名・`schema_version`）がドメイン型に漏れる。canon-json をドメインに入れて revision を自前計算: ダイジェスト計算は I/O 境界の関心でアダプタ層の責務 |
| イベント表現 | 手書き `enum WorkflowExecutionEvent`（12 変種、C5 の形）+ 封筒構造体（intent_id / seq_nr / schema_version / occurred_at）。フィールドは private + アクセサ、`PartialEq` 手実装はドメイン同値が構造同値と一致するため derive 可 | 変種の網羅 match で語彙 12 をコンパイル時固定（NFR1.3）。coding-rules field-visibility / domain-equality に準拠 | trait object / 動的イベント型: 網羅性が失われ C5 との一致が型で保証できない |
| ステージ位置 | `StageIndex` newtype（構築は `WorkflowExecution::stage_index(usize) -> Option<StageIndex>` のみ、`Copy` + `Ord`） | 範囲外を型で排除し `# Panics` を消す（BR5.1、設計監査 C17 / C18 の恒久解、NFR4.3） | 生 `usize`: 範囲検査が各メソッドに散り panic 経路が残る |
| 定義の識別子 | `WorkflowDefinitionId`（エンティティ ID — 内容が変わっても不変。Repository 実装が harness.json の `name` から付与、例 `claude`）+ `DefinitionRevision`（内容版 — 3 入力の正準 JSON の `sha256:`、U1 canon-json hash-canonical で Repository 実装が計算。値属性）。`WorkflowExecution` は `definition_id` で間接参照、C4 は `find_by_id` に改訂（`find()` 廃止、後方互換なし） | 集約はエンティティ — ID が無いのは欠落（オーナー指摘 2026-08-23、ADR-008）。集約間参照は ID 経由。来歴（どの定義・どの内容版で始まったか）がイベントに残る | 内容アドレス ID: 値の同一性で内容が変われば追跡不能 — エンティティの責務違反（却下）。upstream ピンを ID: データに無く、テストシームのローカル差し替えを区別できない |
| 非ゲート判定 | `StageEntry.phase: PhaseId` を Started に保持し `gated = phase ≠ initialization` を集約が導く（オーナー裁定 A） | 実グラフ（initialization 3 ステージ）で upstream のフェーズ単位ゲート判定と一致（NFR1.2）。Quint の stage 0 抽象は ITF 用の合成計画で 1:1 | `gated: bool` のみ保持: 投影（U4 の PHASE_* 行）がフェーズ境界を導けず、phase を別経路で再取得することになる |
| スナップショット | `WorkflowExecutionSnapshot` 値オブジェクト（全状態、アクセサ公開、serde なし）。JSON 化は U3 のワイヤ構造体 | BR5.2 / NFR3.3。`from_snapshot` が不変条件を検証して Err を返す（NFR3.2） | 集約型自体を `Clone` で持ち回す: 不変条件の再検証点が無くなる |
| エラー型 | 手実装 enum + `fmt::Display`（材料のみ、文言はアダプタ層）+ `std::error::Error` 手実装。`StartError` / `CommandError` / `ApplyError` / `SnapshotError` | house style（thiserror / anyhow 不使用 — FR9.6 の規則文面は U9 で正本化）。Bolt B1 ゲート裁定（Error trait A） | thiserror: 依存追加 + 文言をドメインに持ち込む |
| PBT | proptest 1.11（既存）、`PROPTEST_RNG_SEED` 固定（U10 で CI / coverage.sh に設定済み）。コマンド列の生成器（任意長・任意コマンド）と性質 5 本（NFR2.2） | 既存ツールで決定的に実行できる。集約本体に同居（team.md） | quickcheck: 既存依存と重複。ケース数の増加: 検査力より実行時間の増大が先に効く |
| ITF 準拠 | Quint 0.32.0（既存、モデル不変）。`modules/core/domain/tests/engine_loop_conformance.rs` 1 ファイルを新 API（`start` / `complete_stage` / `approve_gate` / …、`next_decision`）へ書き換え（`audit_lock_conformance.rs` は U3 の管轄）。合成計画は initialization 1 ステージ + 残りを Quint のステージ数に合わせる | NFR1.1。モデル側を触らないので Quint ゲートの検査力（mutation テスト済み）を維持 | Quint モデルを 3 ステージ init に改訂: 不変条件 27 本 + witness の再検証が要り、本 intent の範囲を超える |
| 非同期 / ランタイム | なし（純粋・同期）。`async` は C3 の Repository trait（U3 / U5 / U6）側だけ | ドメインは I/O を持たない（BR5.2） | — |
| コーディング規則の機械強制 | `cargo lint`（no-public-fields / checkbox-vocabulary / reap-decision-locality）+ workspace lints 48 ルール（rust 5 + rustdoc 1 + clippy 42、`unreachable_pub` を含む — module-visibility）。BR4.1 の grep 判定式を Bolt B3 の受入手順に含める | NFR2.4。PlanAction の完全移動は再輸出禁止の裁定（module-visibility 追補）で機械検査可能 | — |

## 2. 依存の差分（予定）

| 種別 | 追加・変更 | 備考 |
|---|---|---|
| Rust クレート（runtime） | なし | `core-domain` の `[dependencies]`（内部クレート 3 つ）不変。`Cargo.lock` 不変が期待値 |
| Rust クレート（dev） | なし（proptest / serde_json 既存） | — |
| 他クレートへの波及（B3 の範囲拡張） | `core-use-case` の `WorkflowDefinitionRepository` trait を `find_by_id(&WorkflowDefinitionId)` に改訂（`find()` 削除）、`core-interface-adapter` の `WorkflowDefinitionRepositoryImpl` に id / revision の付与（`canon-json` 依存を追加 — U1 のクレート、外部依存は増えない）、既存テスト（golden parity / repository impl test）の呼出側修正 | ADR-008。後方互換の `find()` は残さない（オーナー裁定） |
| ファイル | `modules/core/domain/src/orchestration/{workflow_execution.rs（全面改訂）, workflow_execution_event.rs（新規）, snapshot.rs（新規）, stage_index.rs（新規）}`、`modules/core/domain/src/workflow_definition/plan_action.rs`（移動先）、`modules/core/domain/tests/engine_loop_*.rs`（書き換え）、BR4.1 の呼出側 10 ファイル | 配置の最終形は code-generation の計画で確定（module-visibility: 型ファイル mod は private、ファサードで `pub use`） |
| GitHub 設定 / CI | なし | U10 の設定をそのまま使う |

## 3. 未決（後続で確定）

- PBT のケース数とコマンド列の最大長（proptest 既定 256 ケース）— code-generation の計画で実行時間を見て確定（NFR2.2 は性質と決定性だけを要求し、量は定めない）。
- C5 改訂提案（StageCompleted の追加 / `Started.stages` / `Started.definition_id`・`definition_revision` / 投影規則の改訂）の U4 側受入 — U4 の functional-design で合意し、監査行の見た目が不変であることを U4 の投影テストで固定する（NFR1.2 / NFR1.3）。
- `WorkflowDefinitionId` の値の正本（harness.json `name` か、framework 名を冠するか）と `DefinitionRevision` の入力順序（graph ‖ grid ‖ scopes の連結規約）— code-generation の計画で固定し、12 号 §2.1 へ U9 が追記。
- `occurred_at` の供給者（ユースケースが時計から取る — gateway-taxonomy: 時計は Infrastructure 機構）と ITF 準拠テストでの固定値 — U5 / U6 の設計で確定。
