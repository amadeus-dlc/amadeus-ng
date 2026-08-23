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
      - { path: "docs/specs/01-domain-model.md", sections_revised: ["§3.1 workflow-definition（集約に id / revision）", "§3.2 orchestration（WorkflowExecution の ES 形、PlanAction 所有）", "§3.3 workspace（集約候補の ES 化後の姿、IntentId = UUIDv7 と IntentDirName）", "§7 クリーンアーキテクチャへの写像原則（ドメインモデルの原則の明記）"], revisions_in_b4: [BR2.2, BR2.4, BR3.1, BR3.2, BR3.6] }
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

## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-08-23T05:07:11Z
**Iteration:** 2（advisory, recovery, unit: u9-canon-docs）

### Findings

| # | Severity | Location | Finding | Recommendation |
|---|---|---|---|---|
| 1 | Major | rules.md BR2.5 ↔ BR5.1 | BR2.5 の改訂範囲は「12 号 §2.3 / §5 / 実装ノート」に限定されているが、`docs/specs/12-workflow-definition.md` 内の `next_in_scope_stage` の実出現は 5 箇所（68・73 行 = §2.3、143 行 = §4「読込の失敗態度」、193 行 = §8 不変条件表 F2 行、209 行 = §9「実装順序」のユビキタス言語例）で、うち 3 箇所（§4 / §8 / §9）は BR2.5 の宣言範囲外。この 3 箇所はいずれも現行仕様として提示されており（「暫定規範」「ドメイン例」等）、履歴注記ではない。一方 BR5.1 は合格条件として「`next_in_scope_stage` = 0 件（履歴注記を除く）」を全体 grep で要求しており、BR2.5 を文字どおり実施すると BR5.1 自身の受入 grep を通らない（または §4/§8/§9 を独自判断で改訂することになり、rules.md にその指示が無い）。 | BR2.5 の適用範囲に §4（未知スコープ表の該当行）・§8（F2 行）・§9（ユビキタス言語例文）を明示的に含めるか、あるいは「構造上の API 名としての言及」と「挙動記述としての言及」を区別して BR5.1 の grep 対象を §2.3/§5 の API 名列挙に限定する旨を明記する。 |
| 2 | Major | coding-rules/gateway-taxonomy.md §1b | BR1.1〜BR1.4 は gateway-taxonomy.md の §2 / §2b / §3 / §4 を改訂対象にしているが、§1b「非 Repository ポートの模範例 — `WorkspaceLock`」（`acquire(&LockIdentity, AcquireBudget) -> Result<LockGuard, AcquireError>` / `release(LockGuard)` の具体シグネチャつき）は本 Unit のどの BR にも含まれていない。しかし BR3.2（ADR-007 準拠）は `WorkspaceLock` を「退役（SQLite Tx + 楽観 version へ）」と明記しており、01 号・11 号の改訂後は `WorkspaceLock` という並行性ポートが正本から消える。coding-rules の「模範例」節だけが退役済み機構を現行の設計指針として提示し続けることになり、AuditLedgerRepository（BR1.4 が同じ理由で除去する対象）と同型の陳腐化した参照が残る。 | §1b を改訂対象に加える（BR1.x へ追加、または新設の BR）。退役の旨を注記するか、模範例を非 Repository ポートの一般形（型にロック意味論を載せる設計パターン）として再構成し、`WorkspaceLock` という具体名への依存を外す。 |
| 3 | Minor | rules.md BR5.1 | BR5.1 の grep 対象 `StageGraphReader` は、AuditLedgerRepository / AuditLedgerService / StateFileStore と異なり、どの BR（BR1.x〜BR4.x）にもこの語の除去指示が無い。実測では `coding-rules/gateway-taxonomy.md:92` の禁止名テーブル（`StateFileStore` 行と同型の意図的な記録）と `docs/specs/research/*.md`（entities.md の改訂対象に含まれない研究文書、5 箇所）にのみ現存する。BR5.1 は grep の探索範囲（ディレクトリ）を明示していないため、`docs/specs/research/` を含めて解釈すると本 Unit の改訂だけでは達成不能な合格条件になる。 | BR5.1 の grep 範囲を `coding-rules/*.md` + `docs/specs/*.md`（`research/` を除く）に明記するか、`StageGraphReader` を禁止名テーブルの正当な記録として除外注記する。除去対象の BR が無いなら sentinel リストから外すことも検討。 |

### iteration 1 所見の解消状況

| # | iteration 1 所見（要旨） | 解消状況 | 確認方法 |
|---|---|---|---|
| 1 | BR2.3 に 10 号 §3 の `AuditLedgerRepository` 行・`WorkspaceLock` 行の削除と `WorkflowExecutionRepository` 実装欄の書き換えを明記（BR5.1 の grep と同期） | 解消 | rules.md BR2.3 本文で明記を確認。`docs/specs/10-orchestration.md:78`（AuditLedgerRepository 行・「同上」）・80 行（WorkspaceLock 行）の実在を確認し、改訂対象が実体を持つことを検証 |
| 2 | entities.md の `01-domain-model.md` インスタンスから `BR3.5` の誤帰属を除去 | 解消 | entities.md の `SpecDocument(01-domain-model.md)` の `revisions_in_b4` は `[BR2.2, BR2.4, BR3.1, BR3.2, BR3.6]` で `BR3.5` を含まない。`BR3.5` は `DesignCatalogue(components.md)` にのみ帰属（正しい） |
| 3 | BR1.1 の `source` に `find_by_id()` 採用理由 | 解消 | rules.md BR1.1 の `source` に「§2b は find / find_by_id の両方を許容するためオーナー確認は不要」の記載を確認。`coding-rules/gateway-taxonomy.md` §2b の許容動詞一覧（`find_by_id` / `find` / `save` / `remove`）と整合 |
| 4 | BR3.3 の `source` に U2 出典の鮮度注記 | 解消 | rules.md BR3.3 の `source` に「2026-08-23 時点で U2 機能設計の最終受領は NOT-READY・pending-revision 未適用」の注記を確認。実測でも `construction/u2-domain-es-core/functional-design/entities.md` に `## Review` 節が無く、同ディレクトリの `pending-revision.md`（更新時刻がより新しい）が存在することを確認し、注記の正確性を裏付けた |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| `aidlc-sensor-traceability.ts --stage functional-design` | `pass:false`（`missing_from_upstream_ids` に FR1〜FR7・FR8.3/8.4・FR9.1〜9.5 が列挙）。ただし `gaps: []` / `orphans: []` / `invalid_targets: []` は空 | dispatch が明示した合格基準（`invalid_targets` / `gaps` / `orphans` が空）はすべて満たす。`missing_from_upstream_ids` の不一致は、センサーがリポジトリ全体の FR/NFR 一覧を基準に照合する一方、u9 の `traceability.json` は自ユニット責務の `FR8.1 / FR8.2 / FR9.6` のみを列挙する構造的な帰結（他の FR は U1〜U8・U10 の責務）であり、u9 固有の欠陥ではない |
| `aidlc-sensor-required-sections.ts` — entities.md | `pass:true`（H2: `## 1. エンティティ（正本）` / `## 2. 要約`） | 問題なし |
| `aidlc-sensor-required-sections.ts` — rules.md | `pass:true`（H2: `## 1. 規則（正本）` / `## 2. 規則の要約`） | 問題なし |

### Summary

iteration 1 の所見 4 件（BR2.3 の削除対象明記、BR3.5 の誤帰属除去、BR1.1/BR3.3 の出典注記）はすべて実測で解消を確認した。今回新たに、BR の改訂範囲と BR5.1 の受入 grep 条件との間に 2 件の不整合（`next_in_scope_stage` の §4/§8/§9 未改訂と `WorkspaceLock` の模範例節が退役後も coding-rules に残る）を検出し、Major として計上した。両方とも文書のみの Unit の性質上、実装（コード）を壊すものではなく、B4 の PR 作成時に「BR を文字どおり実施すると自らの合格条件を満たせない」という手戻りを招く設計ギャップであり、オーナー/アーキテクトの一言の裁定で解消できる規模。`StageGraphReader` の grep 範囲の曖昧さは Minor として付記した。Critical 0・Major 2 のため advisory 判定は READY。承認前にこの 2 件（特に #1・#2）の裁定を推奨する。
