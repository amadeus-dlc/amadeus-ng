# security-requirements のレビュー履歴 — 2026-08-23（旧設計世代の READY 判定、iteration 2）

> 2026-09-07 の再走（Modify）で security-requirements.md 末尾から原文のまま退避した。旧 WorkflowExecution・12 変種・snapshot 値オブジェクト世代の記録であり、現行本文の承認・レビュー判定ではない。

## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-08-23T01:13:50Z
**Iteration:** 2（advisory, recovery, unit: u2-domain-es-core）

### Findings

| # | Severity | Location | Finding | Recommendation |
|---|---|---|---|---|
| 1 | Major | security-requirements.md NFR3.4（+ 波及: NFR4.3）、functional-design/rules.md BR2.2 / BR2.6、functional-design/functional-spec.md エラー一覧表 | NFR3.4 の合格基準は「id 不一致で Err、一致で Ok のテストが緑」を **`start` / `next_decision` の両方**に要求するが、`start` は自身より前の集約状態を持たない静的コンストラクタである。実測（`modules/core/domain/src/orchestration/workflow_execution.rs:101`）でも現行 `pub fn start(plan, conditional) -> Result<Self, StartError>` は `&mut self` を取らず新しい `Self` を返す。ADR-008 後の新シグネチャ `start(&definition, scope)`（rules.md BR2.2 logic）も同様に集約を新規構築するだけで、比較対象となる既存の `self.definition_id` は Started 適用前には存在しない。加えて型の面でも矛盾がある — rules.md BR2.6 logic は `DefinitionMismatch` を `CommandError::DefinitionMismatch` として書いており、functional-spec.md のエラー一覧表（157 行目）も `DefinitionMismatch` を `CommandError` の変種として掲載しているが、`start` の戻り値型は `StartError`（BR2.2 の `violation` フィールドで明記）であり、別のエラー enum である。したがって「`start` が `DefinitionMismatch` を返す」という記述は、(a) 比較対象が構造的に存在しない、(b) 返すべきエラー型が現行設計のどの enum にも収まらない、という二重の理由で文字通りには実装できない。NFR3.4 と NFR4.3 はこの矛盾を検証・解消せずそのまま合格基準に取り込んでいる。 | NFR3.4 の記述を「`next_decision`（および Started 適用後に `&WorkflowDefinition` を受け取る以後のクエリ／コマンド、戻り値は `CommandError`）が id 一致を検査する。`start` は self.definition_id / definition_revision を引数の値から無条件に代入するだけで、一致検査は行わない（比較対象がまだ存在しないため）」に訂正する。もし「二重 start 呼び出しの検出」のような別の意図があるなら、それは集約 API ではなく Repository / ユースケース側の責務として別途明記する。functional-design 側（rules.md BR2.6、functional-spec.md エラー一覧表）にも同じ訂正が波及するため、code-generation 着手前に一次資料として反映すること。 |

### iteration 1 所見の解消状況

| iter1 # | Severity | 判定 | 根拠 |
|---|---|---|---|
| 1（依存ベースラインの実測不一致・NFR4.1「追加0」の再定義） | Major | 解消 | `modules/core/domain/Cargo.toml` を実測。`[dependencies]` = `audit-events` / `directive-schema` / `message-catalog`（3 内部クレート）、`[dev-dependencies]` = `proptest` / `serde_json`。NFR4.1 本文の記載と一字一句一致。`message-catalog` が `autonomy_mode.rs:7`（`use message_catalog::bolt as msg;`）で実際に使用されていることも確認 |
| 2（NFR1.1 の ITF 書き換え対象の範囲） | Major | 解消 | `modules/core/domain/tests/engine_loop_conformance.rs` は `core_domain::orchestration::{AutonomyMode, EngineSignal, PlanAction, Status, WorkflowExecution}`（現行 API）を import しており書き換え対象であることを確認。同ディレクトリの `audit_lock_conformance.rs` は `core_domain::workspace::LockProtocol` のみを対象とし `WorkflowExecution` に一切触れていないことを確認 — 「1 ファイルのみ」の主張は正確 |
| 3（NFR3.3 の snapshot 列挙と entities.md の不一致） | Major | 解消 | entities.md `WorkflowExecution` の属性列（intent_id, definition_id, definition_revision, stages, plan, overlay, conditional, checkbox, cursor, status, parked_at, autonomy, approved, revision_count, seq_nr, version — 16 属性）と NFR3.3 の列挙が完全に一致 |
| 4（NFR2.3 のカバレッジ根拠） | Minor | 部分的に解消（許容） | ドメインクレート単独の実測値は依然未取得だが、artifact はこれを正直に明記し（「ドメインクレート単独の実測値は未取得」）、Bolt B3 着手時に `cargo llvm-cov --package core-domain` を 1 回取る具体的なアクションを合格基準に組み込んだ。コードがまだ存在しない NFR 段階では妥当な着地点であり、追加の指摘は不要と判断 |
| 5（NFR2.4 の lint 数 48） | Major | 解消 | `Cargo.toml` `[workspace.lints]` を実測: rust 5（`unsafe_code` / `missing_docs` / `unsafe_op_in_unsafe_fn` / `dropping_copy_types` / `unreachable_pub`）+ rustdoc 1（`broken_intra_doc_links`）+ clippy 42（列挙して実数を確認）= 48。NFR2.4 本文の内訳と完全一致 |

### Validation Tool Results

| Tool / 確認 | 結果 | 解釈 |
|---|---|---|
| `bun .claude/tools/aidlc-sensor-traceability.ts --stage nfr-requirements` | `{"pass":true,"gaps":[],"orphans":[],...,"findings_count":0}` | traceability.json は upstream_ids（NFR1〜5）を過不足なく被覆。NFR3 の target に NFR3.4 が追加されていることも確認 |
| `bun .claude/tools/aidlc-sensor-required-sections.ts`（security-requirements.md） | `{"pass":true,"h2_count":5,...}` | 必須見出し 5 本が揃っている |
| `bun .claude/tools/aidlc-sensor-required-sections.ts`（tech-stack-decisions.md） | `{"pass":true,"h2_count":3,...}` | 必須見出し 3 本が揃っている |
| `cat modules/core/domain/Cargo.toml` | runtime = audit-events / directive-schema / message-catalog、dev = proptest / serde_json | NFR4.1「追加なし」の主張と一致（iter1 #1 の裏取り） |
| `grep -n 'lints' -A 60 Cargo.toml`（[workspace.lints] 集計） | rust 5 + rustdoc 1 + clippy 42 = 48 | NFR2.4 の lint 数と一致（iter1 #5 の裏取り） |
| `cat aidlc/spaces/default/codekb/docs/technology-stack.md` | proptest 1.11.0、Quint 0.32.0、「ドメインは serde 非依存が規約」 | tech-stack-decisions.md / security-requirements.md の技術選定記載と一致 |
| `head -15 modules/core/domain/tests/engine_loop_conformance.rs` / `audit_lock_conformance.rs` | engine_loop 側は旧 API import、audit_lock 側は `LockProtocol` のみ | NFR1.1 の書き換え対象限定の主張を裏付け（iter1 #2 の裏取り） |
| `grep -n 'pub fn start' modules/core/domain/src/orchestration/workflow_execution.rs` | `pub fn start(plan, conditional) -> Result<Self, StartError>`（静的コンストラクタ） | 所見 #1 の根拠 — `start` に比較対象となる既存 `self` が無いことを実装で確認 |
| entities.md `WorkflowExecution` 属性列 ↔ security-requirements.md NFR3.3 | 16 属性が完全一致 | iter1 #3 の裏取り |
| rules.md BR2.2（`violation: StartError`）↔ BR2.6（`Err(CommandError::DefinitionMismatch)`）↔ functional-spec.md エラー一覧表（`DefinitionMismatch` は `CommandError` 変種） | 3 資料間で `start` の戻り値型 `StartError` と `DefinitionMismatch` の所属型 `CommandError` が食い違う | 所見 #1 の根拠 |

### Summary

iteration 1 の Major 所見 4 件・Minor 所見 1 件はすべて実測で解消（Minor 1 件は NFR 段階として妥当な形で着地）を確認した。依存ベースライン・lint 数・snapshot 属性列挙・ITF 書き換え範囲はいずれも実コード（`Cargo.toml`・テストファイル・`entities.md`）と一字一句一致しており、過大主張は見当たらない。新設 NFR3.4（定義の来歴と同一性）は ADR-008 / BR2.6 / C4 / C5 とおおむね整合するが、合格基準が `start` 関数にも id 一致検査を要求している点で、BR2.2（`start` の戻り値型は `StartError`）・functional-spec.md のエラー一覧表（`DefinitionMismatch` は `CommandError` 変種）と矛盾し、`start` には比較対象となる既存状態も無い（Major 所見 #1）。この矛盾は upstream（rules.md BR2.6）由来だが、NFR3.4 がそれをそのまま合格基準として固定しており、code-generation 段階で開発者が実装に詰まって設計者に確認せざるを得ない具体的な箇所である。advisory 基準（Critical 0 かつ Major ≤ 2 なら READY）に照らすと Major 1 件のみのため READY と判定するが、この 1 件は code-generation 着手前に安価に訂正できる（NFR3.4 の文言修正 + rules.md BR2.6 の同期）ため、人間の承認ゲートで訂正を求めることを推奨する。
