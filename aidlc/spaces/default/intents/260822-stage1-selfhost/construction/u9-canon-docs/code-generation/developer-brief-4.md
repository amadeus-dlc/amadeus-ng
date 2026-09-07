AIDLC-UNIT: u9-canon-docs
AIDLC-TESTING-CONTRACT: sha256:303d9bb7b5d777d54a6761be9ed154d85d5bb3f2d6b9cce02f71f4ed1b3a4ff3

# developer-brief-4 — 派遣 B: 仕様 11 号 + 12 号 + 共有契約 3 本 + ADR-010 注記（U9 再走、2026-09-07）

Conversation language: 日本語（文書本文・注記・報告はすべて日本語。型名 / API 名 / ファイル名 / ID / YAML キー / 逐語文言は英語のまま）。

## 役割と範囲

あなたは aidlc-developer-agent。Unit **u9-canon-docs**（kind: spec）の再走 Bolt、派遣 B を担当する。**コードは書かない** — 所有ファイル（書込可）は次の文書だけ:

- `docs/specs/11-workspace.md`
- `docs/specs/12-workflow-definition.md`
- `aidlc/spaces/default/intents/260822-stage1-selfhost/inception/domain-design/components.md`（全面改訂。ただし `## Review` 見出し（現在 :430）から末尾までは **1 バイトも変えない**）
- `aidlc/spaces/default/intents/260822-stage1-selfhost/inception/contract-design/contract-summary.md`（C1 / C3 / C4 / C5 / C6 / §4 の節単位。`## Review`（現在 :486）以降は不変）
- `aidlc/spaces/default/intents/260822-stage1-selfhost/inception/units-generation/unit-of-work.md`（U3 の記述への**失効注記のみ**、本文は書き換えない。`## Review`（現在 :207）以降は不変）
- `aidlc/spaces/default/intents/260822-stage1-selfhost/inception/domain-design/decisions.md`（**ADR-010 :476-478 の段落への失効注記のみ**。他の ADR・他の段落は触らない）
- 報告: `aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u9-canon-docs/code-generation/developer-report-4.md`（**新規・あなただけが書く**）

それ以外は**読むだけ**。特に `docs/specs/10-*.md` / `01-*.md` と `coding-rules/`（派遣 A が並行して編集中）、`modules/` / `tools/` / `scripts/` / `.github/` / `Cargo.*` /
`formal/`、`docs/specs/research/**`、`docs/specs/deviations.md`、計画 `code-generation-plan.md` / `unit-test-instructions.md` / `code-generation-questions.md` には触らない。

**禁止事項**: `git add` / `git commit` / push / PR / GitHub への書込をしない（コミットはメインが行う）。`.claude/` 配下のツールを実行しない。`AIDLC_*` 環境変数を
設定・変更してフックを回避しない。成果物の保存は Write / Edit ツール経由（シェルのリダイレクトや heredoc で書かない）。書込スコープ外のファイルに触らない。

## 先に読むもの（順に）

1. 規則の束: `aidlc/spaces/default/memory/org.md`、`team.md`、`project.md`（Mandated / Corrections）、`phases/construction.md`。
2. `aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u9-canon-docs/code-generation/code-generation-plan.md`（承認済み。§1〜§5、特に §2 写像表の派遣 B 行と §5.2）
   と同ディレクトリの `unit-test-instructions.md`（受入検査 (2)(4)(6)(7)(9) を自己実行する）。
3. `aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u9-canon-docs/functional-design/gap-measurement-20260907.md` **全文** — §1（コードの現状）、
   §2.2（12 号）/ §2.3（11 号）/ §2.6（共有契約）の表、**§4 の台帳（正しい姿の正本）**、§4.7（未実装）、§4.8（記録間の矛盾の裁き）、§5 と追加実測 1〜4。
4. `aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u9-canon-docs/nfr-design/security-design.md` §2（改訂の作法 — 下に全文転記）と §4（受入検査）。
5. `aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u9-canon-docs/functional-design/rules.md`（BR3.2 / BR3.3 / BR3.7 / BR5.1 / BR5.2 / BR5.3）。
6. 実装の一次出典（読取のみ、主張を書く前に必ず該当箇所を開く）: `Cargo.toml`（members 10）、`modules/core/command/domain/src/{orchestration,workflow_definition,workspace}/`
   （`mod.rs` と `lib.rs:29-31`）、`modules/core/command/use-case/src/orchestration/{port/*.rs,*_use_case.rs,test_support.rs}`、
   `modules/core/command/interface-adapter/src/orchestration/*_repository_impl.rs`、`modules/core/query/use-case/src/`（`Find*UseCase` 13、`*Dao` 14、`directive.rs`）、
   `modules/core/query/interface-adapter/src/`（SQLite DAO / `InMemory*Dao`）、`modules/core/read-model-updater/src/`（`journal_reader.rs`、`sql.rs`、`read_model_updater.rs`、
   `publication_store.rs`、`steering_source.rs`、`workspace/`）、`modules/core/infrastructure/src/`（`canon_json` / `collections`）、`modules/app/aidlc/src/{main.rs,runtime.rs,wording.rs}`、
   `modules/harness/`。

## 正しい姿の正本と、基準の取り方

- **基準は現行コード**（`origin/main` = `02cacea2`、作業ツリーの `modules/` はこれと diff ゼロ — 2026-09-07 メイン実測）。gap-measurement §4 の台帳は 1 件ずつ
  コードで実否確認済みで、**§4 に無い主張は書かない**。記録（Bolt / Unit の設計記録・完了報告・handoff）を転記しない。記録間で食い違う主張は §4.8 の裁きに従う。
- **実測表は 3 列で読む**: §2 の表の『現在の主張』列は**今その行に書いてある語**、『コード』列は**置換後の語**である。行番号は探索の目安であり、同じ節の現在の
  内容を読んで改訂する（レビュアーが列を読み違えて「その行に置換後の語が無い」と誤指摘した実例がある）。
- **主張を書く前にコードを開く**。型名・関数名・変種数・属性数・表名・クレート名は、該当ファイルを Read して確認してから書く。確認できない主張は書かず、
  報告の「保留」に載せる。components.md の全面改訂は特に、YAML の 1 コンポーネントごとに対応するクレート・モジュールを開いて書く。

### メインの基線実測で判明した訂正（gap-measurement を上書きする — 必ず反映）

- **T1. テストダブルの記述**（gap-measurement §1「テストダブル型 `InMemoryXxxRepository` は無く」/ §4.2 P4 の訂正）: `modules/core/command/use-case/src/orchestration/test_support.rs`
  は `#[cfg(test)]`（`orchestration/mod.rs:38-39`）の `pub(crate)` フェイク 4 つ（`InMemoryIntentExecutionRepository` / `InMemoryIntentRepository` /
  `InMemoryWorkflowDefinitionRepository` / `InMemoryCompiledDefinitionRepository`）を持ち、ユースケースの単体テスト（例 `commit_verdict_use_case.rs:317-344`）は
  これを使う。理由は同ファイル冒頭 doc（:1-17）— `core-command-use-case` は `core-command-interface-adapter` を dev-dependency にも書けない（DIP のクレート分離強制）
  ため、**DIP 制約下の use-case 単体テスト専用**の trait フェイク。**公開のインメモリ実装はアダプタ層の `XxxRepositoryImpl<S>::in_memory()`（本家 memory
  バックエンド）が正**で、自作 HashMap ダブルはそれ以外で禁止（オーナー裁定 2026-08-31）。クエリ側は `InMemoryXxxDao` 13 が公開のテストダブル。
  → `unit-of-work.md:144`「テストは `XxxUseCase<InMemoryWorkflowExecutionRepository>` の素の値で組む」への注記は「名称は改名（`InMemoryIntentExecutionRepository`、
  `#[cfg(test)]` の crate 私有フェイク）。公開のインメモリ実装はアダプタ層の `in_memory()`」と書く。components.md / contract-summary でテストダブルに触れる箇所も同じ 3 層で書く。
- **T2. `Rehydrated*`**: 型としては 0 件。`port/mod.rs:15-18` と `port/intent_execution_repository.rs:36-38` の doc に「廃止済み（オーナー裁定 2026-08-30）」の
  言及が残るだけ。`expected_version` はポートの引数には無く、アダプタ実装内のローカル変数（`let expected_version = aggregate.version();`）だけ。
- **T3. `JournalReader`**: メソッド 9 = `async fn` 8（events_after / events_through / checkpoint / advance_checkpoint / publish / pending_publication /
  steering_source_digest / replace_steering）+ 同期 `fn prepare_read_model(&mut self)` 1（`journal_reader.rs:38`）。「async fn 9」と書かない。
- **T4. FCC**: ドメイン 16 型（orchestration 8 / workspace 6 / workflow-definition 2）に加え、`core-infrastructure` に汎用の `Collection<T>` / `NonEmptyCollection<T>`
  （`collections/`、#114）と `canon_json` の `ObjectMembers` が `FirstClassCollection` を実装する。components.md の `core-infrastructure` には `collections`
  （`FirstClassCollection` trait + `Collection<T>` / `NonEmptyCollection<T>`）として書く（K4）。
- **T5. ランタイム**: `modules/app/aidlc/src/main.rs:12` `#[tokio::main(flavor = "current_thread")]`。`tokio::spawn` / `spawn_blocking` は `modules/` に 0 件。
  ポート 4 の動詞はすべて `async fn`。

### 訂正 2 件（security-design §3 (c) — 必ず反映）

1. **RMU 呼出の用語**: 「ポート・ユースケース・RMU は `async fn`、駆動ループ・`tokio::spawn` を持たず、合成ルート（`aidlc`）が読取前と書込後に `catch_up` を await で
   直列に呼ぶ（`runtime.rs:194,200,238`）」と書き、**「同期呼出」「同期 `catch_up`」と書かない**（ADR-006「async は初期化から、ドメインは同期」は有効）。
   「同期」を使えるのは、集約が I/O を持たず純粋であることを述べる文だけ（components.md:30 の「集約は … 純粋・同期」はそのまま）。
2. **ADR-010 の失効注記**（`decisions.md:476-478`、現文は下記）: この 3 行の段落を `~~…~~` で打ち消し、直後に
   `— 失効（2026-08-30 / B13: 楽観 version は集約の内側 — `version: usize` フィールド、`version()` / `with_version()`、`UNPERSISTED_VERSION = 0`。`Rehydrated*` /
   `StatePosition` / `StoreVersion` は撤去、`store(&mut self, &event, &aggregate)` に `expected_version` 引数は無い。実測 `port/mod.rs:15-18` / `intent_execution_repository_impl.rs:455`）`
   を追記する。**削除しない**。ADR-010 の他の段落（TOCTOU の経緯など）と他の ADR は触らない。

   ```text
     楽観 `version` は集約と memento（`WorkflowExecutionState`）から削除し、**集約の外**を持ち回る
     形にした — `find_by_id` は再水和レコード `RehydratedWorkflowExecution`（集約 + ストア採番
     version）を返し、`store` は `expected_version: usize` を引数に取る。
   ```

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

補足: クリティカルパスの番号は `aidlc-state.md` の「クリティカルパス」節（2 = workspace 実装スライス、4 = マルチコール CLI + 文言カタログ配線、5 = 最小フック、
6 = doctor → ドッグフード）。workspace 集約 3 / 供給面 4 / `intents.json` は項目 2、unpark / jump / recompose は項目 4、フック 4 本は項目 5、doctor は項目 6、
Bolt / SwarmBatch はスコープ外（swarm）なので `予定（未実装、swarm はスコープ外）` のように書く。

## 作業対象（gap-measurement §2 の表を 3 列丸ごと — 行番号は 2026-09-07 実測）

### B-1. `docs/specs/11-workspace.md`（226 行）

| 行 | 現在の主張 | コード | 処置 |
|---|---|---|---|
| 3-15 | 冒頭 2 注記（B12 読み替え・B13 優先順位） | — | 本文へ畳み込み、「追従済み（2026-09-07 / U9 再走）」の 1 行に縮める（旧注記は `~~…~~` で 1〜2 行に圧縮して残してよい） |
| 41-49 | §2.1 集約 `Intent` / `Space` / `Worktree`（登録簿 `intents.json` の Intent） | workspace モジュールに集約は無い（値オブジェクト + FCC のみ — S1 / S2）。コードの `Intent` は orchestration の静的 intent 集約（O2）であり登録簿の Intent とは別物 | **予定**と明記し、orchestration の `Intent` との関係（同名・別物）を注記（S4） |
| 47 / 49 | 「監査台帳は `WorkflowExecution` のイベントログ」「`WorkflowExecution` 集約の書込は SQLite Tx」 | `IntentExecution` | 名称のみ |
| 51-69 | §2.2 Domain Primitive（`IntentId` UUIDv7 / `IntentDirName` / `CheckboxState` / `BoltRefs` / `StateVersion` / `AuditFieldKey` …） | 一致 | **維持** |
| 70-85 | §2.3 描画関数は RMU へ移動済み、順序付け純関数はドメイン | 一致（`OrderedAuditEvents` FCC / `StateVersionClassification`） | **維持**（RMU 呼出の用語だけ確認 — 「同期」があれば訂正 1 の形へ） |
| 101 | §3 ポート表 `WorkflowExecutionRepository` 行（`expected_version` 引数） | `IntentExecutionRepository { find_by_id(&IntentExecutionId) -> IntentExecution, store(&mut self, &IntentExecutionEvent, &IntentExecution) }`（P1） | 要改訂 |
| 111-116 | 供給面 `WorktreeService` / `OpaqueFlagStore` / `ScopedStorage` / `SessionStampStore` | 未実装 | **予定**と明記（S4） |
| 126 / 195 | §4 Gateways 末尾・§7 実装順序の `WorkflowExecutionRepositoryImpl` / `InMemory…` | `IntentExecutionRepositoryImpl`、`in_memory()`（T1 の 3 層構造） | 名称のみ |

### B-2. `docs/specs/12-workflow-definition.md`（278 行）

| 行 | 現在の主張 | コード | 処置 |
|---|---|---|---|
| 3-15 | 冒頭 2 注記 | — | 畳み込み（11 号と同じ形） |
| 42-56 | §2.1 `WorkflowDefinition` / `CompiledDefinition` | 一致（属性・イベント・ID の別型 — W1 / W2） | **維持** |
| 77-93 | §2.3 述語 6 + `grid().action()`、`WorkflowExecution` の `effective_plan` / `stages`（89, 91） | 述語は一致（加えて `scope_cost` / `review_policy(slug, scope, override)` / `stage_route` が集約のクエリ — W3）。所有者は `IntentExecution::effective_plan`、計画は `Intent.stages: StageEntries` | 名称改訂 + クエリ 3 件追記 |
| 164-165 / 215 / 221 | §4 #7・§8 F2 / F8 の `WorkflowExecution` | `IntentExecution` / `Intent`（どちらかはコードで確認 — 計画側の主張なら `Intent`） | 名称のみ |
| 178 | §5 ユースケース「`LoadStageGraph` / `LoadScopeCatalog` / `ResolveScopePlan`（読み取り専用）」 | コマンド側 `DefineWorkflowUseCase`、クエリ側 `FindDefinition` / `FindDefinitionStage` / `FindScope` / `FindScopeKeyword` / `FindPhaseEntry`、RMU `read_definition*` 6 表（P5 / P6 / R3） | 要改訂 |
| 183 | `WorkflowDefinitionRepository` 動詞 2 | `find_by_id` / `find_for_intent(&Intent)` / `store`（P1） | 1 動詞追記 |

### B-3. `inception/domain-design/components.md`（457 行、`## Review` :430 以降は不変）

全面改訂（BR3.7 (a)）。ずれ: 冒頭注記「再構成はジャーナル全再生」（コードは最新スナップショット + 差分再生 — O9、§4.8）。YAML: `OrchestrationEngine` の集約
`WorkflowExecution` + イベント 11 / `EngineUseCases`（`NextUseCase` / `ReportUseCase` / `ContinueUseCase` / `DoctorUseCase` / フック 4）/ `WorkflowDefinitionModel`
「ES 対象外・find のみ・`next_in_scope_stage`」/ `PublishedLanguage` の message-catalog / `RehydratedWorkflowExecution`。**クエリ側（`core-query-*`、DAO 14、`Find*` 13）と
`CompiledDefinition`・`Intent` 集約・`read_*` 17 表がコンポーネントとして存在しない**。

正しい姿（台帳から）: クレート 10（K1 — `core-command-domain` / `core-command-use-case` / `core-command-interface-adapter` / `core-query-use-case` /
`core-query-interface-adapter` / `core-read-model-updater` / `core-infrastructure` / `aidlc` / `harness-claude` / `harness-infrastructure`）、CQRS 境界はクレート分離で
物理強制（K2）、ドメインは永続化知識から中立（K3）、infrastructure は言語拡張のみ（K4 + T4）、契約 JSON は canon_json（K5）。集約 4（O1〜O5、W1 / W2）、
イベント 16・1・2・4、コマンド側ポート 4 / ユースケース 9（P1 / P5）、クエリ側ユースケース 13 / DAO 14 / SQLite DAO 14 + `InMemoryXxxDao` 13（P6）、文言は出す側の
`wording`（P7）、RMU = 中間クレート `JournalReader`（T3）+ 投影核 + `read_*` 17 表 + `amadeus_*` 表（R1〜R7）、CLI 面（P9）、workspace は値オブジェクト + FCC
（S1〜S3）、未実装は予定（S4 / §4.7）。既存 YAML の構造（コンポーネント名 / `summary` / `behaviour` / `responsibilities` / `depends_on` / `dependents` /
`## Component Summary` 表など）は保ちつつ中身を現行へ。旧コンポーネント名は `~~旧名~~ — 改名（B12 2026-08-30）` の形で履歴を残す。**`## Review` 見出しから末尾は
Read して形を確認し、1 バイトも変えない**（改訂後に `tail -n +<Review 行>` の sha256 が改訂前と同じであることを自分で確認する — 基線 `b954159a08ef6a312306efa0ec9ddaeecd0403e7b103b12a7eca13614bdc1dbb`）。

### B-4. `inception/contract-design/contract-summary.md`（522 行、`## Review` :486 以降は不変）

節単位の現行化（BR3.7 (b)）: C3 — trait 全文が v2 世代（`WorkflowExecutionRepository`、`find_by_id(&IntentId)`）+ 追記 4 層、2026-08-30 追記「ジャーナル全再生」（O9 の
差分再生へ）、`Rehydrated…` 3 件（T2 — 履歴化）。C4 — `WorkflowExecution::definition_id()`（→ `Intent::definition_id()`、O1 / O2）。C5 — `WorkflowExecutionEvent` 11 変種列挙
（→ `IntentExecutionEvent` 16、O5 / O6 / O7）。C6 — 「16 属性」注記（→ 12 属性、O3）、`read_*` 17 表 / `amadeus_read_model_head` / publication 表の注記（R3 / R4 / R7）。
C1・§4 — message-catalog 3 件（→ 出す側の `wording`、P7）。旧 manifest 綴り `workflow-execution-event/1`（:285 / :305 / :388）は既に打消し線つき履歴なので触らない。
「同一シャード内の直接行と投影行の順序」は未決のまま §4 に残す。`## Review` 以降は不変（基線 sha256 `7374b976335767b3356452648e3ad92fddf1f8730e9f5f070d8aabc75f8f3662`）。

### B-5. `inception/units-generation/unit-of-work.md`（235 行、`## Review` :207 以降は不変）

注記のみ（BR3.7 (c)）— 本文は書き換えず、行末または直後に `（失効 …）` / `（改名 …）` の括弧注記を添える。対象 5 行:

| 行 | 現在 | 注記の内容 |
|---|---|---|
| :34 | U3 行「SQLite EventStore と WorkflowExecutionRepository」 | 改名（`IntentExecutionRepository`、B12 2026-08-30） |
| :64 | 「`version` は失効（2026-08-29 / ADR-010・Bolt B7）: 楽観 version は集約の外へ — `RehydratedWorkflowExecution` が持ち回る」 | この注記自体が B13（2026-08-30）で再失効 — version は集約の内側（`version()` / `with_version()`）、`Rehydrated*` 撤去 |
| :83 | 「独自スキーマ …（ADR-007）。`InMemoryWorkflowExecutionRepository` を先に書く（gateway-taxonomy §6）」 | 失効 — スキーマは本家 event-store-adapter-rs（`=3.0.0`、R4）、インメモリは `XxxRepositoryImpl::in_memory()`（T1） |
| :91 | 「ポート trait（`WorkflowExecutionRepository`、EventStore 同形 trait）はユースケース層に置く」 | 改名（`IntentExecutionRepository`）。置き場（use-case 層 `port/`）は現行どおり |
| :144 | 「テストは `XxxUseCase<InMemoryWorkflowExecutionRepository>` の素の値で組む」 | 改名（`InMemoryIntentExecutionRepository`、`#[cfg(test)]` の crate 私有フェイク — T1）。公開のインメモリ実装はアダプタ層の `in_memory()` |

`## Review` 以降は不変（基線 sha256 `b8d0e4a122bab43d0123c1a0515536e2866d05eb7609f6abcbb87c91dd503880`）。

### B-6. `inception/domain-design/decisions.md`（523 行）

訂正 2 の 1 段落のみ（:476-478）。diff が当該段落の打消し線 + 追記だけであることを `git diff` で確認して報告に貼る。

## 手順（計画 §5.2 の Step 4〜6）

**Step 4（Red）**: 改訂前に次を実行し、結果を `developer-report-4.md` の「Red 基線」に記録する。

```bash
grep -rnE 'effective_plan_action|next_in_scope_stage|AuditLedgerRepository|AuditLedgerService|StateFileStore|report_forward|gate_start|WorkflowExecution|RehydratedWorkflowExecution|message-catalog' docs/specs/11-workspace.md docs/specs/12-workflow-definition.md | grep -vE '~~|旧|失効|是正済み|改名|履歴'
grep -cE '予定（未実装' docs/specs/11-workspace.md docs/specs/12-workflow-definition.md
grep -nE '^## Review$' aidlc/spaces/default/intents/260822-stage1-selfhost/inception/domain-design/components.md aidlc/spaces/default/intents/260822-stage1-selfhost/inception/contract-design/contract-summary.md aidlc/spaces/default/intents/260822-stage1-selfhost/inception/units-generation/unit-of-work.md
tail -n +430 aidlc/spaces/default/intents/260822-stage1-selfhost/inception/domain-design/components.md | shasum -a 256
tail -n +486 aidlc/spaces/default/intents/260822-stage1-selfhost/inception/contract-design/contract-summary.md | shasum -a 256
tail -n +207 aidlc/spaces/default/intents/260822-stage1-selfhost/inception/units-generation/unit-of-work.md | shasum -a 256
```

メインの基線（2026-09-07）: 1 つ目は 11 号 5 + 12 号 8 = 13 件の見込み（実測値を書く）、0 / 0、:430 / :486 / :207、sha256 は上記 3 値。共有契約 3 本の sentinel
出現（`WorkflowExecution` 等）も参考として `grep -c` で数えて記録する（inception は受入 (2) の grep 範囲外だが、改訂の網羅の目安になる）。

**Step 5（Green）**: B-1 〜 B-6 を改訂する。各改訂箇所の末尾に出典注記（ADR / 日付 / 実測所在 — 例 `（実測 sql.rs / read_* 17 表）` / `（B12 2026-08-30 / B13 2026-08-30）` /
`（ADR-006）`）。components.md は 1 コンポーネントごとに対応するクレート・モジュールを Read してから書く。

**Step 6（Refactor）**: 出典注記・表整形・見出し重複を整え、Step 4 の検査を再実行する。期待: 1 つ目は 0 件、11 号・12 号に `予定（未実装` が各 1 件以上、
`## Review` の行番号は変わってよいが `tail -n +<新しい行番号> | shasum -a 256` の 3 値が基線と**同一**、`git diff -- <decisions.md>` が ADR-010 の段落だけ。加えて:

```bash
grep -nE '同期' docs/specs/11-workspace.md docs/specs/12-workflow-definition.md aidlc/spaces/default/intents/260822-stage1-selfhost/inception/domain-design/components.md aidlc/spaces/default/intents/260822-stage1-selfhost/inception/contract-design/contract-summary.md | grep -iE 'rmu|catch_up|投影'
```

が 0 件。表の列数検査は `unit-test-instructions.md` §1 (4) の python スクリプト（所有ファイルだけに絞ってよい）。

## 報告（`developer-report-4.md`、あなただけが書く）

次の節を必ず置く: 「Red 基線」（コマンドと結果）、「改訂一覧」（**改訂 1 件ごとに 1 行**、列 = ファイル:節 / BR / 改訂内容 / 出典注記 / **根拠列: コードの所在
（path:line または型・関数名）/ テストの有無（あればテスト名か件数）/ 仕様の該当節**。components.md はコンポーネント 1 つを 1 行）、「Green / Refactor 後の検査結果」
（Step 6 の実測をそのまま貼る — sha256 の 3 値と decisions.md の diff を含む）、「保留」（根拠を添えられなかった改訂、設計に無い判断が要った点、台帳と食い違うと
思った点 — 勝手に決めず列挙する）、「触っていないもの」（『維持』行・履歴行・`## Review` 節の確認）。改訂内容は簡潔に、しかし読者が diff を見なくても何を何に
変えたか分かる粒度で書く。
