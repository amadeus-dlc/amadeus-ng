# rules — U9 正本・仕様の canon 追従（`u9-canon-docs`）

> Functional Design（Construction 3.1）成果物（Unit: U9、kind: spec）。**改訂履歴**: 初版 2026-08-23（Bolt B4 向け、iteration 2 READY）→ 補完 2026-09-05
> （functional-spec 追加、READY R-01〜R-03）→ **再走 2026-09-07（本版、Modify）**: ステージ差し戻し後の U9 再走で、正本 YAML を現行コード基準へ同期した。
>
> 出典: `../../../inception/units-generation/unit-of-work.md`（U9 の責務・合格）、`../../../inception/units-generation/unit-of-work-story-map.md`
> （FR8 → FR8.1 → FR8.2 → FR9.6）、`../../../inception/requirements-analysis/requirements.md`（FR8 / FR8.1 / FR8.2 / FR9.6、制約 C2 / C4）、
> `../../../inception/domain-design/decisions.md`（ADR-001〜011 とステータス注記）、`../../../inception/contract-design/contract-summary.md`、
> `aidlc/spaces/default/knowledge/aidlc-shared/design-audit-2026-08-22.md`、**`gap-measurement-20260907.md`**（現行コード `main` `02cacea2` を基準にした
> 行単位の実測 §1〜§2、作業パッケージ §3、記録全数探索の統合台帳 §4、未了箇所の仕分け §5）、**`survey-20260907/`**（Bolt / Unit 記録 4 区画の一次台帳）、
> 確認質問 `functional-design-questions.md`（Q1〜Q3 = A、追加 1・2、**Q4 = A**、要約確認 2026-09-07 Looks correct）。
>
> 下の fenced `yaml` が正本。各規則の `status` は **done**（適用済み — 実測で確認）/ **open**（本再走の Bolt で実施）/ **superseded**（後続裁定で失効。
> 文面は履歴として残し、`superseded_by` の規則が現行）。BR1.x = coding-rules（FR8.1）、BR2.x / BR3.x = 仕様と共有契約（FR8.2）、BR4.x = FR9.6、
> BR5.x = 合格条件と作法。**仕様に書く「正しい姿」の正本は `gap-measurement-20260907.md` §4** であり、本書はそれを規則に写す。

## 1. 規則（正本）

```yaml
rules:
  # --- BR1: coding-rules の語彙整合（FR8.1） ---
  - id: BR1.1
    status: done
    statement: "`coding-rules/use-case-rules.md` §4 の例示 `repository.load()` を Repository 語彙 `repository.find_by_id()` に直す"
    category: policy
    applies_to: [CodingRule(use-case-rules.md)]
    trigger: "B4 の文書改訂（適用済み）"
    logic: "IF 正本の例示が gateway-taxonomy §2b の許容動詞に無い語を使う THEN 許容動詞へ置換"
    violation: "grep `repository.load()` = 0 件（2026-09-07 実測 0）"
    source: "FR8.1, 設計監査 C1"
  - id: BR1.2
    status: done
    statement: "`coding-rules/gateway-taxonomy.md` §4 / §5 の散文「load / save」を「find / save」に直す"
    category: policy
    applies_to: [CodingRule(gateway-taxonomy.md)]
    trigger: "B4 の文書改訂（適用済み）"
    logic: "§2b の語彙に揃える"
    violation: "grep `load / save` = 0 件（2026-09-07 実測 0）"
    source: "FR8.1, 設計監査 C2"
  - id: BR1.3
    status: done
    statement: "`coding-rules/gateway-taxonomy.md` §2b に ES Repository の拡張語彙 `store(event, aggregate)` / `find_by_id` の注記を置く"
    category: policy
    applies_to: [CodingRule(gateway-taxonomy.md)]
    trigger: "B4 の文書改訂（適用済み）"
    logic: "ES Repository の動詞は本家ライブラリ（event-store-adapter-rs）の語彙に従う旨を 1 段落"
    violation: "注記が無ければレビューで差し戻し"
    source: "FR8.1, ADR-006 / ADR-010"
  - id: BR1.4
    status: done
    statement: "`coding-rules/gateway-taxonomy.md` §2 から旧称 `AuditLedgerRepository` を除去し、監査台帳はイベントログ（集約ではない）と注記する"
    category: policy
    applies_to: [CodingRule(gateway-taxonomy.md)]
    trigger: "B4 の文書改訂（適用済み）"
    logic: "箇条を削除し注記に置換"
    violation: "grep `AuditLedgerRepository` = 0 件（coding-rules + docs/specs、2026-09-07 実測 0）"
    source: "FR8.1, ADR-006"
  - id: BR1.5
    status: done
    statement: "`coding-rules/gateway-taxonomy.md` §1b「非 Repository ポートの模範例 — WorkspaceLock」を「非 Repository ポートの一般形」（契約の意味論を型に載せる）へ再構成し、退役した具体名への依存を外す"
    category: policy
    applies_to: [CodingRule(gateway-taxonomy.md)]
    trigger: "回復レビュー iteration 2 所見 #2（2026-08-23）→ 改訂は実施済み、設計記録への登録が本再走"
    logic: "§1b 見出し = 「非 Repository ポートの一般形」（2026-09-07 実測 :67）。`WorkspaceLock` は ADR-007 で退役"
    violation: "§1b が退役機構を現行の指針として提示していればレビューで差し戻し"
    source: "pending-revision 項目 2 / 5, ADR-007, レビュー所見 #2（2026-08-23）"
  - id: BR1.6
    status: open
    statement: "coding-rules の旧名・旧クレート名・解体済み機構への現行規範としての言及 12 行（6 ファイル）を現行へ直す: `README.md:50`（message-catalog → 出す側の `wording`）/ `README.md:115`（`WorkflowExecutionState` は消滅 — 是正済みへ）/ `error-handling.md:4`（適用例を `core-command-domain` の現行エラー型へ — `StartError` / `SnapshotError` は存在しない）/ `factory-naming.md:47`（反例は是正済みへ履歴化）/ `factory-naming.md:84`（`IntentExecution::with_version`）/ `gateway-taxonomy.md:223`（集約名）/ `gateway-taxonomy.md:299`（`core-command-domain`）/ `module-visibility.md:11,21`（`core_command_domain::`）/ `module-visibility.md:12`（message_catalog の例を現行の共有語彙へ差し替え）/ `use-case-rules.md:11`（`core-command-use-case` / `core-command-interface-adapter`）/ `use-case-rules.md:36`（コマンド側は `XxxRepositoryImpl<S>` の `in_memory()`、クエリ側は `InMemoryXxxDao`）"
    category: policy
    applies_to: [CodingRule(README.md), CodingRule(error-handling.md), CodingRule(factory-naming.md), CodingRule(gateway-taxonomy.md), CodingRule(module-visibility.md), CodingRule(use-case-rules.md)]
    trigger: "U9 再走の Bolt（P2）"
    logic: "各行を gap-measurement §2.5 の『コード』列の値へ置換する。規則の意味は変えない。是正の経緯・退役の履歴として書かれた言及（command-query-separation:5 / interior-mutability:5 / domain-equality:4 / tell-dont-ask:46 / aggregate-references:62 / cqrs-boundaries:30 / field-visibility:46 / good-examples:84 / ubiquitous-language:21）は触らない"
    violation: "現行規範として旧名が残る行があればレビューで差し戻し（BR5.1 の履歴除外 grep で検出）"
    source: "FR8.1, Q4 = A, gap-measurement §2.5（2026-09-07 実測）"

  # --- BR2: 仕様の canon 追従 — FR8.2 の元の列挙（B4 で適用済み） ---
  - id: BR2.1
    status: done
    statement: "11 号 §3 のポート表・供給面表を gateway-taxonomy 語彙へ: ポートは Repository と外部システムクライアントだけ、`FileStore` は Repository 実装の内部部品、`Clock` は機構、`AuditLedgerService` は退役、造語ポートは禁止"
    category: policy
    applies_to: [SpecDocument(11-workspace.md)]
    trigger: "B4 の文書改訂（適用済み — 11 号 :106 / :125 / :126 で確認）"
    logic: "ポート表 4 列、機構は §4 アダプタ層へ"
    violation: "Store / Reader / Writer / Source / Provider の造語、機構のポート化が残ればレビューで差し戻し"
    source: "FR8.2, 設計監査 R3 / C3 / C4 / C11, ADR-003 / 004 / 007"
  - id: BR2.2
    status: superseded
    superseded_by: [BR3.3]
    statement: "01 号 §3 の集約候補表を当時の裁定（`WorkflowDefinition` に id / revision、`WorkflowExecution` = ES 形 FSM）に合わせる"
    category: policy
    applies_to: [SpecDocument(01-domain-model.md)]
    trigger: "B4 の文書改訂（適用済み）"
    logic: "B12 の集約分割（`Intent` + `IntentExecution`）で当時の記述は失効。現行の姿は BR3.3 が定める"
    violation: "—（履歴）"
    source: "FR8.2, 設計監査 C6 / C12"
  - id: BR2.3
    status: superseded
    superseded_by: [BR3.3]
    statement: "10 号 §3 ポート表の『同上』を廃し 1 trait 1 Impl（`XxxRepositoryImpl` + `InMemoryXxxRepository`）を明記、退役 2 行（AuditLedgerRepository / WorkspaceLock）を削除"
    category: policy
    applies_to: [SpecDocument(10-orchestration.md)]
    trigger: "B4 の文書改訂（適用済み — 退役行 0 件で確認）"
    logic: "退役行の削除は現行。『テストダブル `InMemoryXxxRepository`』の指示は ADR-010（本家 memory バックエンド `in_memory()`）で失効し、BR3.3 が現行の表を定める"
    violation: "—（履歴）"
    source: "FR8.2, 設計監査 C11"
  - id: BR2.4
    status: done
    statement: "`PlanAction` の定義は workflow-definition だけ、`CheckboxState` の定義は workspace だけ。他の号は参照と明記"
    category: policy
    applies_to: [SpecDocument(10-orchestration.md), SpecDocument(12-workflow-definition.md), SpecDocument(01-domain-model.md)]
    trigger: "B4 の文書改訂（適用済み）"
    logic: "所有コンテキストの節にだけ定義を置く（コードも同じ: `PlanAction` は `workflow_definition`、orchestration に再輸出なし — 2026-09-07 実測）"
    violation: "2 か所以上で定義されていればレビューで差し戻し"
    source: "FR8.2, 設計監査 C12 / R1, ADR-005"
  - id: BR2.5
    status: done
    statement: "12 号から削除済み API `next_in_scope_stage` の全出現（§2.3 / §4 / §8 / §9 の 5 箇所）と `StageGraphQuery` 等の個別名を除き、集約の述語面 6 + `grid().action()` の記述にする。§5 は `find_by_id`"
    category: policy
    applies_to: [SpecDocument(12-workflow-definition.md)]
    trigger: "B4 の文書改訂（適用済み — `next_in_scope_stage` 2026-09-07 実測 0 件）"
    logic: "適用範囲は pending-revision 項目 1 のとおり 12 号の全出現（§2.3 に限らない）"
    violation: "削除済みメソッド名が規範として残ればレビューで差し戻し"
    source: "FR8.2, 設計監査 C8 / C9 / C10, pending-revision 項目 1"

  # --- BR3: 仕様と共有契約の現行化（FR8.2） ---
  - id: BR3.1
    status: done
    statement: "ADR-008 を仕様へ: `WorkflowDefinition` は `WorkflowDefinitionId`（不変）と `DefinitionRevision`（内容版）を持つエンティティ、ポートは `find_by_id`"
    category: policy
    applies_to: [SpecDocument(12-workflow-definition.md), SpecDocument(10-orchestration.md), SpecDocument(01-domain-model.md)]
    trigger: "B4 の文書改訂（適用済み — 12 号 4 件で確認）"
    logic: "`DefinitionRevision::of_content` はドメインが導出（b36、コード一致）"
    violation: "エンティティに ID が無い記述が残ればレビューで差し戻し"
    source: "ADR-008, C4 改訂"
  - id: BR3.2
    status: done
    statement: "ES 化の帰結を 01 号 §3.3 と 11 号 §2 へ: `StateFile` / `AuditShard` はリードモデル、`WorkspaceLock` は退役、`IntentId` = UUIDv7、`IntentDirName` は別の値。workspace の集約候補 `Intent` / `Space` / `Worktree` は**未実装の予定**として書く（本再走で BR3.3 が『予定』の明記を追加）"
    category: policy
    applies_to: [SpecDocument(01-domain-model.md), SpecDocument(11-workspace.md)]
    trigger: "B4 の文書改訂（適用済み） + 本再走で予定の明記"
    logic: "コードの workspace モジュールに集約は無い（値オブジェクト + FCC 6 型のみ）。orchestration の `Intent` 集約は同名の別物であることを注記する"
    violation: "リードモデルを集約に、退役済みロックを現行規範に書いた記述が残ればレビューで差し戻し"
    source: "ADR-001 / 003 / 004 / 007, Q2 = A, 追加 1, gap-measurement §4.5 S4"
  - id: BR3.3
    status: open
    statement: "仕様 4 号（10 / 12 / 11 / 01）を現行コード（`main` `02cacea2`）へ全文追従させる。正本は `gap-measurement-20260907.md` §4 の裁定台帳（O1〜O15 / P1〜P9 / R1〜R7 / W1〜W5 / S1〜S4 / K1〜K6）で、改訂箇所は同 §2.1〜§2.4 の行単位表。要点: (a) 10 号 §2.1 を `Intent`（7 属性、`Created`）+ `IntentExecution`（12 属性、コマンド 15 + `start`、イベント 16 変種、`version` は集約内、memento 型なし、再構成 = 最新スナップショット + 差分 `replay(snapshot, events)`、壊れた歴史は panic）の 2 集約に分けて書く、(b) クエリ `next_decision(&Intent, &NextRequest) -> Result<NextDecision, CommandError>`（`IntentMismatch`）ほか §4 O10〜O12 の署名・ガード順、(c) FCC 16 型と `StageSlugSet` 辞書順 + RMU 文書順、(d) §2.2 の `Directive` / `DirectiveKind` / `ContinueToken` / `StateBinding` は所在 = クエリ側 `core-query-use-case` と明記（逐語契約は不変）、(e) §3 ポート表をコマンド側 4 ポート（`IntentExecutionRepository` / `IntentRepository` / `WorkflowDefinitionRepository` / `CompiledDefinitionRepository`、`expected_version` 引数なし、`RepositoryError` 4 変種、テストダブル型なし `in_memory()`）とクエリ側 DAO 14 に書き直し、ユースケース列はコマンド側 9 + クエリ側 `Find*` 13、(f) 12 号 §2.3 に `scope_cost` / `review_policy` / `stage_route` を追記、§5 を `DefineWorkflowUseCase` + `Find*` 5 + `read_definition*` 6 表へ、`find_for_intent` を追記、(g) 11 号 §2.1 / §3 供給面の未実装（集約 3・供給面 4・`intents.json` 直列化）を『予定』と明記、§3 ポート行を `IntentExecutionRepository` へ、RMU 二層・`read_*` 17 表・同期 `catch_up` を §2.3 / §4 に反映、(h) 01 号 §3.2 段落の書き直し、§3.3 は予定、脚注 :121 は是正済みへ履歴化、§7.1 に原則を追記（FCC / ドメイン永続化中立 / CQRS クレート境界の物理強制 / DAO 1 表 1 引当 / ドメインイベント = エンティティ / スナップショット + 差分再生と壊れた歴史のクラッシュ）、原則 5 の例を 2 段（`Intent` → `WorkflowDefinition`、`IntentExecution` → `Intent`）に、(i) 4 号冒頭の B12 読み替え注記・B13 優先順位注記を本文へ畳み込み『追従済み（2026-09-07 / U9 再走）』の 1 行へ縮める、(j) 一度も仕様から参照されていない coding-rules 12 本（abstract-data-type / command-query-separation / domain-object-kinds / domain-persistence-neutrality / domain-services / field-visibility / first-class-collections / infrastructure-layer / interior-mutability / module-visibility / ubiquitous-language / upstream-contracts）へ、該当節から相互参照を付ける、(k) 10 号 :51 のイベント数 11 → 16（`SingleStageRunCommitted` / `SkeletonStanceRecorded` / `ReviewRequested` / `ReviewCompleted` / `PracticesAffirmed` を追加）。旧 manifest 綴りは 4 号すべてで既に打消し線つき履歴（:51）であり改訂対象ではない"
    category: policy
    applies_to: [SpecDocument(10-orchestration.md), SpecDocument(12-workflow-definition.md), SpecDocument(11-workspace.md), SpecDocument(01-domain-model.md)]
    trigger: "U9 再走の Bolt（P3）"
    logic: "各行を §2 表の『処置』列どおりに改訂し、『維持』行は触らない。記録側の主張とコードが食い違う 13 論点は §4.8 の裁き（コード採用）に従う。逐語の upstream 契約（D6）は変えない。未実装は『予定（未実装、クリティカルパス n）』の形で書き、実装済みのように書かない"
    violation: "改訂後に §2 表の行と文書が一致しない、§4 台帳に無い主張を仕様に足す、旧名 `WorkflowExecution` が履歴マーカー無しで残る、のいずれかでレビュー差し戻し"
    source: "FR8.2, Q4 = A（オーナー 2026-09-07「正しさを実装コードもテストも仕様も常に検証」）, gap-measurement §2.1〜§2.4 / §4 / §4.8, ADR-002 改訂 2026-09-02, ADR-009 改訂 2026-08-28〜29, ADR-010 / ADR-011"
  - id: BR3.4
    status: done
    statement: "`docs/specs/deviations.md` に ES / SQLite の逸脱登録 1 行（テキストファイル群 + mkdir ロック → SQLite ジャーナル + 楽観 version、互換ファイルはリードモデル）"
    category: policy
    applies_to: [SpecDocument(deviations.md)]
    trigger: "B4 の文書改訂（適用済み — 2026-09-07 実測 1 行）"
    logic: "表の列形式を守って追記"
    violation: "未登録なら差し戻し"
    source: "NFR1, ADR-003 / 007"
  - id: BR3.5
    status: superseded
    superseded_by: [BR3.7]
    statement: "`components.md` の `WorkspaceModel` を『workspace 語彙（値オブジェクト）』に縮退し、描画関数は ReadModelUpdater へ"
    category: policy
    applies_to: [DesignCatalogue(components.md)]
    trigger: "B4 の文書改訂（適用済み）"
    logic: "縮退自体は現行と一致（描画は RMU、workspace は値オブジェクト + FCC）。ただし components.md 全体が旧世代（下記 BR3.7）のため、本規則の記述は BR3.7 の全面改訂に吸収する"
    violation: "—（履歴）"
    source: "追加 1（2026-08-23）"
  - id: BR3.6
    status: done
    statement: "01 号 §7.1『ドメインモデルの原則』を明記（集約 + 値オブジェクトが主役、ドメインサービスは消極的、永続化責務なし、永続化の指揮はユースケース層、集約間は ID 参照、集約 = FSM）"
    category: policy
    applies_to: [SpecDocument(01-domain-model.md)]
    trigger: "B4 の文書改訂（適用済み — 01 号 :269 で確認）"
    logic: "本再走では BR3.3 (h) が原則 6 件を追記する（新設ではなく追記）"
    violation: "原則に反する記述が 01 号の他節に残ればレビューで差し戻し"
    source: "追加 2（2026-08-23）, project.md Corrections"
  - id: BR3.7
    status: open
    statement: "共有契約（Inception 成果物）を現行へ: (a) `inception/domain-design/components.md` を全面改訂 — 冒頭注記『ジャーナル全再生』を差分再生へ（R-02）、コンポーネントをクレート 10 の実態（コマンド側ドメイン / ユースケース 9 / アダプタ、クエリ側 `Find*` 13 + DAO 14 + SQLite / InMemory DAO、RMU 中間クレート `JournalReader` + 投影 + `read_*` 17 表、`core-infrastructure`、合成ルート `aidlc` の CLI 面と動詞、`harness-*`）へ、集約 4（`Intent` / `IntentExecution` / `WorkflowDefinition` / `CompiledDefinition`）とイベント 16 / 1 / 2 / 4 を記載、`EngineUseCases` の `NextUseCase` / `ReportUseCase` / `ContinueUseCase` / `DoctorUseCase` / フック 4 を現行名と『予定』に振り分け、`PublishedLanguage` の message-catalog を出す側の `wording` へ、`RehydratedWorkflowExecution` を除去、(b) `inception/contract-design/contract-summary.md` を節単位で現行化 — C1 / §4 の message-catalog 3 件、C3 の trait 全文を現行 4 ポートへ（`Rehydrated…` 3 件除去、2026-08-30 追記『全再生』を差分再生へ）、C4 の `WorkflowExecution::definition_id()` を `Intent` へ、C5 のイベント列挙を 16 変種へ、C6 に『16 属性』注記の訂正と `read_*` 17 表 / `amadeus_read_model_head` / `amadeus_publication*` の追記。§4 の未決『同一シャード内の直接行と投影行の順序』は未決のまま残す。打消し線つきの旧 manifest 綴り（:285 / :305 / :388）は履歴として触らない、(c) `inception/units-generation/unit-of-work.md` U3 の失効記述（独自 EventStore スキーマ、`InMemoryWorkflowExecutionRepository` 先行 :83 / :144、`WorkflowExecutionRepository` :34 / :91）に『ADR-010 / B12 で失効 — 現行は本家 event-store-adapter-rs の journal / snapshot と `IntentExecutionRepositoryImpl::in_memory()`』の注記を付ける（本文の書き換えはしない — U3 は完了 Unit）、(d) `decisions.md` は ADR-009 改訂注記（2026-08-28 / 08-29）と ADR-010 の serde 失効注記（:441）が既にあるため変更しない"
    category: policy
    applies_to: [DesignCatalogue(components.md), DesignCatalogue(contract-summary.md), DesignCatalogue(unit-of-work.md)]
    trigger: "U9 再走の Bolt（P4）"
    logic: "components.md は YAML ブロックを gap-measurement §1 / §4 の実測どおりに書き直す。contract-summary は各 C 節の trait / 列挙を現行コードの署名へ揃え、失効した追記は打消し線 + 日付で履歴化する（削除しない）。unit-of-work は注記のみ"
    violation: "components.md にクエリ側・RMU・`CompiledDefinition` / `Intent` 集約・`read_*` 表が無い、contract-summary の C3 / C5 が旧署名のまま、のいずれかでレビュー差し戻し"
    source: "FR8.2, R-02（2026-09-05）, gap-measurement §2.6 / §4, ADR-009 / 010 / 011"

  # --- BR4: FR9.6 エラーハンドリング様式規則 ---
  - id: BR4.1
    status: done
    statement: "`coding-rules/error-handling.md` を正本とする: ドメイン層・ユースケース層の失敗はモジュールごとの手実装エラー enum、`thiserror` / `anyhow` 不使用、`Display` / `Error` 手実装、`Display` は材料だけ（利用者向け文言は出す側の `wording`）、変種フィールドは材料のみ、fallible な公開関数に `# Errors`。**例外**: 再構成（`replay` / `apply_event` / 誕生変換）は壊れた歴史に対して panic し `# Panics` を書く（現行 :37、コード 3 + 1 か所と一致）。B4 当時の文面（message-catalog、`# Panics` の全面禁止）は履歴で、現行ファイルの文面が正"
    category: policy
    applies_to: [CodingRule(error-handling.md)]
    trigger: "B4 で新設（適用済み） → 本再走で正本の所在を現行ファイルに固定"
    logic: "規則の正本は `coding-rules/error-handling.md` の現行本文。本 YAML は所在と要点を写すだけで文面を二重化しない"
    violation: "本 YAML と現行ファイルが食い違えば現行ファイルを正としレビューで指摘"
    source: "FR9.6, Q1 = A, R-01（2026-09-05）, gap-measurement §1『# Panics』"
  - id: BR4.2
    status: open
    statement: "`coding-rules/README.md` の一覧を各ファイルと同期する（error-handling 行 :50 の message-catalog、:115 の `WorkflowExecutionState` 是正対象表記 — BR1.6 と同時）。一覧の行数 = 規則ファイル数（README / good-examples / CONSISTENCY-AUDIT を除く）"
    category: policy
    applies_to: [CodingRule(README.md)]
    trigger: "U9 再走の Bolt（P2）"
    logic: "表の行と実ファイルの一言・機械強制を突き合わせる"
    violation: "README と実ファイルが不一致ならレビューで差し戻し（U9 の合格条件）"
    source: "FR8.1 / FR9.6 合格条件, unit-of-work U9"

  # --- BR5: 合格条件と作法 ---
  - id: BR5.1
    status: open
    statement: "合格 = (a) 各改訂がレビューで確認できる、(b) `coding-rules/README.md` の一覧と各ファイルが矛盾しない、(c) 自己整合 grep — 範囲は `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/*.md` + `docs/specs/*.md`（`docs/specs/research/` を除く）、sentinel は `effective_plan_action` / `next_in_scope_stage` / `AuditLedgerRepository` / `AuditLedgerService` / `StateFileStore` / `report_forward` / `gate_start` / **`WorkflowExecution`** / **`RehydratedWorkflowExecution`** / **`message-catalog`**、判定は『履歴マーカー（同一行に `~~` 打消し線、または 旧 / 失効 / 是正済み / 改名 / 履歴 の語）の無い行』が 0 件。`StageGraphReader` は gateway-taxonomy の禁止名テーブル（意図的な記録）のみに現存するため sentinel から外す、(d) コード変更ゼロ — `git diff --stat origin/main..HEAD -- modules tools scripts .github Cargo.toml Cargo.lock` が空、(e) `gap-measurement-20260907.md` §2.1〜§2.6 の各行の『処置』が文書に反映され、『維持』行が変わっていない"
    category: validation
    applies_to: [CodingRule, SpecDocument, DesignCatalogue]
    trigger: "U9 再走の Bolt の PR"
    logic: "PR の受入チェックとして grep と diff を実行し、結果を PR 本文に貼る"
    violation: "PR を戻す"
    source: "unit-of-work U9 合格, pending-revision 項目 3 / 4, Q4 = A"
  - id: BR5.2
    status: done
    statement: "作法: 仕様の改訂は『構造の規範と所有の記述』に限り、upstream 互換の逐語契約（D6）は変えない。改訂箇所には出典（ADR / 契約 / Bolt / オーナー裁定 / 実測行）を括弧書きで残す。失効した記述は削除せず打消し線 + 日付で履歴化する。日本語正本（制約 C4）、固定トークンは英語"
    category: policy
    applies_to: [SpecDocument, CodingRule, DesignCatalogue]
    trigger: "すべての文書改訂"
    logic: "各改訂行に出典の短い注記"
    violation: "出典の無い改訂はレビューで差し戻し"
    source: "制約 C4, 00-policy §2"
  - id: BR5.3
    status: open
    statement: "検証規律: 仕様・規則・共有契約に書く主張は、記録（Bolt / Unit の設計記録・レビュー本文・完了報告）を鵜呑みにせず、**実装コード・テスト・仕様の三つで検証してから**採用する。提案・改訂案は現行コードの行単位の実測を根拠に付ける。記録同士が矛盾するときはコードの現状で裁き、裁いた結果と両記録の出典を台帳（gap-measurement §4.8 の形式）に残す。仕様の『正しい姿』を定める前に Bolt / Unit 記録を全数探索し、裁定の取り落としが無いことを確認する"
    category: policy
    applies_to: [SpecDocument, CodingRule, DesignCatalogue]
    trigger: "U9 再走以降のすべての文書改訂・レビュー"
    logic: "改訂案 1 件につき『コードの所在（path:line または型 / 関数名）・テストの有無・仕様の該当節』を添える。添えられない改訂案は保留にする"
    violation: "根拠の無い改訂・記録の転記だけの改訂はレビューで差し戻し。§13 学習候補として project.md にも提案する"
    source: "Q4 = A（オーナー 2026-09-07「正しさを実装コードもテストも仕様も常に検証してね。鵜呑みにしないでほしい」「bolt か unit を全部探索して正しい姿に仕様としてる？？」）"
```

## 2. 規則の要約

| ID | status | 区分 | 一言 | 出典 |
|---|---|---|---|---|
| BR1.1 | done | policy | use-case-rules の `load()` → `find_by_id()` | FR8.1 / C1 |
| BR1.2 | done | policy | gateway-taxonomy §4 / §5 の load / save → find / save | FR8.1 / C2 |
| BR1.3 | done | policy | §2b に ES 拡張語彙 `store` の注記 | FR8.1 / ADR-006 |
| BR1.4 | done | policy | §2 から旧称 AuditLedgerRepository を除去 | FR8.1 / ADR-006 |
| BR1.5 | done | policy | §1b を非 Repository ポートの一般形へ（記録への登録が本再走） | pending-revision 2 / 5 |
| BR1.6 | open | policy | coding-rules の旧名・旧クレート名 12 行（6 ファイル）を現行へ | FR8.1 / Q4 = A |
| BR2.1 | done | policy | 11 号 §3 ポート表・供給面表を taxonomy 語彙へ | FR8.2 / R3 |
| BR2.2 | superseded | policy | 01 号 §3 集約候補表（B12 分割で失効 → BR3.3） | FR8.2 |
| BR2.3 | superseded | policy | 10 号 §3『同上』廃止・退役行削除（テストダブル指示は ADR-010 で失効 → BR3.3） | FR8.2 / C11 |
| BR2.4 | done | policy | PlanAction / CheckboxState の所有一意化 | FR8.2 / R1 |
| BR2.5 | done | policy | 12 号の `next_in_scope_stage` 全出現の除去（範囲は全出現） | FR8.2 / pending-revision 1 |
| BR3.1 | done | policy | ADR-008（id / revision、find_by_id）を仕様へ | ADR-008 |
| BR3.2 | done | policy | ES 化後の workspace（リードモデル・退役・予定の明記） | Q2 = A / 追加 1 |
| BR3.3 | open | policy | 仕様 4 号の全文追従（正本 = gap-measurement §4、行単位 = §2.1〜2.4） | FR8.2 / Q4 = A |
| BR3.4 | done | policy | deviations.md に ES / SQLite の逸脱登録 | NFR1 |
| BR3.5 | superseded | policy | components.md WorkspaceModel の縮退（→ BR3.7 全面改訂に吸収） | 追加 1 |
| BR3.6 | done | policy | 01 号 §7.1 ドメインモデルの原則（本再走で 6 件追記） | 追加 2 |
| BR3.7 | open | policy | 共有契約の現行化（components.md 全面、contract-summary 節単位、unit-of-work U3 注記） | FR8.2 / R-02 |
| BR4.1 | done | policy | error-handling.md の現行本文を正本（再構成の panic 例外を含む） | FR9.6 / R-01 |
| BR4.2 | open | policy | README 索引の同期 | FR8.1 / FR9.6 |
| BR5.1 | open | validation | 合格条件（レビュー・README 無矛盾・履歴除外 grep・コード diff 空・実測表一致） | U9 合格 |
| BR5.2 | done | policy | 改訂の作法（逐語契約は不変、出典注記、履歴化、日本語正本） | C4 / 00-policy |
| BR5.3 | open | policy | 検証規律（コード・テスト・仕様で検証、鵜呑みにしない、記録全数探索） | Q4 = A |

## 3. 前版からの変更（2026-09-07 再走）

- `status` / `superseded_by` 属性を全規則に追加した。done 14 / open 6 / superseded 3。
- 新設: BR1.5（実施済みの §1b 改訂を記録に登録 — pending-revision 項目 2 / 5）、BR1.6（coding-rules 12 行）、BR3.7（共有契約）、BR5.3（検証規律）。
- 改訂: BR2.5（適用範囲を全出現に — 項目 1）、BR3.2（未実装は予定と明記）、BR3.3（旧 `WorkflowExecution` 16 属性・12 イベント・memento の指示を廃し、
  gap-measurement §4 を正本とする全文追従に置換 — R-01）、BR4.1（現行ファイルが正、`# Panics` の再構成例外 — R-01）、BR5.1（grep 範囲・履歴除外の判定・
  sentinel の見直し・diff 範囲 — 項目 3 / 4、`StageGraphReader` は除外）。
- 失効: BR2.2 / BR2.3 / BR3.5（文面は履歴として残す）。
- `pending-revision.md` の 5 項目はすべて本版に反映した（項目 1 → BR2.5、2 / 5 → BR1.5、3 / 4 → BR5.1）。
- 要約確認（2026-09-07）で「10 号 :51 の旧 manifest 綴りを直す」と述べたが、実測では 4 号・decisions.md・contract-summary のすべてで既に打消し線つきの
  履歴であり改訂対象ではない（BR3.3 (k) / BR3.7 (b) に明記）。:51 の要改訂はイベント数 11 → 16 のみ。
