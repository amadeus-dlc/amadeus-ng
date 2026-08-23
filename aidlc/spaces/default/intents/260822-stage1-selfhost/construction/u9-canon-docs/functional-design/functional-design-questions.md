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

[Answer]: 

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

[Answer]: 

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

[Answer]: 

## 前提（確認事項）

- P1. A 束（FR8.1）の 4 点はそのまま実施: `use-case-rules.md:38` の `repository.load()` → `find_by_id()`（Repository 語彙）、`gateway-taxonomy.md` §4
  の「load / save」→「find / save」、§2b に ES Repository の拡張語彙 `store`（ADR-006、event-store-adapter-rs 同形）の注記、§2 実例リストから
  旧称 `AuditLedgerRepository` を除去。`coding-rules/README.md` の一覧に `error-handling.md`（Q1 = A/B の場合）を追加。
- P2. コード変更なし（仕様・正本の文書のみ）。合格はレビュー確認 + `coding-rules/README.md` との無矛盾 + 各仕様の自己整合。
- P3. 成果物は entities.md（改訂対象文書の一覧 = エンティティ）/ rules.md（改訂内容と合格条件 = BR）/ traceability.json（FR8.1 / FR8.2 / FR9.6 →
  BR）。functional-spec.md は spec kind のため作らない（produces_kinds）。
