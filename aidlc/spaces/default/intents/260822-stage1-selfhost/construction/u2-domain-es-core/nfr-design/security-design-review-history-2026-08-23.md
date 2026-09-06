# security-design レビュー履歴（2026-08-23、旧世界）— U2 ドメイン ES コア

> 2026-09-07 再走（Modify、質問票 P11）で `security-design.md` 末尾から逐語退避した READY レビュー節（iteration 1、Major 2 / Minor 2）と、
> 同日付の `pending-revision.md`（改訂案 1〜6）の原文。いずれも旧世界（`WorkflowExecution` / `find_by_id` の `GraphReadError::NotFound` /
> ADR-008 の `start` 検査 / `modules/core/domain/` 配置）に対するもので、現行コードでは対象が失効しているか本再走の本文で解消済み。
> 本ファイルは produces ではない。

## 退避 1: `security-design.md` 末尾の `## Review` 節（原文）

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

## 退避 2: `pending-revision.md`（原文）

# pending-revision — U2 nfr-design（ステージゲートの Request Changes で適用する改訂案）

> レビュー（iteration 1、2026-08-23、READY: Major 2 / Minor 2）の所見を是正する編集案。終端の受領が凍結している（review-freeze フック）
> ため、nfr-design ステージゲートで人間が Request Changes を選んだ直後に適用し、レビュアーを再実行する。本ファイルは produces ではない。
> ADR-008 Decision (3)（所見 2 の上流側）は inception 成果物のため先に訂正済み（`start` は記録のみ、検査は `next_decision`）。

1. `logical-components.md` §2「Bolt B3 の範囲拡張」に追記: 「`GraphReadError::NotFound { expected: WorkflowDefinitionId, actual: WorkflowDefinitionId }`
   （新変種）を `core-use-case` に追加し、`find_by_id` が要求 id とハーネス定義 id の不一致を fatal として返す（C4 の `NotFound`）。`InMemory…` /
   `…Impl` の両方で実装」。§4 受入手順に「`find_by_id` の id 不一致 → `NotFound` のテスト（Impl / InMemory）」を追加。
2. `logical-components.md` §1「既存」行を実体に合わせる: 「`orchestration/{autonomy_mode, jump_direction}.rs` は変更なし。`CheckboxState` は
   `workspace/checkbox.rs`（別コンテキスト `workspace` 所有、`use crate::workspace::CheckboxState`）で変更なし。`Status` は現在
   `workflow_execution.rs` にインライン定義 — B3 で private mod `status.rs` に切り出してファサードから `pub use`（module-visibility）」。
3. `security-design.md` §2 の `decide` 行から `NotStale` を除き、第 5 行「`stale_report`（クエリ）: `accepts_commands`（BR1.0）と staleness
   （stage < cursor ∧ Completed）— `Err(CommandError::{NotRunning, NotStale})`」を追加。§1 の「3 か所 + next_decision」に「+ stale_report の
   ガード」を注記。
4. `traceability.json` は変更なし（target の節番号は不変）。
5. （PR #27 CodeRabbit 再掲）`logical-components.md` §2 に C4 `NotFound { expected, actual }` / `HarnessIdentity { path, cause }` の契約行を追加し、Impl と
   InMemory の双方で同じ契約を検証する旨を明記（項目 1 と同じ — 実装済みの内容を設計へ写す）。
6. （PR #27 CodeRabbit 再掲）`security-design.md` §2 の `decide` 行から `NotStale` を除き `stale_report` の検査行を追加（項目 3 と同じ）。
   `nfr-design-questions.md` は人間確認済みバイト（エンジンが凍結）のため変更せず、P1 の「3 か所 + next_decision」の注記はここで補う。
