# code-summary — U2 ドメイン ES コア（`u2-domain-es-core`、Bolt B3）

> Code Generation（Construction 3.5）の成果要約（Unit: U2、kind: library、Bolt: B3、規模 L）。出典: 承認済み計画
> `code-generation-plan.md`（指紋 `sha256:d2b66e0b…`、Testing Contract `sha256:303d9bb7…`）、`unit-test-instructions.md`、開発エージェントの
> 報告 `developer-report-1.md`（委任 1: Step 1〜8）/ `developer-report-2.md`（委任 2: Step 9〜20 + §12 追加作業）、コンダクタの独立検証
> （2026-08-23 UTC、最終コミット `fa6bf64`）。ブランチ `bolt/b3-u2-domain-es-core`（`origin/main` `0092761` 起点）。

## 1. 結果

- U2 の合格条件（unit-of-work / bolt-plan B3）をすべて満たした: FR8.3（`orchestration` に `PlanAction` の定義・再輸出なし — grep 0 件）、
  FR8.4（畳み込みは集約の `effective_plan`、`WorkflowDefinition` から `effective_plan_action` / `next_in_scope_stage` を削除）、
  `WorkflowExecution` の ES 形 FSM（decide / `apply_event` / `snapshot` / `from_snapshot` / `with_version`、イベント 12 変種、`seq_nr` / `version`）、
  `engine_loop.qnt` の ITF 準拠維持（8 fixture + アクション網羅 16 本、モデル不変）、PBT 緑（性質 (a)〜(f) + 定義側から移設した 2 性質）。
- ADR-008（WorkflowDefinition のエンティティ識別子 / 集約間 ID 参照）と C4 改訂（`find_by_id`）を同梱した。旧 API は後方互換を残さず削除。
- 品質ゲート（コンダクタ独立実測、`fa6bf64`）: `cargo fmt --all --check` 緑 / `cargo clippy --workspace --all-targets -- -D warnings` 緑 /
  `cargo lint` 緑 / `PROPTEST_RNG_SEED=20260823 cargo test --workspace` **471 passed, 0 failed**（着手前 368）/ `bash scripts/coverage.sh`
  97.38%（絶対床 90%）/ `cargo llvm-cov -p core-domain --summary-only` lines **96.53%**（着手前基準 94.70%、+1.83pp）。
- 委任は 2 回直列（委任 1 → 委任 2 → 追加作業）、いずれも計画ファイルを書き換えず、各 Red の失敗出力を報告に記録した（TDD）。

## 2. 作成・変更ファイル（ワークスペース、`git diff --stat origin/main..fa6bf64 -- modules tests`）

**core-domain / workflow_definition**（委任 1）
- `modules/core/domain/src/workflow_definition/plan_action.rs`（`orchestration/` から `git mv` — 中身不変）
- `modules/core/domain/src/workflow_definition/workflow_definition_id.rs`（新規 — `WorkflowDefinitionId`、非空・trim・制御文字拒否）
- `modules/core/domain/src/workflow_definition/definition_revision.rs`（新規 — `DefinitionRevision`、`sha256:` + 小文字 hex64 のみ）
- `modules/core/domain/src/workflow_definition/workflow_definition.rs`（`new(id, revision, graph, grid, scopes)`、`id()` / `revision()`、
  `effective_plan_action` / `next_in_scope_stage` 削除、依存テストの書き換え）、`mod.rs`（`pub use` 追加）、`scope_grid.rs` / `stage_graph.rs`（import / doc）

**core-domain / orchestration**（委任 2 + 追加作業）
- 新規: `intent_id.rs`（`IntentId` / `IntentIdError`）、`stage_index.rs`（`StageIndex`、`pub(crate)` コンストラクタ）、`stage_entry.rs`
  （`StageEntry`、`is_gated()`）、`phase_boundary.rs`、`status.rs`（切り出し）、`next_decision.rs`（`NextRequest` / `NextDecision` 8 値 /
  `EngineSignal` + `From<&NextDecision>`）、`workflow_execution_event.rs`（封筒 + 12 変種ペイロード）、`workflow_execution_snapshot.rs`
  （16 属性 + ビルダー）、`start_request.rs`（`StartRequest { scope, request, depth?, test_strategy? }`）、エラー 4 型
  （`start_error.rs` / `command_error.rs` / `apply_error.rs` / `snapshot_error.rs`）
- 全面改訂: `workflow_execution.rs`（集約本体 — `start` / `start_with_entries`、12 コマンド、`apply_event`、クエリ、`snapshot` /
  `from_snapshot` / `with_version`、PBT 4 本）、`mod.rs`（公開面の `pub use` 列挙、コンテキスト rustdoc）、`lib.rs`（クレート rustdoc）
- 削除: 旧 `orchestration/plan_action.rs`（移動）、旧 API（`report_forward` / `gate_start` / `reject` / `revise` / `report_skipped` /
  `recompose_flip` / `next`）
- `modules/core/domain/tests/engine_loop_conformance.rs`（新 API で Quint トレース再生 — `start_with_entries` → decide → `apply_event`）

**core-use-case / core-interface-adapter / tests**（委任 1）
- `modules/core/use-case/src/orchestration/workflow_definition_repository.rs`（`find_by_id(&WorkflowDefinitionId)`、`find()` 削除、
  `GraphReadError::{NotFound{expected, actual}, HarnessIdentity{path, cause}}`）
- `modules/core/interface-adapter/src/orchestration/workflow_definition_repository_impl.rs`（`load_harness_identity` / `compute_revision` /
  `serialize_grid`、`load_graph` / `load_grid` が生値も返す、診断文言 2 本）、`.../memory/workflow_definition_repository.rs`（id 契約）、
  `.../tests/workflow_definition_repository_impl_test.rs`（`find_by_id` へ移行 + 識別子 / 内容版テスト 8 本）、`.../tests/golden_parity_test.rs`
  （識別子面 3 本、`find_by_id("claude")`）
- `tests/golden/upstream-3c3146cf/harness.json`（新規 — upstream ピンの実バイト 76 B、sha256 `85bfdec8…`、`.claude/tools/data/harness.json` と一致）、
  同 `README.md`（表に 1 行追加、既存行のバイト不変）

`core-domain` の `Cargo.toml` は不変（依存追加なし）。`core-interface-adapter` は既存の `canon-json` 依存を使う（追加なし）。

## 3. TDD の記録（各 Red の失敗出力 — 詳細は developer-report-1 §2 / developer-report-2 §3 / §12）

| Red | 対象 | 失敗の観測 | Green 後 |
|---|---|---|---|
| 委任 1 Red 1 | Data model（`WorkflowDefinitionId` / `DefinitionRevision` / `WorkflowDefinition::new` 5 引数） | `cargo test -p core-domain --lib`: 7 failed（名前付き失敗 — 検証なしスタブで Red を採取） | 140 passed |
| 委任 1 Red 2 | Repository（`find_by_id` / `NotFound` / `HarnessIdentity` / revision の安定性） | 3 コマンドで 11 failed（memory 3 / impl test 7 / golden 1） | 18 / 27 / 9 passed |
| 委任 2 Red A | Data model（leaf 型 + エラー 4 型） | コンパイルエラー 91 件（E0432 / E0433 / E0425） | 169 passed |
| 委任 2 Red B | Data model（イベント / スナップショット / NextDecision） | コンパイルエラー 78 件 | 187 passed |
| 委任 2 Red C | Business logic（集約本体） | コンパイルエラー 16 件 | 233 passed |
| 委任 2 Red D | PBT（性質 (a)〜(f) + 移設 2 性質） | `prop_assert!` のコンパイルエラー | 237 passed |
| 委任 2 Red E | API（ITF 準拠テスト） | `engine_loop_conformance` 26 件（旧 API 不在 / 型不一致） | 1 passed（8 fixture） |
| 追加作業 Red | `StartRequest` / `Started.depth` / `test_strategy` | コンパイルエラー 13 件 | 471 passed（workspace） |

Rust の静的型付けでは未定義型へのテストはコンパイルエラーとして Red になる。委任 1 は「シグネチャだけのスタブで名前付き失敗を採取」、
委任 2 はコンパイルエラーを Red として記録した（いずれも Green で置換、スタブ残骸なし）。

## 4. 主要な実装判断（設計との差分 — レビューと設計側の反映対象）

| # | 判断 | 根拠 |
|---|---|---|
| D1 | 集約とスナップショットの `stages` は `Vec<StageEntry>`（slug + phase + plan_action + conditional）。`plan` / `conditional` は独立列のまま、`from_snapshot` が整合を検査 | entities の `list<StageSlug>` と BR1.3（phase で gated 判定）の矛盾 — phase を再水和で失うと実装不能。設計側で entities を訂正（pending-revision） |
| D2 → 追加作業 | `Started` に `depth` / `test_strategy`（`Option<String>`、素通し）を `StartRequest` 経由で載せる | C5 / entities どおり。U4 の Scope Configuration 投影に必要。計画 §2 の欠落はコンダクタの誤り |
| D3 | `IntentId` は一般の kebab（`[a-z0-9]+` を `-` で連結）を受理 | entities の「`-<id8>` 必須」は実データ `260822-stage1-selfhost` と不一致。設計側で訂正 |
| D4 | `EngineSignal::from` は UnparkThenResume / ResumeMenu / NewWorkRouting を `Done` に畳む | Quint の DirectiveKind に対応語なし。ITF は踏まない |
| D5 | `start` は「initialization フェーズ全ステージが EXECUTE・非 CONDITIONAL」+「索引 0 は EXECUTE」の 2 ガード | cursor_in_scope の初期条件 |
| D6 | `apply_event(Started)` は genesis 専用（既存集約への適用は `InvariantViolation`） | BR2.3 のリプレイは from_snapshot 起点。seq_nr=1 からの再構成が要るなら U3 で入口を足す |
| D7 | decide 内 `commit` の到達不能な `Err` 腕は `InvalidTarget(cursor)` で状態不変（panic なし） | NFR4.3 |
| D8 / D9 | `WorkflowExecutionSnapshotBuilder` を公開（16 属性 > too_many_arguments）、公開面に `WorkflowExecutionEventPayload` / `IntentIdError` / `StartRequest` を追加 | 列挙型の使用に不可欠、利便再エクスポートではない |
| 委任 1-3 | `WorkflowDefinition` の `PartialEq` は derive 維持（id / revision も等価に参加） | 読取モデルの内容比較をテストが使う。エンティティ同一性の比較は `id()` 同士（`next_decision`） |
| 委任 1-4 | `DefinitionRevision` の scopes 要素は読取モデルが保持する 6 値（`description` を含まない、生バイトではない） | 「読めた 3 入力の内容版」。生バイト版にするなら後続 Bolt |
| 委任 1-7 / 1-8 | `harness.json` に env オーバライド無し。`NotFound` / `HarnessIdentity` は診断文言（upstream 互換対象外） | upstream に対応概念なし |

## 5. テスト

- ワークスペース 471 テスト（着手前 368、+103）。内訳の増分: `core-domain` lib 126 → 237+（Data model / Business logic / PBT 4 本 / StartRequest）、
  ITF 準拠 1 本（8 fixture、アクション網羅 16 本）、`core-interface-adapter` impl test 19 → 27、golden parity 6 → 9、orchestration lib 15 → 18。
- PBT（`PROPTEST_RNG_SEED=20260823`、既定 256 ケース、コマンド列 ≤ 59、合成定義 stage_count 2〜8 / initialization 1〜3）: (a) decide 後 == 旧 +
  apply、(b) replay == execute、(c) seq_nr 単調 + SequenceGap、(d) Quint 不変条件、(e) Err 無副作用、(f) snapshot 往復、+ 実効プラン合成 /
  次 in-scope の最小性（定義側から移設）。
- 実グラフ索引テスト: initialization 3 ステージの合成列で索引 0〜2 非ゲート / 3 ゲート / `jump(1)` = InvalidTarget。
- カバレッジ: core-domain lines 96.53%（regions 97.05% / functions 95.39%）。未到達は到達不能な防御腕と網羅 match のテストヘルパ腕
  （developer-report-2 §6）。

## 6. 計画からの逸脱

- 計画 §2 に無かった `StartRequest` と `Started.depth` / `test_strategy`（追加作業 — C5 / entities への整合）。
- `start_with_entries` を公開 API に追加（ITF 準拠テスト用、計画 §3 BR2.5 行に記載どおり）。
- 委任 1 が定義側の PBT 2 本を削除し、委任 2 が集約側で等価物を復活（計画 §5.3 の想定内）。
- 計画の他の Step はすべて実施。計画ファイルのチェックボックスは更新しない（承認バイト凍結 — 進捗は報告ファイル）。

## 7. 申し送り

- **設計側（機能設計の pending-revision）**: D1 / D3 / D2 / D4 / D5 / D6 / D8-9 の entities / rules / functional-spec への反映（ゲートの Request Changes 経路）。
- **U3**: `revision_count` が集約状態になった → C6 snapshot 列に 1 列追加。`WorkflowExecutionSnapshot` は `pub(crate)` フィールド + 公開ビルダーから組む。
  seq_nr = 1 からの全イベント再構成が要るなら `from_started` 相当の入口を U3 設計で追加。
- **U4**: C5 改訂提案（StageCompleted / Started.stages（StageEntry 列）/ Started.definition_id・definition_revision / 投影規則）の受入。
- **U5 / U6**: `GateOpened.artifacts` / `GateApproved.phase_boundary` / `GateRejected.feedback` / `occurred_at` / `StartRequest.depth` / `test_strategy` は
  呼出側が供給する素通し値。`next_decision` の `DefinitionMismatch` は処理中断。
- **U9**: 12 号 §2.1 / 01 号の集約表へ `WorkflowDefinitionId` / `DefinitionRevision` を追記。

## 8. コミット（ブランチ `bolt/b3-u2-domain-es-core`、`origin/main` 起点）

| SHA | メッセージ |
|---|---|
| `21dfa8a` | chore(aidlc): record U2 design (functional / NFR), ADR-008 and the Bolt B3 plan |
| `6cda871` | refactor(workflow-definition): move PlanAction out of orchestration (FR8.3/FR8.4) |
| `3e44965` | feat(workflow-definition): add WorkflowDefinitionId / DefinitionRevision (ADR-008) |
| `0b333a9` | test(golden): pin the upstream harness.json bytes at 3c3146cf |
| `6c924e6` | feat(workflow-definition): identify the definition and switch the port to find_by_id (ADR-008, C4) |
| `9210685` | test(workflow-definition): characterise the phase column of stages_in_scope |
| `83ffd7e` | chore(aidlc): record B3 delegation 1 (brief, report, diaries) and the U10 pending revisions |
| `55e9384` | feat(core-domain): event-sourced WorkflowExecution — events, snapshot, StageIndex |
| `ded4c0d` | feat(core-domain): decide/apply commands on WorkflowExecution |
| `f4910dc` | test(itf): replay engine_loop traces through the event-sourced aggregate |
| `1d035f5` | test(core-domain): cover the defensive branches of next_decision and from_snapshot |
| `fa6bf64` | feat(core-domain): carry depth / test_strategy on Started via StartRequest (C5) |

## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-08-23T03:06:19Z
**Iteration:** 1（advisory, unit: u2-domain-es-core）

### Findings

| # | Severity | Location | Finding | Recommendation |
|---|---|---|---|---|
| 1 | Major | `functional-design/functional-spec.md`（末尾 `## Review`）、`functional-design/entities.md`（`WorkflowExecution.stages` の型、`IntentId` の制約文）、`functional-design/pending-revision.md` | U2 の上流成果物 `functional-design` は**最後に記録された検証結果が NOT-READY**（iteration 2、2026-08-23T00:18:27Z、Critical 所見14: 「非ゲート = stage 0」という Quint の抽象を実グラフに持ち込んだ内部矛盾 — 実グラフでは索引 0/1/2 の 3 つとも initialization フェーズで、集約が phase を保持しないため判定不能）。この Critical は `rules.md` BR1.3（`gated(stage) = phase(stage) ≠ initialization`）と `entities.md` の `StageEntry.phase` 属性には反映済みだが、`pending-revision.md` 自身が「回復レビュー枠は消費済み — functional-design ステージゲートで Request Changes を選んだ直後に適用してレビュアーを再実行する」と明記するとおり、**その適用と再レビューはまだ行われていない**。証拠として `entities.md` の `WorkflowExecution.stages` は今も `type: list<StageSlug>`（`list<StageEntry>` への修正が pending-revision.md 項目1 として未適用）、`IntentId` の制約文も実データと矛盾する `-<id8>` 必須のまま（同項目2）。Bolt B3（本 code-generation）はこの NOT-READY な設計を出典として計画・実装され、開発エージェントが実装中に同じ矛盾を独立に発見し（code-summary D1、developer-report-2 §4 D1「entities.md の `list<StageSlug>` のままでは phase が再水和で失われ実装不能」）、`StageEntry` に `phase` を持たせる形で自力解決した。**コード自体はこの是正を正しく実装している**（`modules/core/domain/src/orchestration/stage_entry.rs` の `is_gated()` は phase 判定のみで「索引 0 の特別扱いはしない」と明記、`workflow_execution.rs` の `every_initialization_stage_is_non_gated_and_the_rest_are_gated` テストが索引 0〜2 非ゲート / 3 ゲート / `jump(1) == InvalidTarget` を実測で固定 — レビュアーが実行して確認済み）ため、コード品質そのものへの影響はない。しかし手続き上は、設計ゲートが NOT-READY のまま次工程が進み、本来は設計者が確定すべき是正（entities.md の型修正・IntentId 制約の書き直し）を開発エージェントが実装判断として肩代わりした形になっている。 | (1) `pending-revision.md` の 7 項目（特に項目1: `stages` の型、項目2: `IntentId` の制約文）を `entities.md` / `functional-spec.md` へ即時適用し、functional-design のレビュアーを再実行して verdict を確定させる（コードは既に正しいので、通る見込みは高い）。(2) 併せて `functional-spec.md` の埋め込み `## Review` セクション自体を更新し、「NOT-READY のまま stale」な状態を解消する。(3) 今後の学習として、code-generation の計画承認は上流設計ゲートの最新 verdict が READY であることを前提条件として明示的に確認する運用に倣うことを検討する。 |

### Validation Tool Results

| ツール | 結果 | 解釈 |
|---|---|---|
| `cargo fmt --all --check` | exit 0 | 緑 |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0（warning 0 件） | 緑。`unwrap_used` / `expect_used` deny を含む workspace lints 47 本が全通過 — プロダクトコードに `unwrap`/`expect` が無いことの機械的裏付け |
| `cargo lint`（`tools/lint` カスタムルール） | exit 0 | 緑。gateway-taxonomy / field-visibility 等の既存機械強制ルールを通過 |
| `PROPTEST_RNG_SEED=20260823 cargo test --workspace` | exit 0、**471 passed, 0 failed** | 緑。内訳を個別クレートごとに再実行し合計 471（core-domain lib 244 他）で code-summary の申告と一致することを確認 |
| `grep -rnE 'enum PlanAction\|pub use .*PlanAction' modules/core/domain/src/orchestration` | 0 件 | BR4.1 / FR8.3 合格。`PlanAction` は `workflow_definition` に完全移動 |
| `bun .claude/tools/aidlc-sensor-traceability.ts --output-path .../traceability.json --stage code-generation` | `"pass": false`、`invalid_targets: []`、`gaps: []`、`orphans: []`、`invalid_entries: []`、`missing_from_upstream_ids` に FR1 系 36 件 | ブリーフの想定どおり — `upstream_ids` が本 Unit 用の狭い集合（FR8.3/FR8.4/FR1.3/FR2.1/FR3.1/FR3.3 と BR/NFR 群）のみを列挙するため、要求書全体の FR ユニバースと突き合わせるセンサーはノイズを検出する。実害となる `invalid_targets`（存在しないファイル参照）・`gaps`（未被覆）・`orphans` はいずれもゼロで、42 件の coverage エントリの target はすべて実在パス（1 件を除き `modules/`・`scripts/`・`Cargo.toml` 配下、BR3.2 のみ意図的な N/A + 説明文） |
| golden bytes（`tests/golden/upstream-3c3146cf/harness.json` vs `.claude/tools/data/harness.json`） | sha256 完全一致（`85bfdec8…`） | I3 の申告どおり |
| `StageEntry::is_gated()` / `WorkflowExecution::gated()` の実装確認 | phase 判定のみ、索引 0 特別扱いなし | 所見1 の是正が実装済みであることの直接確認 |
| `next_decision` の優先順（0 DefinitionMismatch → 1 park → … → 7 next-in-scope/Done） | `rules.md` BR3.1 / `functional-spec.md` W4 と一致 | 実コードで逐語確認 |
| `jump` の forward/backward/redo 差分ロジック | BR1.6 の 2 条件（介在ステージ in-flight・現ステージ active）と一致 | 実コードで逐語確認 |
| `StageIndex` の構築制御 | `pub(crate) const fn new` — 公開経路は `stage_index(usize) -> Option<Self>`（範囲検査）と `from_snapshot` のみ | BR5.1 / NFR4.3 合格 |
| ADR-008 との突合 | `decisions.md` ADR-008 の Decision (1)〜(4) が BR2.6・実装（`start` は無条件記録、`next_decision` のみ id 検査）と一致 | 合格 |

### Summary

コード自体の品質は高い。品質ゲート4本（fmt/clippy/lint/test）と合格 grep はレビュアーの独立実行で全て緑を確認し、traceability センサーも実害ゼロ（`invalid_targets`/`gaps`/`orphans` すべて空）。TDD の Red 記録（developer-report-1/2）は実測のコンパイルエラー・失敗テスト名を伴い具体的で、PBT の実効性もカバレッジ駆動で裏取りされている（`next_decision` の防御腕2本が未到達と判明し到達経路を特定してテスト追加、という誠実な報告）。BR2.6/ADR-008（集約間 ID 参照、`DefinitionMismatch`）、BR1.6（jump の差分集合）、BR5.1（`StageIndex` の型保証）はいずれも規則どおりの実装をコードで確認した。

一方、唯一かつ重要な Major 所見は、上流の functional-design が最後の記録上 **NOT-READY**（Critical 所見14 未解消のまま）であるにもかかわらず code-generation が進んだ点である。実装側は同じ矛盾を独立に発見し正しく解消しているためコードの正しさへの実害は無いが、本来は設計ゲートで確定すべき是正を実装判断（D1 等）が肩代わりした形跡であり、`entities.md` は今も矛盾した記述のまま放置されている。この 1 件のみで Major ≤ 2 の閾値には収まるため advisory verdict は READY とするが、承認ゲートでは `entities.md` の是正適用と functional-design レビュアーの再実行（verdict 確定）を人間が優先度高く扱うことを推奨する。
