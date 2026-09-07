AIDLC-UNIT: u9-canon-docs
AIDLC-TESTING-CONTRACT: sha256:303d9bb7b5d777d54a6761be9ed154d85d5bb3f2d6b9cce02f71f4ed1b3a4ff3

# developer-brief-3 — 派遣 A: coding-rules 9 ファイル + 仕様 10 号 + 01 号（U9 再走、2026-09-07）

Conversation language: 日本語（文書本文・注記・報告はすべて日本語。型名 / API 名 / ファイル名 / ID / YAML キー / 逐語文言は英語のまま）。

## 役割と範囲

あなたは aidlc-developer-agent。Unit **u9-canon-docs**（kind: spec）の再走 Bolt、派遣 A を担当する。**コードは書かない** — 所有ファイル（書込可）は次の文書だけ:

- `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/README.md`
- `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/error-handling.md`
- `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/factory-naming.md`
- `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/gateway-taxonomy.md`
- `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/module-visibility.md`
- `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/use-case-rules.md`
- `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/command-query-separation.md`（履歴マーカー 1 行のみ）
- `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/interior-mutability.md`（履歴マーカー 1 行のみ）
- `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/field-visibility.md`（履歴マーカー 1 行のみ）
- `docs/specs/10-orchestration.md`
- `docs/specs/01-domain-model.md`
- 報告: `aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u9-canon-docs/code-generation/developer-report-3.md`（**新規・あなただけが書く**）

それ以外は**読むだけ**。特に `docs/specs/11-*.md` / `12-*.md`、`inception/**`（派遣 B が並行して編集中）、`modules/` / `tools/` / `scripts/` / `.github/` / `Cargo.*` /
`formal/`、`docs/specs/research/**`、`docs/specs/deviations.md`、他の coding-rules ファイル（`CONSISTENCY-AUDIT-*.md` 含む）、計画 `code-generation-plan.md` /
`unit-test-instructions.md` / `code-generation-questions.md` には触らない。

**禁止事項**: `git add` / `git commit` / push / PR / GitHub への書込をしない（コミットはメインが行う）。`.claude/` 配下のツールを実行しない。`AIDLC_*` 環境変数を
設定・変更してフックを回避しない。成果物の保存は Write / Edit ツール経由（シェルのリダイレクトや heredoc で書かない）。書込スコープ外のファイルに触らない。

## 先に読むもの（順に）

1. 規則の束: `aidlc/spaces/default/memory/org.md`、`team.md`、`project.md`（Mandated / Corrections）、`phases/construction.md`。
2. `aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u9-canon-docs/code-generation/code-generation-plan.md`（承認済み。§1〜§5、特に §2 写像表の派遣 A 行と §5.1）
   と同ディレクトリの `unit-test-instructions.md`（受入検査 (2)(3)(4)(7)(9) を自己実行する）。
3. `aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u9-canon-docs/functional-design/gap-measurement-20260907.md` **全文** — §1（コードの現状）、
   §2.1（10 号）/ §2.4（01 号）/ §2.5（coding-rules）の表、**§4 の台帳（正しい姿の正本）**、§4.7（未実装）、§4.8（記録間の矛盾の裁き）、§5 と追加実測 1〜4。
4. `aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u9-canon-docs/nfr-design/security-design.md` §2（改訂の作法 — 下に全文転記）と §4（受入検査）。
5. `aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u9-canon-docs/functional-design/rules.md`（BR1.6 / BR3.3 / BR3.6 / BR4.2 / BR5.1 / BR5.2 / BR5.3）。
6. 実装の一次出典（読取のみ、主張を書く前に必ず該当箇所を開く）: `modules/core/command/domain/src/orchestration/{intent.rs,intent_execution.rs,intent_execution_event.rs}`、
   `modules/core/command/domain/src/lib.rs`、`modules/core/command/use-case/src/orchestration/port/*.rs`、`modules/core/command/use-case/src/orchestration/test_support.rs`、
   `modules/core/command/interface-adapter/src/orchestration/*_repository_impl.rs`、`modules/core/query/use-case/src/`（`Find*UseCase`、`*Dao`、`directive.rs`）、
   `modules/core/infrastructure/src/`（`canon_json` / `collections`）、`Cargo.toml`（members）、`modules/app/aidlc/src/main.rs`。

## 正しい姿の正本と、基準の取り方

- **基準は現行コード**（`origin/main` = `02cacea2`、作業ツリーの `modules/` はこれと diff ゼロ — 2026-09-07 メイン実測）。gap-measurement §4 の台帳は 1 件ずつ
  コードで実否確認済みで、**§4 に無い主張は書かない**。記録（Bolt / Unit の設計記録・完了報告・handoff）を転記しない。記録間で食い違う主張は §4.8 の裁きに従う。
- **実測表は 3 列で読む**: §2 の表の『現在の主張 / 現行文言』列は**今その行に書いてある語**、『コード』列は**置換後の語**である。行番号は探索の目安であり、
  同じ節の現在の内容を読んで改訂する（レビュアーが列を読み違えて「その行に置換後の語が無い」と誤指摘した実例がある）。
- **主張を書く前にコードを開く**。型名・関数名・変種数・属性数・クレート名は、該当ファイルを Read して確認してから書く。確認できない主張は書かず、報告の「保留」に載せる。

### メインの基線実測で判明した訂正（gap-measurement を上書きする — 必ず反映）

- **T1. テストダブルの記述**（gap-measurement §1「テストダブル型 `InMemoryXxxRepository` は無く」/ §4.2 P4 の訂正）: `modules/core/command/use-case/src/orchestration/test_support.rs`
  は `#[cfg(test)]`（`orchestration/mod.rs:38-39`）の `pub(crate)` フェイク 4 つ（`InMemoryIntentExecutionRepository` / `InMemoryIntentRepository` /
  `InMemoryWorkflowDefinitionRepository` / `InMemoryCompiledDefinitionRepository`）を持ち、ユースケースの単体テスト（例 `commit_verdict_use_case.rs:317-344`）は
  これを使う。同ファイル冒頭の doc（:1-17）に理由がある — `core-command-use-case` は `core-command-interface-adapter` を dev-dependency にも書けない（DIP の
  クレート分離強制、`use-case-rules.md` §1）ため、**DIP 制約下の use-case 単体テスト専用**の trait フェイクとして置く。**公開のインメモリ実装はアダプタ層の
  `XxxRepositoryImpl<S>::in_memory()`（本家 memory バックエンド、`intent_execution_repository_impl.rs:182` 等）が正**で、自作 HashMap ダブルはそれ以外で禁止
  （オーナー裁定 2026-08-31）。クエリ側は `InMemoryXxxDao` 13 が公開のテストダブル。→ `use-case-rules.md:36` と 10 号 :99-101 はこの 3 層構造で書く
  （「テストダブル型は無い」と書かない）。
- **T2. `Rehydrated*`**: 型としては 0 件。`port/mod.rs:15-18` と `port/intent_execution_repository.rs:36-38` の doc に「廃止済み（オーナー裁定 2026-08-30）」の
  言及が残るだけ。`expected_version` はポートの引数には無く、アダプタ実装内のローカル変数（`let expected_version = aggregate.version();`）だけ。
- **T3. `JournalReader`**: メソッド 9 = `async fn` 8（events_after / events_through / checkpoint / advance_checkpoint / publish / pending_publication /
  steering_source_digest / replace_steering）+ 同期 `fn prepare_read_model(&mut self)` 1（`journal_reader.rs:38`）。「async fn 9」と書かない。
- **T4. FCC**: ドメイン 16 型（orchestration 8 / workspace 6 / workflow-definition 2）に加え、`core-infrastructure` に汎用の `Collection<T>` / `NonEmptyCollection<T>`
  （`collections/`、#114）と `canon_json` の `ObjectMembers` が `FirstClassCollection` を実装する。ドメインの FCC 数を書くときは「ドメイン 16」と限定する。
- **T5. ランタイム**: `modules/app/aidlc/src/main.rs:12` `#[tokio::main(flavor = "current_thread")]`。`tokio::spawn` / `spawn_blocking` は `modules/` に 0 件。
  ポート 4 の動詞はすべて `async fn`。

### 訂正 2 件（security-design §3 (c) — 必ず反映）

1. **RMU 呼出の用語**: 「ポート・ユースケース・RMU は `async fn`、駆動ループ・`tokio::spawn` を持たず、合成ルート（`aidlc`）が読取前と書込後に `catch_up` を await で
   直列に呼ぶ（`runtime.rs:194,200,238`）」と書き、**「同期呼出」「同期 `catch_up`」と書かない**（ADR-006「async は初期化から、ドメインは同期」は有効）。
   「同期」を使えるのは、集約が I/O を持たず純粋であることを述べる文だけ。
2. ADR-010 への失効注記は派遣 B の担当（あなたは `decisions.md` に触らない）。

## 改訂の作法（security-design §2 の全文）

| 作法 | 内容 |
|---|---|
| 最小変更 | 対象節だけを書き換え、周辺の逐語・体裁は保つ。改訂箇所は gap-measurement §2.1〜§2.6 の表の行に限り、『維持』行は触らない（BR5.1 (e)）。節の新設は BR が求める場合のみ（01 号 §7.1 の原則追記、10 号 §3 のポート 2 行追加、12 号 §2.3 のクエリ 3 件追記） |
| 出典注記 | 改訂した文・表の行・箇条の末尾に括弧書きで出典を残す — 形式 `（ADR-010）` / `（B13 2026-08-30）` / `（オーナー裁定 2026-09-02）` / **`（実測 intent_execution.rs:40 / IntentExecution::replay）`**。複数は `/` 区切り。実測の所在は path:line または型・関数名（NFR1.3、tech-stack §1） |
| 逐語契約の保護 | `docs/specs/research/**` は読むだけ（変更ゼロ）。10 号 §1 の「逐語の完全列挙は抽出文書と upstream を正とする」を維持。監査イベント名 / CLI 語彙 / `AIDLC_*` / 逐語文言 / ファイル形式の記述は引用のみで改変しない。`Directive` / `DirectiveKind` / `ContinueToken` は「所在 = クエリ側」を書き足すだけで、kind の列挙や JSON 形は変えない（BR3.3 (d)） |
| 履歴の残し方 | 失効した記述は削除せず `~~旧文~~ — 失効（日付 / 出典）` で残す。履歴として残す行には履歴マーカー（同一行に `~~`、または 旧 / 失効 / 是正済み / 改名 / 履歴 のいずれか）を必ず含める — これが受入 (2) の grep 除外条件になる。coding-rules の履歴的言及 8 行（P4）は本文を変えずマーカー語だけを添える。既存の `## Review` 節（components / contract-summary / unit-of-work）は 1 バイトも変えない（NFR1.5） |
| 実装状態の表記 | gap-measurement §4.7 の未実装項目（unpark / jump / recompose のユースケースと CLI 配線、フック 4 本、doctor、`aidlc-state` 他 24 動詞・`aidlc-bolt` 他 7 動詞、workspace 集約 3 と供給面 4、`intents.json` 直列化、Bolt / SwarmBatch）は `予定（未実装、クリティカルパス n）` の形で書く。記録間で食い違う主張は §4.8 の裁き（コードの現状）に従い、記録 A / B の文面を転記しない（NFR1.4） |
| 用語 | RMU の呼出は「ポート・ユースケース・RMU は `async fn`、駆動ループ・`tokio::spawn` を持たず、合成ルートが await で直列に呼ぶ」と書き、**「同期呼出」とは書かない**（ADR-006 は有効 — gap-measurement 追加実測 1）。「同期」を使えるのはドメイン（集約）が I/O を持たず純粋であることを述べる文だけ |
| 言語と体裁 | 日本語正本、固定トークン（型名 / API 名 / ファイル名 / ID / YAML キー / 逐語文言）は英語のまま。Markdown 表は見出しと同じ列数（regex 内の `\|` はエスケープ）、同一見出しの重複を作らない（NFR2.4 / NFR2.5） |
| 範囲の規律 | 改訂対象は P4 の確定版 — coding-rules **20 行 9 ファイル**（改訂 12 行 6 ファイル + 履歴マーカー付与 8 行 3 ファイル）、仕様 4 号（01 / 10 / 11 / 12。deviations は触らない）、共有契約 3 本（components 全面 / contract-summary 節単位 / unit-of-work 注記のみ）、`decisions.md` ADR-010 :476-478 への失効注記 1 段落。コード（modules / tools / scripts / .github / Cargo.*）と `formal/` は触らない |

補足: クリティカルパスの番号は `aidlc-state.md` の「クリティカルパス」節（4 = マルチコール CLI + 文言カタログ配線、5 = 最小フック、6 = doctor → ドッグフード）。
workspace 集約 3 / 供給面 4 / `intents.json` は項目 2、Bolt / SwarmBatch はスコープ外（swarm）なので `予定（未実装、swarm はスコープ外）` のように書く。

## 作業対象（gap-measurement §2 の表を 3 列丸ごと — 行番号は 2026-09-07 実測）

### A-1. coding-rules — 改訂 12 行（BR1.6）

| ファイル:行 | 現在（今その行に書いてある語） | コード（置換後） | 処置 |
|---|---|---|---|
| `README.md:50` | error-handling 行「利用者向け文言はアダプタ層（message-catalog）」 | message-catalog は 2026-08-29 解体、出す側の `wording`（`modules/app/aidlc/src/wording.rs`、RMU `workspace/wording.rs`） | 要改訂（error-handling.md 本文 :12 と同期） |
| `error-handling.md:4` | 適用例「core-domain `CommandError` / `ApplyError` / `StartError` / `SnapshotError`」 | `StartError` / `SnapshotError` は存在しない。現行は `core-command-domain` の `CommandError` / `ApplyError` / `IntentError` / `IntentExecutionError` / `PlanError` / `StageSlotsError` / `TransitionStepsError` / `PromotionPlanError` …（`ls modules/core/command/domain/src/orchestration/*error*.rs` で実測してから書く） | 要改訂 |
| `factory-naming.md:47` | 反例「`WorkflowExecutionState` はリテラル 2 箇所 … 次 Bolt で是正」 | 型ごと消滅（是正済み） | 履歴化（`~~…~~ — 是正済み（B13 2026-08-30、型ごと消滅）` の形） |
| `factory-naming.md:84` | 例「`WorkflowExecution::with_version`」 | `IntentExecution::with_version` / `WorkflowDefinition::with_version` | 名称のみ |
| `gateway-taxonomy.md:223` | 改訂注記の本文「書込モデル（集約 `WorkflowExecution` + `EventStore`）」 | `IntentExecution` | 名称のみ |
| `gateway-taxonomy.md:299` | 「`core-domain` に存在する集約ルート型名」 | `core-command-domain` | クレート名 |
| `module-visibility.md:11` / `:21` | `core_domain::{workspace, …}` / `core_domain::workspace::CheckboxState` | `core_command_domain::…`（`lib.rs:29-31` に `pub mod orchestration / workflow_definition / workspace`、`CheckboxState` 現存） | クレート名 |
| `module-visibility.md:12` | 例「`message_catalog::{state, lock, bolt}`」 | 解体済み。現行の共有語彙は `core_infrastructure::canon_json` 等（`modules/core/infrastructure/src/lib.rs` を見て実在のモジュール名で例を組む） | 例を差し替え |
| `use-case-rules.md:11` | 「`core-use-case` の Cargo.toml に `core-interface-adapter` が無い」 | `core-command-use-case` / `core-command-interface-adapter`（`modules/core/command/use-case/Cargo.toml` で確認） | クレート名 |
| `use-case-rules.md:36` | 「実装は実質 2 つ（Impl + InMemory）… `XxxUseCase<InMemoryXxxRepository>`」 | **T1 の 3 層構造**: 公開のインメモリ実装はアダプタ層 `XxxRepositoryImpl<S>::in_memory()`（本家 memory バックエンド）。use-case クレートの単体テストだけは DIP 制約により `#[cfg(test)]` の `pub(crate)` フェイク `InMemoryXxxRepository`（`test_support.rs`、オーナー裁定 2026-08-31）を使う。クエリ側は `InMemoryXxxDao` | 要改訂 |

### A-2. coding-rules — 履歴マーカー付与 8 行（本文は変えず、同一行にマーカー語 `旧` / `失効` / `是正済み` / `改名` / `履歴` のいずれか、または `~~` を添える）

`command-query-separation.md:5` / `interior-mutability.md:5` / `field-visibility.md:46` / `factory-naming.md:5` / `factory-naming.md:99` / `error-handling.md:12` /
`gateway-taxonomy.md:20` / `gateway-taxonomy.md:290`。これらは是正の経緯・退役の履歴として書かれており、**文意を変えない**。最小の形は行末に
`（履歴: 旧名 `WorkflowExecution` は B12 2026-08-30 で `IntentExecution` に改名）` のような括弧注記を添えること。既にマーカー語を含む行なら触らない
（受入 (2) の grep で当該行が消えることを確認してから次へ）。

### A-3. `docs/specs/10-orchestration.md`（262 行）

| 行 | 現在の主張 | コード | 処置 |
|---|---|---|---|
| 3-15 | 冒頭の B12 読み替え注記・B13 優先順位注記（「全文追従は後続 Bolt」） | 該当 Bolt = 本再走 | 本文へ畳み込み、注記を「追従済み（2026-09-07 / U9 再走）」の 1 行に縮める（旧注記は `~~…~~` で 1〜2 行に圧縮して残してよい） |
| 43-46 | §2.1 見出し「集約: `WorkflowExecution`」、identity `IntentId`、実装パス `modules/core/domain/` | `Intent`（`IntentId`）+ `IntentExecution`（`IntentExecutionId`、`intent_id` で参照）、`modules/core/command/domain/` | 要改訂（2 集約へ分割して記述 — 台帳 O1 / O2 / O3） |
| 48 | 「16 属性。`version` 列を除去、`RehydratedWorkflowExecution` が持ち回る」、`stages: Vec<StageEntry>` / `plan` / `conditional` 展開列 | `IntentExecution` 12 属性（`version` は集約内）、計画は `Intent.stages: StageEntries`、実行側は `slots: StageSlots`（FCC）、`Rehydrated*` 型なし | 要改訂（O3 / O8） |
| 50 | コマンド（11） | 15 + `start` | 要改訂（5 件追加: record_single_stage_run / record_skeleton_stance / request_review / record_review_verdict / affirm_practices — O4） |
| 51 | ドメインイベント（11） | 16 変種（`intent_execution_event.rs:82-112`） | 要改訂（O5） |
| 52 | メメント `state()` / `from_state()`、`WorkflowExecutionState`、serde は memento 経由 | memento 型なし。復号検査点は `IntentExecution::new`（アダプタの DTO から）、再生は `replay(snapshot, events)` | 要改訂（O9。B13 注記で非規範化済みだが本文に残る） |
| 55 | 「`next_decision` は失敗しないクエリ `(&self, &NextRequest) -> NextDecision`」 | `(&self, &Intent, &NextRequest) -> Result<NextDecision, CommandError>`（`IntentMismatch`、b51） | 要改訂（O10） |
| 56 | Tx 境界「ジャーナル追記 + スナップショット更新を同一 Tx」 | 一致 | **維持** |
| 70 | §2.2 `EffectivePlan` 行「集約 `WorkflowExecution` の `effective_plan`」 | `IntentExecution::effective_plan(&self, ..)` | 名称のみ |
| 66-76 | §2.2 `DirectiveKind` / `Directive` / `ContinueToken` / `ProgressSignature` / `DirectiveMaxBytes` を orchestration の Domain Primitive として列挙 | 実装は `core-query-use-case`（クエリ側の View / DTO）。コマンド側ドメインには `NextDecision` / `NextRequest` / `StateBinding` 材料 | 「所在 = クエリ側」を列に追加（構造の規範、逐語契約 = kind の列挙・JSON 形は不変 — O15） |
| 82-87 | §2.3 表・注記の `WorkflowExecution`（3 件）、`next_decision(&NextRequest)` | `IntentExecution`、`(&Intent, &NextRequest)` | 名称・署名を改訂 |
| 93 | §3 ユースケース列（`Next` / `Continue` / `Report` / `Park` / `Unpark` / `JumpResolve` / `JumpExecute` / `SetAutonomy` / `Recompose` / …） | コマンド側 9（`CommitVerdict` / `CreateIntent` / `DefineWorkflow` / `Park` / `PromotePractices` / `RecordReview` / `RecordSingleStageRun` / `RecordSkeletonStance` / `SwitchAutonomy`）、`Next` / `Continue` はクエリ側 `Find*` 13、`Unpark` / `Jump*` / `Recompose` は未実装 | 実装名へ揃え、未実装は**予定**と明記（P5 / P6 / §4.7） |
| 99-101 | 「1 trait 1 Impl（`XxxRepositoryImpl` + `InMemoryXxxRepository`）」「`WorkflowDefinitionRepository` は `InMemoryWorkflowDefinitionRepository` を持つ」 | **T1 の 3 層構造**で書く | 要改訂 |
| 105 | ポート表 `WorkflowExecutionRepository` 行（`store(.., expected_version: usize)`、`Rehydrated…` を返す、`find_by_id(&IntentId)`） | `IntentExecutionRepository { find_by_id(&IntentExecutionId) -> IntentExecution, store(&mut self, &event, &aggregate) }`（P1） | 要改訂 |
| 106 | `WorkflowDefinitionRepository` 行「`save` は持たない」「`GraphReadError`」 | `find_by_id` / `find_for_intent` / `store`、エラーは `RepositoryError<WorkflowDefinitionId>`（P1 / P2） | 要改訂 |
| （欠落） | — | `IntentRepository { find_by_id, find_for_execution(&IntentExecution), store }` / `CompiledDefinitionRepository { find_by_id, store }` | 2 行追加 |
| 110 | 「監査台帳は `WorkflowExecution` のイベントログ」 | `IntentExecution` | 名称のみ |
| 137 / 205-206 / 236 | I8・実装順序・S1 の `WorkflowExecution` / `InMemory…` | 同上 | 名称のみ（205-206 は履歴化） |

### A-4. `docs/specs/01-domain-model.md`（289 行）

| 行 | 現在の主張 | コード | 処置 |
|---|---|---|---|
| 3-15 | 冒頭 2 注記 | — | 畳み込み（10 号と同じ形） |
| 96 | §3.1 状態機械「所有者は集約 `WorkflowExecution`」 | `IntentExecution::effective_plan` | 名称のみ |
| 102 | §3.2 集約「`WorkflowExecution`、遷移動詞 11、decide 12 コマンド、16 属性、イベント 12 種、メメント `WorkflowExecutionState`、`Rehydrated…`」 | `Intent`（7 属性、`Created`）+ `IntentExecution`（12 属性、15 コマンド + genesis、16 変種、memento なし、`version` は集約内） | 要改訂（段落を書き直す — O1〜O9） |
| 106 | Domain Primitive 候補「`WorkflowExecution::start` が焼き込む」 | `Intent::create(id, &WorkflowDefinition, StartRequest, WorkspaceScan, at)` が `StageEntries` を確定、`IntentExecution::start(id, &Intent, at)` | 要改訂（O2 / O7） |
| 116 | §3.3 集約 `Intent` / `Space` / `Worktree` | 未実装（11 号 §2.1 と同じ） | **予定**と明記（S4 — orchestration の `Intent` は同名・別物と注記） |
| 121 | 脚注「U2 実装の `IntentId::parse` は kebab を受理、B5 で是正」 | 是正済み（UUIDv7 検証、`intent_id.rs`） | 履歴化（「是正済み」） |
| 184 / 231 | §4 B1・§6 の `WorkflowExecution` | `IntentExecution` | 名称のみ |
| 278 | §7.1 原則 5「`WorkflowExecution` が `WorkflowDefinition` を `definition_id` で参照」 | `Intent` が `WorkflowDefinition` を `definition_id` で、`IntentExecution` が `Intent` を `intent_id` で参照 | 要改訂（例を 2 段に） |

## 手順（計画 §5.1 の Step 1〜3）

**Step 1（Red）**: 改訂前に次を実行し、結果（件数と行）を `developer-report-3.md` の「Red 基線」に記録する。

```bash
grep -rnE 'effective_plan_action|next_in_scope_stage|AuditLedgerRepository|AuditLedgerService|StateFileStore|report_forward|gate_start|WorkflowExecution|RehydratedWorkflowExecution|message-catalog' aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/README.md aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/error-handling.md aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/factory-naming.md aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/gateway-taxonomy.md aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/module-visibility.md aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/use-case-rules.md aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/command-query-separation.md aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/interior-mutability.md aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/field-visibility.md docs/specs/10-orchestration.md docs/specs/01-domain-model.md | grep -vE '~~|旧|失効|是正済み|改名|履歴'
ls aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/*.md | grep -vE 'README|good-examples|CONSISTENCY-AUDIT' | wc -l
grep -cE '^\| \[' aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/README.md
grep -nE '規則が [0-9]+ 本|^2026-[0-9-]+追加:' aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/README.md
grep -cE '予定（未実装' docs/specs/10-orchestration.md docs/specs/01-domain-model.md
```

メインの基線（2026-09-07）: 1 つ目は全体で 40 件（coding-rules 12 + 10 号 9 + 11 号 5 + 01 号 6 + 12 号 8。あなたの所有範囲では coding-rules 12 + 10 号 9 + 01 号 6 = 27 件の
見込み — 実測値を報告に書く）、22、23（good-examples 行 1 を含む）、:10 と :12 の 2 行、0 / 0。

**Step 2（Green）**: A-1 〜 A-4 を『処置』列どおりに改訂する。各改訂箇所の末尾に出典注記（台帳 ID ではなく ADR / 日付 / 実測所在を書く — 例
`（実測 intent_execution_event.rs:82-112）` / `（B12 2026-08-30 / B13 2026-08-30）` / `（ADR-006）`）。README（BR4.2）は :10 の告知行を「規則が衝突したら」節から
外して一覧表の first-class-collections 行に裁定日として畳み、:12「規則が 13 本」を現行の件数（22）に直し、各行の一言・機械強制が本文と一致するか全行を突合する。

**Step 3（Refactor）**: 出典注記・表の列数・見出し重複を整え、Step 1 の検査を再実行する。期待: 1 つ目は所有ファイルで 0 件、22 / 23（規則行 22）、
「規則が 22 本」の 1 行のみ（告知行なし）、10 号・01 号に `予定（未実装` が各 1 件以上。加えて次を実行して 0 件を確認する。

```bash
grep -nE '同期' docs/specs/01-domain-model.md docs/specs/10-orchestration.md | grep -iE 'rmu|catch_up|投影'
```

表の列数検査は `unit-test-instructions.md` §1 (4) の python スクリプト（所有ファイルだけに絞ってよい）。

## 報告（`developer-report-3.md`、あなただけが書く）

次の節を必ず置く: 「Red 基線」（コマンドと結果）、「改訂一覧」（**改訂 1 件ごとに 1 行**、列 = ファイル:節 / BR / 改訂内容 / 出典注記 / **根拠列: コードの所在
（path:line または型・関数名）/ テストの有無（あればテスト名か件数）/ 仕様の該当節**）、「Green / Refactor 後の検査結果」（Step 3 の実測をそのまま貼る）、
「保留」（根拠を添えられなかった改訂、設計に無い判断が要った点、台帳と食い違うと思った点 — 勝手に決めず列挙する）、「触っていないもの」（『維持』行と
履歴行の確認）。改訂内容は簡潔に、しかし読者が diff を見なくても何を何に変えたか分かる粒度で書く。
