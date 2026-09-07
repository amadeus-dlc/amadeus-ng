# functional-design-questions — U9 正本・仕様の canon 追従（`u9-canon-docs`）

> Functional Design（Construction 3.1）の質問票（Unit: U9、kind: spec、Bolt: B4、規模 S）。出典:
> `../../../inception/units-generation/unit-of-work.md`（U9 の責務: FR8.1 / FR8.2 / FR9.6）、`../../../inception/units-generation/
> unit-of-work-story-map.md`、`../../../inception/requirements-analysis/requirements.md`（FR8.1 / FR8.2 / FR9.6、制約 C2 / C4）、
> `../../../inception/domain-design/components.md`、`../../../inception/domain-design/decisions.md`（ADR-005 / ADR-006 / ADR-008）、
> `../../../inception/contract-design/contract-summary.md`（C4 / C5 改訂）、`aidlc/spaces/default/knowledge/aidlc-shared/design-audit-2026-08-22.md`
> （A 束 C1 / C2、B 束 C3 / C4 / C6 / C8 / C9 / C10 / C11 / C12）、`../../../inception/practices-discovery/evidence.md`（FR9.6 の文面ドラフト）、
> `docs/specs/{01-domain-model,10-orchestration,11-workspace,12-workflow-definition}.md`、`coding-rules/*.md`、Bolt B3 の実装結果
> （`../../u2-domain-es-core/code-generation/code-summary.md`）。
>
> U9 は**文書だけ**の Unit（コード変更なし）。「エンティティ」= 改訂対象の正本文書、「規則」= 各改訂の内容と合格条件。
> 質問は、オーナー裁定を要する 3 点に絞る（Q1 = FR9.6 の規則文面、Q2 = `IntentId` の正本、Q3 = B 束の範囲追加）。

## 質問

### Q1. FR9.6 — エラーハンドリング様式規則の文面（coding-rules へ 1 ファイル追加、オーナー確認が要件）

practices-discovery のドラフト（`evidence.md` 確定アクション 5）を、その後の裁定（Bolt B1 ゲート: `std::error::Error` 手実装可 /
設計監査 R4: ドメイン層のエラーは**材料のみ**保持し逐語文言はアダプタ層の message-catalog）に合わせて改訂した案:

> **ルール**（`coding-rules/error-handling.md`）: ドメイン層・ユースケース層の失敗はモジュールごとの**手実装エラー enum** で表現する。
> `thiserror` / `anyhow` 等のエラーハンドリング外部クレートには依存しない。各エラー enum は `std::fmt::Display` と `std::error::Error` を
> 手実装する。`Display` は**材料**（ID・索引・状態・原因）だけを描く開発者向けの診断表示であり、利用者向けの逐語文言（upstream 互換面）は
> アダプタ層（message-catalog）が組み立てる — ドメイン層に文言を持ち込まない。変種フィールドは材料のみ（`stage`, `actual`, `expected`,
> `path`, `cause` など）で、`String` の文言を運ぶ変種を作らない。fallible な公開関数には `# Errors` セクションを付ける（`missing_errors_doc` deny）。
> `# Panics` を要する公開関数は作らない（範囲は型で保証 — `StageIndex` 等）。
>
> **根拠**: 依存最小化と、エラー型をドメイン語彙に閉じ込める方針（Always Valid、R4）。**機械強制**: `missing_errors_doc` / `missing_panics_doc`
> deny lint、`unwrap_used` / `expect_used` deny。`thiserror` / `anyhow` の禁止は `cargo lint` カスタムルール候補（赤例テスト必須の DoD）。

- A. 上の改訂ドラフトのまま採用する（推奨 — 現行実装 core-domain / use-case の全エラー型と一致）
- B. 文面を修正して採用する（修正点を指定）
- C. 今回は見送る（FR9.6 を後続 intent へ）
- X. Other (please specify)

[Answer]: A

### Q2. `IntentId` の正本 — 01 号（UUIDv7）と Bolt B3 実装（記録ディレクトリ名）の不一致

`docs/specs/01-domain-model.md` §3 workspace の Domain Primitive 候補は **`IntentId`（UUIDv7 — `intents.json` の `uuid`）** と定める一方、
Bolt B3 の `core_domain::orchestration::IntentId` は**記録ディレクトリ名**（`intents.json` の `dirName`、例 `260822-stage1-selfhost`、一般の
kebab）を受理している（U2 機能設計 entities.md の記述を実データに合わせた結果）。集約 `WorkflowExecution` の識別子（C6 `journal.aggregate_id` /
`snapshot.aggregate_id`）としてどちらを正本にするかの裁定が要る。

- A. **UUIDv7 を集約 ID にする**（推奨 — 01 号を維持。`IntentId` = `intents.json` の `uuid`（UUIDv7、文字列ソートで作成順）、記録ディレクトリ名は
  別の値 `IntentDirName`（投影のパス解決に使う読取モデルの関心）。U2 の `IntentId::parse` を UUIDv7 形式に改め `dirName` 用の型を分ける是正は
  Bolt B5（U3 — aggregate_id を SQLite に書く最初の Unit）で行い、U2 機能設計 entities の記述も同期する）
- B. 記録ディレクトリ名を集約 ID にする（01 号 §3 の `IntentId` 定義を改訂 — B4 の FR8.2 に含める）
- X. Other (please specify)

[Answer]: A

### Q3. B 束（FR8.2）の範囲 — 設計監査後に確定した裁定を canon 追従へ含めるか

FR8.2 の列挙（11 号 §2.3/§3、01 号 §3、10 号 §3、10/12 号 PlanAction・CheckboxState 所有、12 号 §2.3/§5/§39）に加えて、設計監査後に
ADR / 契約で確定した次の事項も同じ B4 で仕様へ追従させる案:

1. ADR-008: 12 号 §2.1 / 01 号 §3 の `WorkflowDefinition` に識別子 `WorkflowDefinitionId`（harness.json の `name`）と内容版 `DefinitionRevision` を追記、
   10 号 §3 ポート表の `WorkflowDefinitionRepository::find` → `find_by_id`（C4）。
2. ADR-001〜007（ES 化）の帰結: 01 号 §3 の集約候補表（`AuditLedger` はイベントログ、`StateFile` は媒体、`WorkspaceLock` 退役）、
   11 号 §3 のポート表（`AuditLedgerService` → 退役、Repository は `WorkflowExecutionRepository`、`FileStore` は Repository 実装内部、Clock /
   ProcessProbe はアダプタ層機構）— 監査 C3 / C4 / C6 / C11 の「現行の正」を ADR で確定した内容に合わせる。
3. Bolt B3 で確定した `gated = phase ≠ initialization`、`Started` の自己完結（StageEntry 列・definition_id）、`effective_plan` の集約所有（C8）を
   10 号 / 12 号の該当箇所へ反映。
4. `docs/specs/deviations.md` へ「SQLite ファイルの追加・ロック dir 非生成・互換ファイルはリードモデル」（ADR-003 / 007、NFR1）を登録。

- A. 1〜4 をすべて B4 に含める（推奨 — B5 以降の実装レビューの基準を一本化する。規模 S → M 相当に増えるが文書のみ）
- B. 1 と 3 だけ含める（2 と 4 は U3 の Bolt B5 で実装と同時に）
- C. FR8.2 の元の列挙だけにする（追加分は後続 intent）
- X. Other (please specify)

[Answer]: A

## 前提（確認事項）

- P1. A 束（FR8.1）の 4 点はそのまま実施: `use-case-rules.md:38` の `repository.load()` → `find_by_id()`（Repository 語彙）、`gateway-taxonomy.md` §4
  の「load / save」→「find / save」、§2b に ES Repository の拡張語彙 `store`（ADR-006、event-store-adapter-rs 同形）の注記、§2 実例リストから
  旧称 `AuditLedgerRepository` を除去。`coding-rules/README.md` の一覧に `error-handling.md`（Q1 = A/B の場合）を追加。
- P2. コード変更なし（仕様・正本の文書のみ）。合格はレビュー確認 + `coding-rules/README.md` との無矛盾 + 各仕様の自己整合。
- P3. 成果物は entities.md（改訂対象文書の一覧 = エンティティ）/ rules.md（改訂内容と合格条件 = BR）/
  functional-spec.md（文書改訂の手順・失敗時の扱い・完了条件）/ traceability.json（FR8.1 / FR8.2 / FR9.6 → BR）。
  現行のステージ契約では spec kind にも functional-spec.md が必須のため、旧前提「作らない」を置き換える。

## 再走時の追問（2026-09-07 — functional-design ステージ差し戻し後の U9 再走）

> 前回（B4、2026-08-23）の文書改訂は PR #28 でマージ済み。2026-09-05 の補完（PR #112）で functional-spec を追加し、レビュー READY（R-01 Major / R-02・R-03 Minor）。
> 本再走は、その所見と保留改訂（`pending-revision.md` 5 項目）、および U2 再走（b51、PR #118）で確定した現行の正に、U9 の設計記録と改訂指示を同期させる。

### Q4. U9 再走の Bolt の範囲（実測: `gap-measurement-20260907.md`）

オーナー指示（2026-09-07）「あいまいな提案するな。コードを実測して提案しろ。現状のコードを基準だろ」に従い、`main` `02cacea2` のコードを基準に
正本・仕様・共有契約のずれを行単位で実測した（`gap-measurement-20260907.md` §1 コード現状、§2 ずれ一覧、§3 パッケージ）。要点:

- コード: 集約 4（`Intent` 7 属性 / `IntentExecution` 12 属性・コマンド 15・イベント 16 / `WorkflowDefinition` / `CompiledDefinition`）、FCC 16 型、
  コマンド側ポート 4・ユースケース 9、クエリ側ユースケース 13・DAO 14、RMU `read_*` 17 表、再構成 = 最新スナップショット + 差分再生、楽観 version は集約の内側。
- 仕様 4 号: `WorkflowExecution` 37 件のうち要改訂 ≈ 46 行（10 号 20 / 12 号 9 / 11 号 8 / 01 号 9）+ 冒頭注記 2 種 × 4 号。10 号 §2.1 は属性数・コマンド数・
  イベント数・version の置き場・`next_decision` の署名・memento の 6 点でコードと食い違う。11 号 / 01 号の workspace 集約 3 つは未実装（「予定」の明記が要る）。
- coding-rules: 6 ファイル 10 箇所（クレート旧名・message-catalog・存在しないエラー型・`InMemoryXxxRepository`）。
- 共有契約: `components.md` は全面改訂級（クエリ側・RMU 表・`CompiledDefinition` / `Intent` が無い、全再生注記）、`contract-summary.md` は C1 / C3 / C4 / C5 / C6 / §4。

既存記録の扱いは **Modify**（既存 4 点を土台に更新、履歴は残す）とする — Keep は R-01 の古い YAML を残し、Redo は B4 の回答・レビュー履歴を失うため、
「現状のコードを基準」と両立しない。

- A. P1 + P2 + P3 + P4 を U9 再走の 1 Bolt で行う（文書のみ・コード変更なし。委譲 2 派遣: 派遣 1 = coding-rules + 仕様 4 号、派遣 2 = components + contract-summary。
  受入 = §2 の各行がコードと一致・sentinel 0 件・`git diff -- modules tools scripts .github Cargo.*` 空）（推奨）
- B. P1 + P2 + P3 を U9 Bolt、P4（Inception 共有契約 2 本）は次の Bolt に分ける（PR を小さく保つ）
- C. P1 + P2 のみ（仕様・共有契約は後続 intent）
- X. Other (please specify)

[Answer]: A（オーナー回答逐語 2026-09-07: 「正しさを実装コードもテストも仕様も常に検証してね。鵜呑みにしないでほしい。A」— 検証規律として BR5.3 に規則化し、§13 学習候補にも載せる）

## Consolidated Summary Confirmation

- Q1 = A: FR9.6 のエラーハンドリング様式規則は改訂ドラフトのまま `coding-rules/error-handling.md` として追加（材料のみ・文言はアダプタ層・Display / Error 手実装・
  thiserror / anyhow 不使用・`# Errors` 必須・`# Panics` なし、機械強制の現状と候補を明記）。README の一覧に追加
- Q2 = A: `IntentId` = UUIDv7（`intents.json` の `uuid`、01 号維持）。記録ディレクトリ名は別の値（`IntentDirName`）。U2 の `IntentId::parse` の是正と
  `dirName` 用の型の分離は Bolt B5（U3）で行い、U2 機能設計 entities と 01 号の記述を同期（本 Bolt では 01 号に `IntentDirName` の行を追加）
- Q3 = A: B 束は元の列挙 + ADR-008（12 号 / 01 号の識別子・内容版、10 号 `find_by_id`）+ ES 化の帰結（01 号集約表・11 号ポート表）+ B3 確定事項
  （gated = phase ≠ initialization、Started の自己完結、effective_plan の集約所有）+ deviations.md 登録 — すべて B4 に含める（規模 S → M 相当、文書のみ）
- 前提 P1〜P3: A 束 4 点、コード変更なし、成果物は entities / rules / functional-spec / traceability
  （現行のステージ契約に合わせ、functional-spec は文書改訂の手順・失敗時の扱い・完了条件を定める）
- 追加 1（オーナー質問「WorkspaceModel は集約か」への回答から）: 01 号 §3.3 の workspace 集約候補を ES 化後の姿に改訂（`Intent` / `Space` / `Worktree` が集約、
  `StateFile`・`AuditShard` はリードモデル、`WorkspaceLock` は退役）。`components.md` の `WorkspaceModel` を「workspace 語彙（値オブジェクト）」に縮退させ、
  状態ファイル描画の関数群は ReadModelUpdater（U4）へ移す方針を明記（コード移動は U4 の Bolt）
- 追加 2（オーナー確認の原則）: 01 号の設計原則に「ドメインモデルは集約（エンティティ）と値オブジェクトが主役。純粋関数としてのドメインサービスは消極的に使う。
  ドメインモデル・ドメインサービスは永続化責務を持たない（永続化を呼ばない）。永続化の指揮はユースケース層（trait はユースケース層、実装はアダプタ層）」を明記

**再開時の補完範囲（2026-09-05）**

- Q1〜Q3 と追加 1・2 は、当時の回答として保持する。再開によって過去の文書改訂やコード実装をやり直すことはしない。
- 不足する `functional-spec.md` に、対象文書の確認、改訂根拠の照合、改訂案の作成、整合検査、レビュー、確定・差し戻しの手順を記載する。
- エンティティ関係図と規則の要約は、既存の `entities.md` と `rules.md` から導出し、それぞれの正本を二重化しない。
- 対象は正本の語彙整理（FR8.1）、仕様との整合（FR8.2）、エラーハンドリング規則（FR9.6）。コード・外部向けの互換契約は変更しない。
- 古い記述を現在の設計へ戻さない。後続の確定裁定（集約名の変更、イベント列からの再構成、CQRS の責務分離など）と照合し、根拠だけでは解消できない矛盾は改訂前に確認する。
- `pending-revision.md` に残る改訂候補と既存レビューの所見は、確認が必要な事項として仕様書に明示し、未対応を完了扱いにしない。
- 前回の要約確認は 2026-08-23T04:47:15Z の監査記録に保存されている。今回は、後から追加された P3 の `functional-spec.md` を含む補完範囲を確認する。

**再走時の確認範囲（2026-09-07 — 差し戻し後の U9 再走、Q4 = A）**

- Q1〜Q3・追加 1・2・2026-09-05 の補完範囲は当時の回答として保持し、2026-09-05 の要約確認（監査記録に保存）をやり直さない。今回は Q4 で確定した再走範囲だけを確認する。
- 基準は現行コード（`main` `02cacea2`）。提案・改訂はすべて `gap-measurement-20260907.md` の行単位の実測（§1〜§2）を根拠にし、記録・仕様・レビュー本文の主張は鵜呑みにせず、実装コード・テスト・仕様で検証してから採用する（BR5.3 として規則化）。
- Bolt / Unit 記録は construction 配下 29 ディレクトリ・224 ファイル + handoff 16 本 + ADR-001〜011 を 4 区画で全数探索した（`survey-20260907/` 4 台帳）。そこから拾った裁定は 1 件ずつコードで実否を確認し、統合版を同文書 §4（O1〜O15 / P1〜P9 / R1〜R7 / W1〜W5 / S1〜S4 / K1〜K6、未実装 §4.7、矛盾の裁き §4.8 13 件）に載せた。この §4 が仕様の「正しい姿」の正本になる。
- P1 設計記録の同期: `rules.md`（BR1.1〜BR3.6 に status / 履歴、BR1.5 = gateway-taxonomy §1b 一般形は実施済み、BR1.6 = coding-rules 10 箇所、BR2.5 範囲 5 箇所、BR3.3 / BR3.7 / BR4.1 / BR4.2 / BR5.1 改訂、BR5.3 新設）、`entities.md`（再走属性、contract-summary / factory-naming / module-visibility のインスタンス追加、旧 Review は履歴化）、`functional-spec.md`（§1 再走範囲、§2 入力、§5 / §6 を 2026-09-07 の実測へ、旧 Review を履歴節へ）、`traceability.json`（FR8 親行 = R-03、BR5.3 等の reverse）。pending-revision 5 項目と R-01〜R-03 を閉じる。
- P2 coding-rules: 現行 10 箇所（README:50、error-handling 行 4 / 37、gateway-taxonomy :223 / :299、factory-naming :47 / :84、module-visibility :11 / :12 / :21、use-case-rules :11 / :36）の旧名 `WorkflowExecution` 系を現行名へ。規則の意味は変えない。
- P3 仕様 4 号の全文追従: 10 号 §2.1（Intent + IntentExecution、12 + 7 属性、コマンド 15、イベント 16、version 集約内、差分再生、`next_decision` の `Result` / `IntentMismatch`、FCC）と §3 ポート表、12 号、11 号（RMU 二層・17 表・同期 catch_up）、01 号（§3 集約表、§7.1 原則追記: FCC / ドメイン永続化中立 / CQRS クレート境界 / DAO 1 表 1 引当 / ドメインイベント = エンティティ / スナップショット + 差分再生と壊れた歴史のクラッシュ）。B12 読み替え注記・B13 優先順位注記は本文へ畳み込む。未実装（§4.7）は「予定」と明記。10 号 :51 の旧 manifest 綴りを直す。仕様から一度も参照されていない coding-rules 12 本へ相互参照を付ける。
- P4 共有契約 2 本の現行化: `components.md` 全面（クレート 10 / RMU 中間クレート / クエリ側 13 + 14）、`contract-summary.md` C1 / C3 / C4 / C5 / C6 / §4（旧 manifest 綴り 2 行、未決 1 件は §4 に残す）。加えて `decisions.md:473` の旧 manifest 綴り、`unit-of-work.md` U3 の失効記述への注記。
- 扱わない（§5）: U1 / U2 / U3 / U10 の設計本文、Bolt 記録（read-model-spec / inventory）、`formal/orchestration/journal_protocol.qnt` のコメント 5 行（コード扱い — 別 Bolt へ）、codekb・docs/CLAUDE.md・CI 設定の裁定。
- 文書のみ・コード変更なし。実装は委譲 2 派遣（P2 + P3 の 10 号・01 号 / P3 の 11 号・12 号 + P4）、書込スコープは非重複、diff 全件レビューと最終検証はメインが行う。完了条件 = BR5.1（grep 範囲 `coding-rules/*.md` + `docs/specs/*.md`、sentinel 0 件、`git diff --stat -- modules tools scripts .github Cargo.toml Cargo.lock` が空、§2 / §4 の各行が文書と一致）。
- 成果物は Modify（`reuse-artifact functional-design --decision modify` 記録済み）。次ステージは U9 の NFR Requirements。

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct
