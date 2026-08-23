# entities — U9 正本・仕様の canon 追従（`u9-canon-docs`）

> Functional Design（Construction 3.1）成果物（Unit: U9、kind: spec、Bolt: B4）。出典: `../../../inception/units-generation/unit-of-work.md`（U9）、
> `../../../inception/units-generation/unit-of-work-story-map.md`（FR8.1 / FR8.2 / FR9.6）、`../../../inception/requirements-analysis/requirements.md`
> （FR8.1 / FR8.2 / FR9.6、制約 C2 / C4）、`../../../inception/domain-design/components.md`（WorkspaceModel の縮退対象）、
> `../../../inception/domain-design/decisions.md`（ADR-001〜008）、`../../../inception/contract-design/contract-summary.md`（C4 / C5 改訂）、
> `aidlc/spaces/default/knowledge/aidlc-shared/design-audit-2026-08-22.md`（A 束 / B 束）、確認質問 `functional-design-questions.md`（Q1〜Q3 = A、
> 追加 1・2、P1〜P3、Looks correct）。
>
> U9 は文書だけの Unit。ここでの「エンティティ」= **改訂対象の正本文書**（識別子はリポジトリ相対パス）。下の fenced `yaml` が正本。

## 1. エンティティ（正本）

```yaml
entities:
  - name: CodingRule
    description: "コーディング規則の正本ファイル（`aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/<rule>.md`、1 ルール 1 ファイル、README が索引）。オーナー裁定で確定した規則の記述 — 人間と全エージェントが読む"
    attributes:
      - { name: path, type: string, required: true, unique: true, constraints: "coding-rules/ 配下のリポジトリ相対パス" }
      - { name: rule_name, type: string, required: true, constraints: "一言（README の表の 2 列目）" }
      - { name: enforcement, type: enum, required: true, allowed_values: [type, existing-lint, cargo-lint, review], constraints: "機械強制の手段（README の表の 3 列目）" }
      - { name: decided_on, type: date, required: true, constraints: "裁定日（オーナー）" }
      - { name: revisions_in_b4, type: list<string>, required: true, constraints: "本 Unit で加える改訂（BR の ID）" }
    instances:
      - { path: "coding-rules/use-case-rules.md", revisions_in_b4: [BR1.1] }
      - { path: "coding-rules/gateway-taxonomy.md", revisions_in_b4: [BR1.2, BR1.3, BR1.4] }
      - { path: "coding-rules/error-handling.md", revisions_in_b4: [BR4.1], note: "新規（FR9.6、Q1 = A）" }
      - { path: "coding-rules/README.md", revisions_in_b4: [BR4.2], note: "索引の更新" }
    constraints:
      - "README の一覧と各ファイルの裁定日・強制手段が矛盾しない（U9 の合格条件）"

  - name: SpecDocument
    description: "仕様の正本（`docs/specs/NN-*.md`、00-policy が最上位）。upstream 互換の観測可能契約とクリーンアーキテクチャへの写像を規範として持つ。コード実装の判定基準"
    attributes:
      - { name: path, type: string, required: true, unique: true }
      - { name: sections_revised, type: list<string>, required: true, constraints: "本 Unit で改訂する節" }
      - { name: revisions_in_b4, type: list<string>, required: true }
    instances:
      - { path: "docs/specs/01-domain-model.md", sections_revised: ["§3.1 workflow-definition（集約に id / revision）", "§3.2 orchestration（WorkflowExecution の ES 形、PlanAction 所有）", "§3.3 workspace（集約候補の ES 化後の姿、IntentId = UUIDv7 と IntentDirName）", "§7 クリーンアーキテクチャへの写像原則（ドメインモデルの原則の明記）"], revisions_in_b4: [BR2.2, BR2.4, BR3.1, BR3.2, BR3.5, BR3.6] }
      - { path: "docs/specs/10-orchestration.md", sections_revised: ["§2.1 集約（ES 形・gated = phase・Started 自己完結・effective_plan 所有）", "§2.2 Domain Primitive（PlanAction は workflow_definition 所有を参照）", "§3 ユースケース層のポート表（同上 → 1 trait 1 Impl、find_by_id、WorkflowExecutionRepository）"], revisions_in_b4: [BR2.3, BR2.4, BR3.1, BR3.3] }
      - { path: "docs/specs/11-workspace.md", sections_revised: ["§2.1 集約（StateFile / AuditShard はリードモデル、WorkspaceLock 退役）", "§2.3 ドメインサービス（描画関数は投影へ）", "§3 ユースケース層（ポート表・供給面表を gateway-taxonomy 語彙へ、AuditLedgerService 退役）", "§4 インターフェイスアダプタ層（FileStore は Repository 実装内部、Clock / ProcessProbe は機構）"], revisions_in_b4: [BR2.1, BR3.2] }
      - { path: "docs/specs/12-workflow-definition.md", sections_revised: ["§2.1 集約（WorkflowDefinitionId / DefinitionRevision）", "§2.3 ドメインサービス（next_in_scope_stage 行の R2 整合、StageGraphQuery 等の個別名廃止）", "§5 ユースケース層（find_by_id）", "§1 / §10 集約昇格の第一理由（lockstep 一貫性単位）"], revisions_in_b4: [BR2.5, BR3.1, BR3.3] }
      - { path: "docs/specs/deviations.md", sections_revised: ["表に 1 行追加（SQLite ファイルの追加・ロック dir 非生成・互換ファイルはリードモデル）"], revisions_in_b4: [BR3.4] }
    constraints:
      - "各改訂は ADR / 契約 / 実装済みコード（Bolt B3）のいずれかを出典として示す（推測で仕様を変えない）"
      - "逐語の upstream 契約（D6）は変更しない — 変えるのは構造の規範と所有の記述だけ"

  - name: DesignCatalogue
    description: "Inception の設計成果物のうち、本 Unit が追従させる記述を持つもの（`components.md` の WorkspaceModel）"
    attributes:
      - { name: path, type: string, required: true, unique: true }
      - { name: revisions_in_b4, type: list<string>, required: true }
    instances:
      - { path: "aidlc/spaces/default/intents/260822-stage1-selfhost/inception/domain-design/components.md", revisions_in_b4: [BR3.5], note: "WorkspaceModel → workspace 語彙（値オブジェクト）へ縮退、描画関数は ReadModelUpdater へ（コード移動は U4）" }

relationships:
  - { from: CodingRule(README.md), to: CodingRule(*), cardinality: "one-to-many", description: "README が全規則ファイルを索引する（新規 error-handling.md を含む）" }
  - { from: SpecDocument, to: CodingRule, cardinality: "many-to-many", description: "仕様が規則ファイルを参照する（gateway-taxonomy / module-visibility / use-case-rules）— 語彙を揃える" }
  - { from: SpecDocument(01), to: SpecDocument(10/11/12), cardinality: "one-to-many", description: "01 号のコンテキストマップと集約表が各コンテキスト仕様の上位。集約・所有の記述は 01 号と各号で一致させる" }
  - { from: DesignCatalogue(components.md), to: SpecDocument(01 §3.3 / 11), cardinality: "one-to-many", description: "WorkspaceModel の縮退は 01 号 §3.3 と 11 号 §2 の改訂と同じ裁定（追加 1）" }
```

## 2. 要約

- 改訂対象は **coding-rules 4 ファイル**（うち `error-handling.md` は新規）、**仕様 5 ファイル**（01 / 10 / 11 / 12 号 + deviations）、
  **components.md 1 ファイル**。コードは触らない。
- 識別子はパス。各エンティティは「本 Unit で加える改訂（BR の ID）」を属性として持ち、rules.md の BR と 1:1 に対応する。
- 出典の規律: 改訂は ADR（001〜008）、契約（C4 / C5 改訂）、設計監査の確定裁定（R1〜R5）、Bolt B3 の実装済みコード、オーナー裁定（2026-08-23 の
  IntentId / WorkspaceModel / ドメインモデルの原則）のいずれかに遡れるものだけ。
