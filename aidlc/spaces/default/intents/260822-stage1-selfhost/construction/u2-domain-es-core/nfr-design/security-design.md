# security-design — U2 ドメイン ES コア（`u2-domain-es-core`）

> NFR Design（Construction 3.3）成果物（Unit: U2、kind: library）。出典: `../nfr-requirements/security-requirements.md`
> （NFR1.1〜1.3 / NFR2.1〜2.4 / NFR3.1〜3.4 / NFR4.1〜4.5、STRIDE、データ分類）、`../nfr-requirements/tech-stack-decisions.md`
> （依存追加なし、定義の識別子、PBT / ITF、手実装エラー型）、`../functional-design/functional-spec.md`（§2 インターフェイス、W1〜W7、
> §4 状態遷移、§5 エラー一覧）、`../functional-design/rules.md`（BR1.0〜BR1.9 / BR2.1〜BR2.6 / BR3.x / BR5.x）、
> `../functional-design/entities.md`、`../../../inception/contract-design/contract-summary.md`（C3 / C4 / C5 / C6）、
> `../../../inception/domain-design/decisions.md`（ADR-001〜008）、確認事項 `nfr-design-questions.md`（前提 P1〜P4、Looks correct）。
> performance / scalability / reliability / observability の要求・設計は kind = library のため存在しない。
>
> 設計ステージの制約に従い、コードは ≤15 行の例示のみ。

## 1. 設計方針

U2 は I/O・認証・認可・永続化・ログを持たない純粋な集約。セキュリティ設計は 3 点に絞る:
**(a) 不変条件の検査点を 3 か所に集約する**（decide / `apply_event` / `from_snapshot` — どれも panic せず Err）、
**(b) 定義との関係を ID で固定する**（`definition_id` の一致検査と来歴の記録 — ADR-008）、
**(c) 境界を薄く保つ**（依存追加なし・serde なし・人間入力は素通し・時計／乱数／環境を読まない）。

## 2. 不変条件の検査点（NFR3.2 / NFR4.3 / NFR1.1）

| 検査点 | 何を検査するか | 違反の返し方（状態不変） | 出典 |
|---|---|---|---|
| decide（各コマンド） | `accepts_commands`（BR1.0）、checkbox 前提（BR1.3 / BR1.4 / BR1.5）、対象の妥当性（BR1.6 / BR1.8 — 非 initialization・範囲内・Pending）、autonomy（BR1.7 / BR1.8） | `Err(CommandError::{NotRunning, CheckboxPrecondition{stage, actual}, NotSkippable, NotStale, InvalidTarget, RefusedUnderAutonomy})` | BR1.x、functional-spec W2 / W5 / W6 |
| `apply_event` | 封筒 seq_nr = 現在値 + 1（BR2.1）、ペイロードのステージ slug が `stages` に存在（UnknownStage）、適用後に不変条件（cursor in-scope / active ≤ 1 / gated Completed ⇒ approved / parked_at = cursor）が保たれること | `Err(ApplyError::{SequenceGap{expected, actual}, UnknownStage, InvariantViolation})` — 適用前の状態を保つ（検証してから書く、または一時コピーに適用して差し替える） | BR2.1 / BR2.3、functional-spec W3 |
| `from_snapshot` | 長さ一致（stages / plan / overlay / conditional / checkbox / approved / revision_count = stage_count ≥ 1）、cursor < stage_count かつ実効プラン EXECUTE（running 時）、active ≤ 1、gated Completed ⇒ approved、parked_at = cursor（park 中）、definition_id / definition_revision の存在、seq_nr ≥ 1、version ≥ 0 | `Err(SnapshotError::InvariantViolation{reason})`（U3 が `RepositoryError::Corrupt` に写す — C3） | BR5.2、NFR3.2 / NFR3.3、C6 |
| `next_decision` | 引数の `WorkflowDefinition` の id = `definition_id`（BR2.6）。revision の差は Err にしない | `Err(CommandError::DefinitionMismatch{expected, actual})` | BR2.6、NFR3.4、ADR-008 |
| `start` | scope の妥当性（UnknownScope）、グラフ空（Empty — 防御的）、initialization ステージが SKIP / conditional に畳まれた（InitializationMustExecute / InitializationMustBeUnconditional） | `Err(StartError::…)` — 集約は生成されない | BR2.2、functional-spec W1 |

- **panic なし**: 範囲を持つ索引は `StageIndex`（`stage_index(usize) -> Option<StageIndex>` でのみ構築、BR5.1）で型保証し、集約内部の
  ベクタ添字はすべて `StageIndex` 経由に限定する。`unwrap` / `expect` はプロダクトコードで禁止（workspace lint）。`# Panics` を持つ公開
  API は 0 件。
- **1 コマンド 1 イベント・Err は無副作用**（BR1.1）: decide はガードをすべて通した後にイベントを構築し、`apply_event` を経て自身に
  適用する。ガード不成立では `self` に触れない。

```text
// 検査点の形（例示）
fn approve_gate(&mut self, user_input: Option<String>, phase_boundary: Option<PhaseBoundary>)
    -> Result<WorkflowExecutionEvent, CommandError>;       // ガード → イベント構築 → apply_event → Ok(event)
fn apply_event(&mut self, event: &WorkflowExecutionEvent) -> Result<(), ApplyError>;   // seq_nr / slug / 不変条件
fn from_snapshot(s: WorkflowExecutionSnapshot) -> Result<Self, SnapshotError>;         // 不変条件を検証して復元
fn next_decision(&self, def: &WorkflowDefinition, req: &NextRequest) -> Result<NextDecision, CommandError>;
```

## 3. 定義の同一性と来歴（NFR3.4 / BR2.6 / ADR-008）

- `WorkflowDefinition` はエンティティ: `id(): &WorkflowDefinitionId`（内容が変わっても不変 — Repository 実装が harness.json の `name`
  から付与）と `revision(): &DefinitionRevision`（3 入力の正準 JSON の `sha256:` — 値属性。計算は `WorkflowDefinitionRepositoryImpl`
  が canon-json で行い、ドメインは値を運ぶだけ）。
- `start` は `def.id()` / `def.revision()` を `Started` に無条件に記録する（比較対象となる既存状態が無い静的コンストラクタ）。以後
  `WorkflowExecution` は `definition_id` / `definition_revision` を保持し、`snapshot()` にも含める（NFR3.3）。
- `next_decision` は id の一致を検査する（不一致 = 別の定義で駆動しようとした — `DefinitionMismatch`）。revision の差（ピン更新）は
  Err にしない — 計画は `Started` に自己完結しており、upstream も dist 更新をまたいでワークフローを続ける。drift は U4 / U7 が
  観測して提示できるよう、`definition_revision()` アクセサを公開する。
- ITF 準拠テストは合成の `WorkflowDefinitionId` / `DefinitionRevision`（固定値）で集約を作る（BR2.5 の合成計画と同じ扱い）。

## 4. ペイロードと情報の扱い（NFR4.4 / NFR3.1）

- 人間入力（`request` / `user_input` / `feedback` / `reason`）は `String` / `Option<String>` の素通し。集約は内容を解釈・検証・切詰め・
  要約しない。`Display` 実装（エラー型）は材料（ID・索引・状態）だけを出し、人間入力を埋め込まない（文言はアダプタ層）。
- 集約は時計・乱数・環境変数・ログ基盤を持たない。`occurred_at` は封筒値として呼出側から受け取る。`core-domain` に `std::time` /
  `std::env` / 乱数の利用が無いことをレビュー項目にする（NFR3.1）。
- 秘密情報・トークンを載せる経路は設けない（イベント型に資格情報のフィールドは無い）。

## 5. サプライチェーンと境界（NFR4.1 / NFR4.2 / NFR4.5）

- `core-domain` の依存はベースライン（`audit-events` / `directive-schema` / `message-catalog`、dev: `proptest` / `serde_json`）から
  増やさない。serde / canon-json はドメインに入れない（JSON 化は U3 のワイヤ構造体、revision 計算はアダプタ層）。
- `unsafe_code = "forbid"`（workspace lint — U10）。
- デシリアライズ面を持たない — 外部バイト列はドメインに届かず、parse-don't-validate は U3 の境界。ドメインは型で受け取った値に
  不変条件検証（§2）だけを適用する。

## 6. 決定性と契約の維持（NFR1.1 / NFR1.2 / NFR1.3 / NFR2.2 / NFR3.1）

- decide / apply は純関数的（同じ状態 + 同じコマンド → 同じイベントと次状態）。PBT で (a) decide 後の状態 == 旧状態 + apply、
  (b) replay == execute、(c) seq_nr 単調と SequenceGap、(d) Quint 不変条件、(e) Err 無副作用の 5 性質を `PROPTEST_RNG_SEED` 固定で
  固定する（NFR2.2）。生成器はコマンド列（任意長）と合成定義（任意 stage_count、initialization 1〜3 ステージ）。
- ITF 準拠（NFR1.1）: `engine_loop.qnt` のトレースを decide → apply 経路で再生し BR2.5 の射影表で突き合わせる。合成計画は
  initialization 1 ステージ（Quint の stage 0）+ 残りステージ。`audit_lock_conformance.rs` は U3 の管轄で触れない。
- ゲート判定（NFR1.2）: `gated(stage) = StageEntry.phase ≠ initialization`。実グラフ（initialization 3 ステージ）での索引 0〜2 非ゲート /
  3 以降ゲート / initialization への jump = InvalidTarget をユニットテストで固定。
- イベント語彙（NFR1.3）: 12 変種の `enum` と網羅 `match`（`#[non_exhaustive]` は付けない — 変種追加は C5 改訂を伴う設計事項）。
  ペイロードは C5 の形（+ `c5_revision_proposal`）。

## 7. 失敗の扱い

- 失敗はすべて `Result` で呼出側へ返す。沈黙の失敗なし（ガード不成立・封筒違反・不変条件違反はそれぞれ専用の Err 変種）。
- エラー型は手実装 enum + `fmt::Display`（材料のみ）+ `std::error::Error` 手実装（house style、thiserror / anyhow 不使用）。
- `DefinitionMismatch` を受けたユースケース（U6）は処理を中断して上位へ返す（別定義での駆動は契約違反）。

## 8. 要求への対応

| 要求 | 設計上の手当て |
|---|---|
| NFR1.1 | ITF 準拠を decide → apply 経路で再生、合成計画 + 射影表（§6） |
| NFR1.2 | `gated = phase ≠ initialization`、実グラフ索引のユニットテスト（§6） |
| NFR1.3 | 12 変種 enum + 網羅 match、C5 の形（§6） |
| NFR2.1 / NFR2.3 / NFR2.4 | TDD・カバレッジ・機械強制は logical-components §4（テスト配置）と Bolt B3 の受入手順 |
| NFR2.2 | PBT 5 性質、シード固定、生成器（§6） |
| NFR3.1 | 純関数的 decide / apply、時計・乱数・環境なし（§4 / §6） |
| NFR3.2 | 検査点 3 か所 + Err 変種、panic なし（§2） |
| NFR3.3 | snapshot に全状態（definition_id / definition_revision を含む）、`from_snapshot` 検証（§2 / §3） |
| NFR3.4 | `start` は記録のみ、`next_decision` が id 検査、revision は観測（§3） |
| NFR4.1 / NFR4.2 / NFR4.5 | 依存追加なし・unsafe forbid・デシリアライズ面なし（§5） |
| NFR4.3 | `StageIndex` による型保証、Err 境界、`# Panics` 0 件（§2） |
| NFR4.4 | 人間入力の素通し、Display は材料のみ、秘密情報の経路なし（§4） |

## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-08-23T01:27:34Z
**Iteration:** 1（advisory, unit: u2-domain-es-core）

### Findings

| # | Severity | Location | Finding | Recommendation |
|---|---|---|---|---|
| 1 | Major | logical-components.md §2「Bolt B3 の範囲拡張」、contract-summary.md C4 | C4（`../../../inception/contract-design/contract-summary.md:142-158`）は改訂後の `WorkflowDefinitionRepository::find_by_id` の `# Errors` に「`NotFound`（要求 id がこのハーネスの定義 id と異なる — 契約上 fatal）、読取・解析失敗は `GraphReadError` の**既存**変種」と明記しており、`NotFound` を既存変種と明確に区別している。ところが実測（`modules/core/use-case/src/orchestration/workflow_definition_repository.rs`）で `GraphReadError` の現行変種は `NotReadable` / `InvalidJson` / `ScopeFile` / `Malformed` の 4 つのみで `NotFound` は存在しない。logical-components.md §2 の Bolt B3 範囲拡張の記述（trait 改訂・`WorkflowDefinitionRepositoryImpl` への id/revision 付与・呼出側修正の列挙）はこの id 不一致時の fatal パスに一切触れておらず、`GraphReadError` に新変種を追加するのか・追加する場合の名称/フィールドをどうするのかが未確定のまま Bolt B3 の受入手順（logical-components.md §4）にも現れない。 | logical-components.md §2 の Bolt B3 範囲拡張の列挙に「`GraphReadError::NotFound{expected, actual}`（仮）を追加し、`find_by_id` が要求 id とハーネス定義 id の不一致を fatal として返す」を明記し、§4 の受入手順にもこの Err 経路のテストを追加する。 |
| 2 | Major | 上流 `decisions.md` ADR-008、functional-design/rules.md BR2.6、本ファイル §2 / §3 | `decisions.md`（`inception/domain-design/decisions.md:157-159`、ADR-008 Decision (3)）は「`WorkflowExecution` は `definition_id` / `definition_revision` を `Started` に記録して保持し、**`start` / `next_decision`** は引数の `&WorkflowDefinition` の id が一致しなければ `Err(DefinitionMismatch)`」と書いており、`start` にも id 一致検査を要求している。これに対し `rules.md` BR2.6（実測: 「start は def.id()/def.revision()を無条件にStartedに記録する（比較対象となる既存状態が無い静的コンストラクタ — 検査しない）」）と本ファイル §2 の検査点表（`start` 行は `UnknownScope`/`Empty`/`InitializationMustExecute`/`InitializationMustBeUnconditional` のみで `DefinitionMismatch` を持たない）は、`start` は検査しないという逆の記述を採用している。この後者（`start` は記録のみ）は構造的に正しい（`start` は比較対象となる既存状態を持たない静的コンストラクタ）が、`decisions.md` ADR-008 自体は未修正のまま残っており、本ファイルはこの上流との食い違いを一言も注記していない。project.md の学習ルール（「上流成果物の間に矛盾を見つけたら、読み替えて進まず、成果物を生成する前に人間へ裁定を求める」）に照らすと、この矛盾は沈黙のうちに解消（rules.md 側を正としてスルー）されており、ADR-008 を読む将来の実装者・レビュアーには `start` が id 検査をすると誤解される余地が残る。 | ADR-008 の Decision (3) から「`start` /」を削除する訂正（または ADR-008 に「2026-08-23 追記: `start` は記録のみで id 検査はしない — 比較対象となる既存状態がないため」という訂正注記を追加）を、code-generation 着手前に人間の裁定として反映する。本ファイル §3 に「ADR-008 原文は `start` にも検査を要求する記述だが、構造的に成立しないため本設計は記録のみとする」という明示の注記を加えると、以後の実装者が同じ矛盾に迷わない。 |
| 3 | Minor | logical-components.md §1「既存」行 | 「既存 | `orchestration/{checkbox, autonomy_mode, jump_direction, status}.rs` | 変更なし」という記載を実コードと突合すると、`autonomy_mode.rs` / `jump_direction.rs` は実際に `modules/core/domain/src/orchestration/` に存在するが、`checkbox.rs` は存在しない（`CheckboxState` の定義は `modules/core/domain/src/workspace/checkbox.rs` — 別の境界づけられたコンテキスト `workspace` に属する。実測: `use crate::workspace::CheckboxState;` at `workflow_execution.rs:15`）。`status.rs` も存在せず、`Status` は `workflow_execution.rs` 内にインライン定義されている（`enum Status` at `workflow_execution.rs:36`、`orchestration/mod.rs` は `pub use workflow_execution::{EngineSignal, Status};` で再輸出）。4 件中 2 件のファイルパス記載が実体と一致しない。 | 当該行を「`orchestration/{autonomy_mode, jump_direction}.rs`（変更なし）+ `workflow_execution.rs` 内の `Status`（変更なし）+ `workspace::CheckboxState`（別コンテキスト、変更なし、`orchestration` からの依存として §2 に明記）」のように実体に合わせて訂正する。 |
| 4 | Minor | security-design.md §2「不変条件の検査点」表、functional-design/rules.md BR1.9 | §2 の `decide` 行の Err 列挙に `NotStale` を含めているが、`NotStale` を実際に返すのは `stale_report`（BR1.9 — 書込なしのクエリで、BR1.0 が列挙する 12 の decide コマンドには含まれない）であり、`decide` コマンド群からは発生しない。本ファイル §1 / P1 が謳う「検査点を 3 か所（decide / apply_event / from_snapshot）+ next_decision の definition_id 検査」という整理には `stale_report` 自身の独立したガード検査（`accepts_commands` と staleness 判定）が row として現れておらず、`NotStale` の帰属が `decide` 行に紛れ込んでいる。 | `decide` 行から `NotStale` を除き、`stale_report`（BR1.9）を検査点の一覧に第 5 の行として明示するか、少なくとも §2 冒頭の「3 か所 + next_decision」という数え方に `stale_report` を注記として加える。 |

### Validation Tool Results

| Tool | 結果 | 解釈 |
|---|---|---|
| `bun .claude/tools/aidlc-sensor-traceability.ts --stage nfr-design --output-path .../traceability.json` | `{"pass":true,"gaps":[],"orphans":[],"missing_from_table":[],"missing_from_upstream_ids":[],"invalid_entries":[],"invalid_targets":[],"findings_count":0}` | traceability.json は NFR1.1〜NFR4.5 の 16 ID を過不足なく被覆し、target もすべて成果物内の節を指している |
| `bun .claude/tools/aidlc-sensor-required-sections.ts`（security-design.md） | `{"pass":true,"h2_count":8,...}` | 必須見出し 8 本（§1〜§8）が揃っている |
| `bun .claude/tools/aidlc-sensor-required-sections.ts`（logical-components.md） | `{"pass":true,"h2_count":5,...}` | 必須見出し 5 本（§1〜§5）が揃っている |
| `cat modules/core/domain/src/orchestration/mod.rs` | `mod` 一覧: autonomy_mode / jump_direction / plan_action / skeleton_stance / verdict / workflow_execution（`checkbox` / `status` の private mod は無い） | 所見 #3 の根拠 — `checkbox.rs` / `status.rs` が `orchestration/` に存在しないことを実測で確認 |
| `grep -n "enum CheckboxState" modules/core/domain/src/`、`cat modules/core/domain/src/workspace/mod.rs` | `CheckboxState` は `workspace/checkbox.rs` に定義、`workspace/mod.rs` が `pub use checkbox::{CheckboxEntry, CheckboxState};` で再輸出 | 所見 #3 の根拠 — `CheckboxState` が `orchestration` ではなく `workspace` コンテキスト所属であることを確認 |
| `grep -n "enum Status" modules/core/domain/src/orchestration/workflow_execution.rs` | `workflow_execution.rs:36` にインライン定義 | 所見 #3 の根拠 |
| `cat modules/core/use-case/src/orchestration/workflow_definition_repository.rs` | 現行 `GraphReadError` は `NotReadable` / `InvalidJson` / `ScopeFile` / `Malformed` の 4 変種、`find(&self) -> Result<WorkflowDefinition, GraphReadError>`（引数なし） | 所見 #1 の根拠 — `NotFound` 変種が存在しないことを確認 |
| `grep -n "PlanAction" modules/core/domain/src/workflow_definition/scope_grid.rs modules/core/domain/src/workflow_definition/workflow_definition.rs` | 両ファイルとも `use crate::orchestration::PlanAction;`（逆依存） | logical-components.md の PlanAction 完全移動（`workflow_definition` へ）が設計監査 C13 の逆依存を解消する設計であることを確認。移動計画自体に問題なし |
| `grep -n "ADR-008" -A 30 .../domain-design/decisions.md` | Decision (3) が「`start` / `next_decision` は...一致しなければ `Err(DefinitionMismatch)`」と明記 | 所見 #2 の根拠 |
| `grep -n "BR2\.6" -A 10 .../functional-design/rules.md` | 「start は...無条件に...記録する（...検査しない）」 | 所見 #2 の根拠 — rules.md は ADR-008 と異なる記述を採用済み |

### Summary

security-design.md / logical-components.md / traceability.json は、機械検証（traceability センサー・required-sections センサー）をいずれも通過し、不変条件の検査点（decide / apply_event / from_snapshot / next_decision）・エラー変種の割り当て・モジュール分割の大枠（`orchestration` → `workflow_definition` 一方向依存、PlanAction 完全移動による設計監査 C13 の逆依存解消）は実コードおよび BR1.x/BR2.x/BR5.x と整合している。一方で実コード・上流成果物との突合で 2 件の Major 所見を検出した: (1) C4 が要求する `find_by_id` の id 不一致時 fatal パス（`GraphReadError::NotFound` 相当）が Bolt B3 の範囲拡張の記述から欠落しており、developer が実装時に迷う具体的な穴になる。(2) ADR-008 原文が `start` にも `DefinitionMismatch` 検査を要求すると読める一方、rules.md / 本ファイルは `start` を記録のみとする逆の記述を採用しており、この食い違いが upstream に残ったまま注記されていない（project.md の「上流矛盾は人間裁定へ」ルールに照らすと本来は明示すべき分岐点）。加えて Minor 2 件（logical-components.md の既存ファイル一覧の一部不正確、`NotStale` の帰属先の誤り）を検出した。advisory 基準（Critical 0 かつ Major ≤ 2 なら READY）により READY と判定するが、所見 #1・#2 は code-generation 着手前に安価に訂正できるため、承認ゲートでの訂正を推奨する。
