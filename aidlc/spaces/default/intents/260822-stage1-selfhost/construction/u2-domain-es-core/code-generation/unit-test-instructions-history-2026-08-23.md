# unit-test-instructions — U2 ドメイン ES コア（`u2-domain-es-core`）

> Code Generation（Construction 3.5）のユニットテスト指示（Unit: U2、Bolt: B3）。Testing Contract: tdd / standard / classic / brownfield
> （`code-generation-plan.md` の `## Testing Contract`）。方針の正本は `aidlc/spaces/default/memory/team.md` Testing Posture。

## 1. テストフレームワークと設定

- Rust 標準テストハーネス（`cargo test`）+ proptest 1.11（PBT、`core-domain` の dev-dependency — 既存）+ serde_json（dev、ITF の JSON 読取）。
  新規依存なし。
- PBT のシードは固定: `PROPTEST_RNG_SEED=20260823`（`scripts/coverage.sh` / CI と同じ値。proptest 1.11 の `RngSeed::Fixed`）。
- lint: `cargo clippy --workspace --all-targets -- -D warnings`（テストコードは `clippy.toml` で `unwrap` / `expect` 許可）、`cargo lint`。
- テストコードでは `unwrap` / `expect` を使ってよい（統合テストは file-level `#![allow(clippy::unwrap_used)]` — 既存どおり）。

## 2. 本 Unit のテストの走らせ方（Unit スコープのコマンド）

| 対象 | コマンド |
|---|---|
| ドメイン（ユニット + PBT） | `PROPTEST_RNG_SEED=20260823 cargo test -p core-domain --lib` |
| ITF 準拠（engine_loop） | `cargo test -p core-domain --test engine_loop_conformance` |
| Repository ポート（use-case） | `cargo test -p core-use-case` |
| Repository 実装・ゴールデン（interface-adapter、本 Unit が触るテストのみ） | `cargo test -p core-interface-adapter --test workflow_definition_repository_impl_test --test golden_parity_test` + `cargo test -p core-interface-adapter --lib orchestration::` |
| 合格 grep（FR8.3） | `grep -rnE 'enum PlanAction\|pub use .*PlanAction' modules/core/domain/src/orchestration` → 0 件で合格 |
| カバレッジ（ドメイン単独、基準値の記録） | `cargo llvm-cov -p core-domain --summary-only` |

最初の TDD Red の前に、brownfield の実測で上のコマンドが走ること（2026-08-23 実測: `core-domain` 126 + ITF 2 テスト緑）を確認する
（Step 2）。Build and Test は各 Unit のコマンドを実行するため、ワークスペース全体の `cargo test --workspace` は品質ゲート（Step 20）
でのみ使う。

## 3. テスト範囲と量（standard: コンポーネントごと 5〜8 本）

| コンポーネント | テスト（代表） |
|---|---|
| `WorkflowDefinitionId` / `DefinitionRevision` | parse 往復、空・不正形の拒否、`sha256:` 形式検証、Display、Eq / Ord |
| `WorkflowDefinition`（id / revision） | `new` + アクセサ、`stages_in_scope` の PhaseId、`effective_plan_action` / `next_in_scope_stage` の不在（コンパイルで担保） |
| `WorkflowDefinitionRepository`（Impl / InMemory） | `find_by_id` 一致で Ok、不一致で `NotFound`、harness.json 欠落で `HarnessIdentity`、revision の安定性と変化、golden parity が `claude` で読める |
| `StageIndex` / `StageEntry` / `IntentId` | 範囲外 → None、Ord、parse |
| `WorkflowExecutionEvent` | 封筒（seq_nr / schema_version = 1 / occurred_at）、12 変種のペイロードアクセサ、Eq |
| `WorkflowExecutionSnapshot` | 16 属性のアクセサ、`from_snapshot(snapshot()) == self`、不変条件違反の各 Err |
| `WorkflowExecution`（decide） | 現行 9 本の移植 + complete_stage の initialization 限定 / approve_gate 省略経路 / reject の revision_count / jump の差分集合 / recompose 複数件 / unpark / Err 無副作用 |
| `WorkflowExecution`（apply / クエリ） | SequenceGap / UnknownStage / InvariantViolation、`next_decision` 8 分岐 + DefinitionMismatch + revision 差、`jump_resolve`、`stale_report` |
| 実グラフ索引 | initialization 3 ステージの合成 StageEntry 列で索引 0〜2 非ゲート / 3 ゲート / jump(1) = InvalidTarget |
| PBT（6 性質） | (a) decide = 旧 + apply、(b) replay == execute、(c) seq_nr 単調 / SequenceGap、(d) Quint 不変条件、(e) Err 無副作用、(f) snapshot 往復 |
| ITF 準拠 | 8 fixture 全緑 + アクション網羅アサート（既存）を新 API で維持 |

## 4. カバレッジ目標

- ワークスペース絶対床 90%（`scripts/coverage.sh`）。ドメインクレート単独は Step 0 の基準値（`cargo llvm-cov -p core-domain --summary-only`）
  を下回らない。除外は `main.rs` のみ（U2 のコードに除外を足さない）。

## 5. モック / スタブ

- ドメインは I/O を持たないためモック不要。Repository のテストは tempdir（既存フィクスチャ）に 3 入力 + `harness.json` を書いて実ファイルで
  検証、`InMemoryWorkflowDefinitionRepository` はテストダブル（`Impl` 接尾辞を付けない）。
- ITF 準拠テストは合成 `WorkflowDefinitionId("itf")` / `DefinitionRevision("sha256:" + "0"×64)` と Quint の plan / conditional から合成した
  `StageEntry` 列（索引 0 = initialization）で集約を作る（`start_with_entries`）。

## 6. テストデータ

- Quint トレース fixture: `tests/conformance/fixtures/engine_loop/*.itf.json`（8 本、不変）。
- 実グラフ: `tests/golden/upstream-3c3146cf/{stage-graph,scope-grid,harness}.json`（harness.json は本 Bolt で追加 — upstream 実バイト）。
- 各テストは自前でデータを組み立て、共有の可変状態を持たない。
