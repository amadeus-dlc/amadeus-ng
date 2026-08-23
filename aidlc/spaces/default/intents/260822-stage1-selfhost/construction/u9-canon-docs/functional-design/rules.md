# rules — U9 正本・仕様の canon 追従（`u9-canon-docs`）

> Functional Design（Construction 3.1）成果物（Unit: U9、kind: spec、Bolt: B4）。出典: `../../../inception/units-generation/unit-of-work.md`（U9 の
> 責務・合格）、`../../../inception/units-generation/unit-of-work-story-map.md`（FR8.1 → FR8.2 → FR9.6 の順）、`../../../inception/requirements-analysis/
> requirements.md`（FR8.1 の 4 点、FR8.2 の列挙、FR9.6）、`../../../inception/domain-design/decisions.md`（ADR-001〜008）、`../../../inception/
> contract-design/contract-summary.md`（C4 `find_by_id`、C5 `Started`）、`aidlc/spaces/default/knowledge/aidlc-shared/design-audit-2026-08-22.md`
> （R1〜R5、C1〜C12）、`../../../inception/practices-discovery/evidence.md`（FR9.6 ドラフト）、Bolt B3 の実装（`../../u2-domain-es-core/code-generation/
> code-summary.md`）、確認質問 `functional-design-questions.md`（Q1〜Q3 = A、追加 1・2）。
>
> 下の fenced `yaml` が正本。BR1.x = A 束（FR8.1）、BR2.x = B 束の元の列挙（FR8.2）、BR3.x = B 束の追加分（Q3 = A、追加 1・2）、BR4.x = FR9.6、
> BR5.x = 合格条件と作法。

## 1. 規則（正本）

```yaml
rules:
  # --- BR1: A 束 — canon 語彙の自己矛盾修正（FR8.1） ---
  - id: BR1.1
    statement: "`coding-rules/use-case-rules.md` §4（38 行目）の例示 `repository.load()` を Repository 語彙に合わせ `repository.find_by_id()` に直す（`find()` は C4 改訂で廃止済み）"
    category: policy
    applies_to: [CodingRule(use-case-rules.md)]
    trigger: "B4 の文書改訂"
    logic: "IF 正本の例示が §2b の許容動詞（find_by_id / find / save / remove / store）に無い語を使う THEN 許容動詞へ置換"
    violation: "レビューで差し戻し（grep `repository.load()` = 0 件が合格）"
    source: "FR8.1, 設計監査 C1"
  - id: BR1.2
    statement: "`coding-rules/gateway-taxonomy.md` §4 の散文「単一の Repository が集約の load / save を持つ」を「find / save」に直す（§5 末尾の「load / save の指揮」も同様）"
    category: policy
    applies_to: [CodingRule(gateway-taxonomy.md)]
    trigger: "B4 の文書改訂"
    logic: "同上（§2b の語彙に揃える）"
    violation: "grep `load / save` = 0 件（gateway-taxonomy.md）"
    source: "FR8.1, 設計監査 C2"
  - id: BR1.3
    statement: "`coding-rules/gateway-taxonomy.md` §2b の許容動詞一覧に ES Repository の拡張語彙 `store`（event-store-adapter-rs 同形。§2b はステートソーシング Repository の規則であり、ES Repository の動詞は本家ライブラリの語彙に従う）を注記として追加する"
    category: policy
    applies_to: [CodingRule(gateway-taxonomy.md)]
    trigger: "B4 の文書改訂"
    logic: "§2b に『ES Repository（WorkflowExecutionRepository）は `store(event, aggregate)` / `find_by_id` — ADR-006』の 1 段落を足す"
    violation: "レビューで差し戻し"
    source: "FR8.1, ADR-006 Consequences"
  - id: BR1.4
    statement: "`coding-rules/gateway-taxonomy.md` §2 の実例リストから旧称 `AuditLedgerRepository` の行を除去し、`AuditLedger` はイベントログ（ADR-001 / 003）であって集約ではない旨を 1 行注記する"
    category: policy
    applies_to: [CodingRule(gateway-taxonomy.md)]
    trigger: "B4 の文書改訂"
    logic: "§2 の箇条 `AuditLedger → AuditLedgerRepository` を削除し注記に置換"
    violation: "grep `AuditLedgerRepository` = 0 件（coding-rules/ と docs/specs/ の双方 — 11 号 §3 は BR2.1 で同時に除去）"
    source: "FR8.1（requirements.md レビュー所見どおり §2）, ADR-006"

  # --- BR2: B 束 — 仕様の canon 追従（FR8.2 の元の列挙） ---
  - id: BR2.1
    statement: "11 号 §3 ユースケース層のポート表・供給面表を gateway-taxonomy 語彙へ書き直す: ポートは `WorkflowExecutionRepository`（ES 形 store / find_by_id、C3）と外部システムクライアント（Git）だけ。`FileStore`（アトミック書込・追記 open）は Repository 実装の内部部品、`Clock` / `ProcessProbe` / `Tmpdir` はアダプタ層の機構（ポートではない）、`AuditLedgerService` は退役（監査シャードは ReadModelUpdater の投影 — FR1.1）、`GitPort` は外部システムクライアント名（例 `GitWorktreeClient`）へ"
    category: policy
    applies_to: [SpecDocument(11-workspace.md)]
    trigger: "B4 の文書改訂"
    logic: "§3 の表を『ポート | 消費するユースケース | 契約 | 実装の所在』の 4 列で再構成し、機構は §4 アダプタ層へ移す"
    violation: "Store / Reader / Writer / Source / Provider の造語、媒体名の Repository、機構のポート化が残ればレビューで差し戻し"
    source: "FR8.2, 設計監査 R3 / C3 / C4 / C11, ADR-003 / 004 / 007"
  - id: BR2.2
    statement: "01 号 §3 の集約候補表を現行の裁定に合わせる: §3.1 `WorkflowDefinition`（集約ルート、id / revision つき — ADR-008）、§3.2 `WorkflowExecution`（ES 形 FSM — ADR-002 / 004、PlanAction は workflow_definition 所有 — ADR-005）、§3.3 は BR3.2（ES 化後の姿）に従う"
    category: policy
    applies_to: [SpecDocument(01-domain-model.md)]
    trigger: "B4 の文書改訂"
    logic: "各節の『集約候補』段落を裁定済みの集約 / 値オブジェクト / リードモデルに振り分けて書き直す"
    violation: "ADR と矛盾する集約候補が残ればレビューで差し戻し"
    source: "FR8.2, 設計監査 C6 / C12, ADR-001〜008"
  - id: BR2.3
    statement: "10 号 §3 ポート表の実装欄『同上』を廃し、1 trait 1 Impl（`XxxRepositoryImpl` + `InMemoryXxxRepository`）を各行に明記する。`WorkflowDefinitionRepository` の動詞は `find_by_id`（C4）"
    category: policy
    applies_to: [SpecDocument(10-orchestration.md)]
    trigger: "B4 の文書改訂"
    logic: "表の各行に実装名を書く（gateway-taxonomy §5 の命名）"
    violation: "『同上』が残ればレビューで差し戻し"
    source: "FR8.2, 設計監査 C11, C4 改訂"
  - id: BR2.4
    statement: "10 号 §2.2 / 12 号 §2.2 / 01 号 §3 で `PlanAction` の所有を workflow_definition に一意化し（orchestration は利用のみ、再輸出なし — ADR-005 改訂、Bolt B3 実装済み）、`CheckboxState` の所有を workspace（値オブジェクト）に一意化する。10 号 §2.2 の表は『所有元: workflow_definition』の参照行にする"
    category: policy
    applies_to: [SpecDocument(10-orchestration.md), SpecDocument(12-workflow-definition.md), SpecDocument(01-domain-model.md)]
    trigger: "B4 の文書改訂"
    logic: "所有コンテキストの節にだけ定義を置き、他は参照と明記"
    violation: "2 か所以上で定義されていればレビューで差し戻し"
    source: "FR8.2, 設計監査 C12 / R1"
  - id: BR2.5
    statement: "12 号 §2.3 / §5 / 実装ノートの整合: `next_in_scope_stage` 行を削除（畳み込みは集約 `WorkflowExecution::effective_plan` / `next_decision` — R2、Bolt B3 で削除済み）、`StageGraphQuery` / `StageNodeView` / `SensorBindingView` の個別名を廃し『集約の述語面』の記述へ（C9）、集約昇格の第一理由を『3 入力は compile が lockstep で出す（一貫性単位）』へ（C10）、§5 の `find` → `find_by_id`"
    category: policy
    applies_to: [SpecDocument(12-workflow-definition.md)]
    trigger: "B4 の文書改訂"
    logic: "該当行を削除・改訂し、残す述語 6 つ（is_valid_scope / valid_scopes / scope_metadata / subgraph_for_scope / stages_in_scope / first_in_scope_stage_of_phase）と `grid().action()` を列挙"
    violation: "削除済みメソッド名が規範として残ればレビューで差し戻し"
    source: "FR8.2, 設計監査 C8 / C9 / C10, FR8.4"

  # --- BR3: B 束の追加分（Q3 = A、追加 1・2） ---
  - id: BR3.1
    statement: "ADR-008 を仕様へ: 12 号 §2.1 の `WorkflowDefinition` 行に識別子 `WorkflowDefinitionId`（`<harnessRoot>/tools/data/harness.json` の `name` — 内容が変わっても不変のエンティティ ID）と内容版 `DefinitionRevision`（3 入力の正準 JSON の `sha256:`、値属性）を追記、§5 と 10 号 §3 のポートを `find_by_id(&WorkflowDefinitionId)`（`find()` 廃止、`NotFound` / `HarnessIdentity` の失敗態度）に改訂、01 号 §3.1 の Domain Primitive 候補に 2 型を追加"
    category: policy
    applies_to: [SpecDocument(12-workflow-definition.md), SpecDocument(10-orchestration.md), SpecDocument(01-domain-model.md)]
    trigger: "B4 の文書改訂"
    logic: "ADR-008 Decision (1)〜(3) と Bolt B3 実装（`workflow_definition_id.rs` / `definition_revision.rs` / `find_by_id`）を出典に書く"
    violation: "エンティティ（集約）に ID が無い記述が残ればレビューで差し戻し"
    source: "ADR-008, C4 改訂, オーナー裁定 2026-08-23"
  - id: BR3.2
    statement: "ES 化（ADR-001〜007）の帰結を 01 号 §3.3 と 11 号 §2 へ: workspace の集約は `Intent`（intents.json の登録 — uuid / slug / dirName と生死、birth の単一チョークポイント）/ `Space` / `Worktree`（構築）。`StateFile` と `AuditShard` はリードモデル（ReadModelUpdater の投影、真実源は SQLite ジャーナル）、`WorkspaceLock` は退役（SQLite Tx + 楽観 version）。Domain Primitive: `IntentId` = UUIDv7（01 号維持、Q2 = A）、`IntentDirName` = 記録ディレクトリ名（kebab、投影のパス解決用 — 新設）、`SpaceName` / `CloneId` / `ShardName` / `StateFieldValue` / `CheckboxState` / `StateVersion` は値オブジェクトとして残す。11 号 §2.3『ドメインサービス（純関数）』の状態ファイル描画関数は投影（U4）の責務へ移す旨を明記"
    category: policy
    applies_to: [SpecDocument(01-domain-model.md), SpecDocument(11-workspace.md)]
    trigger: "B4 の文書改訂"
    logic: "集約候補段落とドメイン層の節を書き直す。U2 実装の `IntentId`（dirName 受理）の是正は Bolt B5（U3）で行う旨を 01 号の脚注に書く"
    violation: "リードモデルを集約に、退役済みロックを現行の規範に書いた記述が残ればレビューで差し戻し"
    source: "ADR-001 / 003 / 004 / 007, Q2 = A, 追加 1（オーナー質問への回答）"
  - id: BR3.3
    statement: "Bolt B3 で確定した規範を 10 号 / 12 号へ: (a) ゲート判定は `gated(stage) = phase ≠ initialization`（索引 0 の特別扱いなし。Quint slice-1 の stage 0 は ITF 用合成計画上の抽象）、(b) `Started` は自己完結（definition_id / definition_revision / scope / request / depth? / test_strategy? / stages = StageEntry 列）でリプレイに定義を要しない、(c) 有効プランの畳み込み（`effective_plan` = overlay）は集約 `WorkflowExecution` の所有（12 号 B1 の『呼び出し側』= 集約と明記）、(d) `WorkflowExecution` の状態 16 属性・12 イベント・decide / apply_event / snapshot / from_snapshot・`next_decision` の優先順（Result 型、DefinitionMismatch）— 10 号 §2.1 を ES 形に書き換える"
    category: policy
    applies_to: [SpecDocument(10-orchestration.md), SpecDocument(12-workflow-definition.md)]
    trigger: "B4 の文書改訂"
    logic: "U2 機能設計（entities / rules / functional-spec）と code-summary を出典に、仕様は『構造の規範』として写す（逐語の upstream 契約は不変）"
    violation: "旧 API（report_forward 等）や stage 0 特別扱いが規範に残ればレビューで差し戻し"
    source: "Bolt B3 実装, U2 機能設計 BR1.3 / BR2.2 / BR2.5 / BR4.2, ADR-002"
  - id: BR3.4
    statement: "`docs/specs/deviations.md` の表に 1 行追加: 分類『設計変更』、upstream『テキストファイル群 + mkdir ロック』、amadeus-ng『SQLite ジャーナル（真実源）+ 楽観 version、ロック dir は生成しない、`aidlc-state.md` / 監査シャードはリードモデルとして維持（バイト互換）』、理由『ADR-001 / 003 / 004 / 007（NFR1 の逸脱登録）』、記録『2026-08-23 / ADR-003, ADR-007』。予約行の整理も行う"
    category: policy
    applies_to: [SpecDocument(deviations.md)]
    trigger: "B4 の文書改訂"
    logic: "表の列形式（# / 分類 / upstream / amadeus-ng / 理由 / 記録）を守って追記"
    violation: "NFR1 の逸脱台帳に未登録のまま U3 が SQLite を書けばレビューで差し戻し"
    source: "NFR1, ADR-003 / 007, requirements.md NFR1 注記"
  - id: BR3.5
    statement: "`components.md` の `WorkspaceModel` を『workspace 語彙（値オブジェクト）』に縮退させる: summary / behaviour / responsibilities を値オブジェクト群（SpaceName / CloneId / ShardName / StateFieldValue / CheckboxState / StateVersion / IntentId / IntentDirName）に限定し、状態ファイル描画の関数群は ReadModelUpdater（U4）の責務へ移す方針を明記（コードの移動は U4 の Bolt B6 で実施 — 本 Unit ではコードを触らない）"
    category: policy
    applies_to: [DesignCatalogue(components.md)]
    trigger: "B4 の文書改訂"
    logic: "components.md の YAML ブロックの該当エントリを改訂し、ReadModelUpdater の responsibilities に『状態ファイル・チェックボックスの描画（旧 WorkspaceModel の純関数）』を追加"
    violation: "『純関数』の寄せ集めが集約でも値オブジェクトでもない形で残ればレビューで差し戻し"
    source: "追加 1（オーナー質問『WorkspaceModel は集約か』2026-08-23）"
  - id: BR3.6
    statement: "01 号 §7『クリーンアーキテクチャへの写像原則』にドメインモデルの原則を明記: (1) ドメインモデルは集約（エンティティ）と値オブジェクトが主役、(2) 純粋関数としてのドメインサービスは消極的に使う（集約に置けない横断の判断のみ）、(3) ドメインモデル・ドメインサービスは永続化責務を持たない（永続化を呼ばない）、(4) 永続化の指揮はユースケース層（Repository の trait はユースケース層、実装はアダプタ層、Tx の所有は実装・呼出はユースケース）、(5) 集約間の依存は ID による間接参照（ADR-008）、(6) 集約は FSM として設計する（project.md の統一ルール）"
    category: policy
    applies_to: [SpecDocument(01-domain-model.md)]
    trigger: "B4 の文書改訂"
    logic: "§7 に『ドメインモデルの原則』の小節を追加し、coding-rules（use-case-rules / gateway-taxonomy / domain-equality / tell-dont-ask）への相互参照を付ける"
    violation: "原則に反する記述が 01 号の他節に残ればレビューで差し戻し"
    source: "追加 2（オーナー確認 2026-08-23）, project.md Corrections（集約 = FSM）, ADR-008"

  # --- BR4: FR9.6 エラーハンドリング様式規則 ---
  - id: BR4.1
    statement: "`coding-rules/error-handling.md` を新設する（Q1 = A の改訂ドラフトのまま）: ドメイン層・ユースケース層の失敗はモジュールごとの手実装エラー enum、`thiserror` / `anyhow` 等に依存しない、`Display` と `std::error::Error` を手実装、`Display` は材料（ID・索引・状態・原因）だけを描く開発者向け診断表示で利用者向けの逐語文言はアダプタ層（message-catalog）、変種フィールドは材料のみ（文言 `String` を運ぶ変種を作らない）、fallible な公開関数には `# Errors`、`# Panics` を要する公開関数は作らない（範囲は型で保証）。根拠（依存最小化、Always Valid、R4）、適用例（B1 / B3 のエラー型）、機械強制（`missing_errors_doc` / `missing_panics_doc` / `unwrap_used` / `expect_used` deny、thiserror / anyhow 禁止は `cargo lint` 候補 — 赤例テスト必須）、裁定日 2026-08-23 を記す"
    category: policy
    applies_to: [CodingRule(error-handling.md)]
    trigger: "B4 の文書追加"
    logic: "既存ルールファイルの書式（裁定日 / 適用例 / 機械強制 / ルール / 根拠 / 対象外）に合わせる"
    violation: "Q1 = A の文面から逸脱すればオーナー再確認"
    source: "FR9.6, Q1 = A, evidence.md 確定アクション 5, R4, Bolt B1 ゲート裁定（Error 手実装）"
  - id: BR4.2
    statement: "`coding-rules/README.md` の一覧表に `error-handling.md` の行（一言 / 機械強制）を追加し、同表の他行（gateway-taxonomy の『Store/Reader/Writer 造語と媒体名は禁止』等）と BR1.x の改訂を同期する"
    category: policy
    applies_to: [CodingRule(README.md)]
    trigger: "B4 の文書改訂"
    logic: "表の行数 = ルールファイル数"
    violation: "README と実ファイルが不一致ならレビューで差し戻し（U9 の合格条件）"
    source: "FR8.1 / FR9.6 合格条件, unit-of-work U9"

  # --- BR5: 合格条件と作法 ---
  - id: BR5.1
    statement: "合格 = (a) 各改訂がレビューで確認できる、(b) `coding-rules/README.md` の一覧と各ファイルが矛盾しない、(c) 仕様 4 号 + deviations が自己整合（削除済み API 名・退役済み機構・旧称が規範として残らない — grep: `effective_plan_action` / `next_in_scope_stage` / `AuditLedgerRepository` / `AuditLedgerService` / `StateFileStore` / `StageGraphReader` / `report_forward` / `gate_start` = 0 件（履歴注記を除く））、(d) コード変更ゼロ（`git diff --stat -- modules tools` が空）"
    category: validation
    applies_to: [CodingRule, SpecDocument, DesignCatalogue]
    trigger: "Bolt B4 の PR"
    logic: "PR の受入チェックとして grep と diff を実行"
    violation: "PR を戻す"
    source: "unit-of-work U9 合格, bolt-plan B4（デモ = diff レビュー）"
  - id: BR5.2
    statement: "作法: 仕様の改訂は『構造の規範と所有の記述』に限り、upstream 互換の逐語契約（D6）は変えない。改訂箇所には出典（ADR / 契約 / Bolt / オーナー裁定）を括弧書きで残す。日本語正本（制約 C4）、固定トークンは英語"
    category: policy
    applies_to: [SpecDocument]
    trigger: "B4 の文書改訂"
    logic: "各改訂行に出典の短い注記"
    violation: "出典の無い改訂はレビューで差し戻し"
    source: "制約 C4, 00-policy §2（仕様と実装の分離）"
```

## 2. 規則の要約

| ID | 区分 | 一言 | 出典 |
|---|---|---|---|
| BR1.1 | policy | use-case-rules の `load()` → `find_by_id()` | FR8.1 / C1 |
| BR1.2 | policy | gateway-taxonomy §4 の load / save → find / save | FR8.1 / C2 |
| BR1.3 | policy | §2b に ES 拡張語彙 `store` の注記 | FR8.1 / ADR-006 |
| BR1.4 | policy | §2 から旧称 AuditLedgerRepository を除去 | FR8.1 / ADR-006 |
| BR2.1 | policy | 11 号 §3 ポート表・供給面表を taxonomy 語彙へ | FR8.2 / R3 |
| BR2.2 | policy | 01 号 §3 集約候補表を裁定に合わせる | FR8.2 / C6 / C12 |
| BR2.3 | policy | 10 号 §3『同上』→ 1 trait 1 Impl、find_by_id | FR8.2 / C11 |
| BR2.4 | policy | PlanAction / CheckboxState の所有一意化 | FR8.2 / R1 |
| BR2.5 | policy | 12 号 §2.3 / §5 / 実装ノートの整合（C8 / C9 / C10） | FR8.2 |
| BR3.1 | policy | ADR-008（id / revision、find_by_id）を仕様へ | Q3 = A |
| BR3.2 | policy | ES 化後の workspace 集約表、IntentId = UUIDv7、IntentDirName | Q2 = A / Q3 = A / 追加 1 |
| BR3.3 | policy | B3 確定事項（gated = phase、Started 自己完結、effective_plan 所有、ES 形）を 10 / 12 号へ | Q3 = A |
| BR3.4 | policy | deviations.md に ES / SQLite の逸脱登録 | Q3 = A / NFR1 |
| BR3.5 | policy | components.md WorkspaceModel を値オブジェクト語彙へ縮退 | 追加 1 |
| BR3.6 | policy | 01 号 §7 にドメインモデルの原則を明記 | 追加 2 |
| BR4.1 | policy | error-handling.md の新設（Q1 = A の文面） | FR9.6 |
| BR4.2 | policy | README 索引の更新 | FR9.6 / FR8.1 |
| BR5.1 | validation | 合格条件（レビュー・README 無矛盾・自己整合 grep・コード変更ゼロ） | U9 合格 |
| BR5.2 | policy | 改訂の作法（逐語契約は不変、出典注記、日本語正本） | C4 / 00-policy |
