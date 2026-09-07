# entities — U9 正本・仕様の canon 追従（`u9-canon-docs`）

> Functional Design（Construction 3.1）成果物（Unit: U9、kind: spec）。**改訂履歴**: 初版 2026-08-23（Bolt B4 向け）→ **再走 2026-09-07（本版、Modify）**:
> 現行コード（`main` `02cacea2`）基準の実測 `gap-measurement-20260907.md` と Bolt / Unit 記録の全数探索 `survey-20260907/` に合わせ、改訂対象の
> インスタンスと改訂 ID を同期した。出典: `../../../inception/units-generation/unit-of-work.md`（U9）、`../../../inception/units-generation/
> unit-of-work-story-map.md`（FR8 / FR8.1 / FR8.2 / FR9.6）、`../../../inception/requirements-analysis/requirements.md`（制約 C2 / C4）、
> `../../../inception/domain-design/components.md`、`../../../inception/contract-design/contract-summary.md`、`../../../inception/domain-design/decisions.md`
> （ADR-001〜011）、確認質問 `functional-design-questions.md`（Q1〜Q4、追加 1・2、要約確認 2026-09-07 Looks correct）。
>
> U9 は文書だけの Unit。ここでの「エンティティ」= **改訂対象の正本文書**（識別子はリポジトリ相対パス）。下の fenced `yaml` が正本。
> `revisions_in_b4` は B4（2026-08-23〜）で適用した改訂 ID、`revisions_in_rerun_20260907` は本再走の Bolt で適用する改訂 ID（rules.md の `status: open`）。

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
      - { name: revisions_in_b4, type: list<string>, required: true, constraints: "B4 で適用した改訂（BR の ID、status done / superseded）" }
      - { name: revisions_in_rerun_20260907, type: list<string>, required: true, constraints: "本再走で適用する改訂（BR の ID。原則 status open — BR3.2 だけは done だが『予定』の明記を本再走で追記するため含める）。空 = 本再走では触らない" }
    instances:
      - { path: "coding-rules/use-case-rules.md", revisions_in_b4: [BR1.1], revisions_in_rerun_20260907: [BR1.6], note: ":11 クレート名、:36 テストダブルの記述" }
      - { path: "coding-rules/gateway-taxonomy.md", revisions_in_b4: [BR1.2, BR1.3, BR1.4, BR1.5], revisions_in_rerun_20260907: [BR1.6], note: "BR1.5（§1b 一般形）は実施済み・記録登録が本再走。:223 集約名、:299 クレート名" }
      - { path: "coding-rules/error-handling.md", revisions_in_b4: [BR4.1], revisions_in_rerun_20260907: [BR1.6], note: "現行本文が正本（BR4.1）。:4 適用例のエラー型名" }
      - { path: "coding-rules/README.md", revisions_in_b4: [BR4.2], revisions_in_rerun_20260907: [BR1.6, BR4.2], note: ":50 message-catalog、:115 WorkflowExecutionState 是正対象表記" }
      - { path: "coding-rules/factory-naming.md", revisions_in_b4: [], revisions_in_rerun_20260907: [BR1.6], note: ":47 反例の履歴化、:84 with_version の型名" }
      - { path: "coding-rules/module-visibility.md", revisions_in_b4: [], revisions_in_rerun_20260907: [BR1.6], note: ":11 / :21 クレート名、:12 message_catalog の例" }
    constraints:
      - "README の一覧と各ファイルの一言・強制手段が矛盾しない（U9 の合格条件、BR4.2 / BR5.1 (b)）"
      - "是正の経緯・退役の履歴として書かれた旧名（command-query-separation / interior-mutability / domain-equality / tell-dont-ask / aggregate-references / cqrs-boundaries / field-visibility / good-examples / ubiquitous-language）は触らない（BR1.6 logic）"

  - name: SpecDocument
    description: "仕様の正本（`docs/specs/NN-*.md`、00-policy が最上位）。upstream 互換の観測可能契約とクリーンアーキテクチャへの写像を規範として持つ。コード実装の判定基準。本再走では『正しい姿』の正本を `gap-measurement-20260907.md` §4 とし、改訂箇所は同 §2.1〜§2.4 の行単位表で定める"
    attributes:
      - { name: path, type: string, required: true, unique: true }
      - { name: sections_revised, type: list<string>, required: true, constraints: "本再走で改訂する節（§2 表の行）" }
      - { name: revisions_in_b4, type: list<string>, required: true }
      - { name: revisions_in_rerun_20260907, type: list<string>, required: true }
    instances:
      - { path: "docs/specs/10-orchestration.md", sections_revised: ["冒頭注記 2 種の畳み込み（:3-15）", "§2.1 集約を Intent + IntentExecution に分割（:43-56。12 + 7 属性、コマンド 15 + start、イベント 16、version 集約内、差分再生、next_decision の Result）", "§2.2 EffectivePlan の所有名、Directive 系の所在 = クエリ側（:66-76）", "§2.3 名称・署名（:82-87）", "§3 ユースケース列と 4 ポートの表、テストダブル記述の廃止、未実装は予定（:93-110）", "I8 / 実装順序 / S1 の名称（:137 / :205-206 / :236）"], revisions_in_b4: [BR2.3, BR2.4, BR3.1, BR3.3], revisions_in_rerun_20260907: [BR3.3] }
      - { path: "docs/specs/12-workflow-definition.md", sections_revised: ["冒頭注記の畳み込み", "§2.3 所有名 + scope_cost / review_policy / stage_route の追記（:77-93）", "§4 #7・§8 F2 / F8 の名称（:164-165 / :215 / :221）", "§5 ユースケースを DefineWorkflow + Find* 5 + read_definition* 6 表へ、find_for_intent 追記（:178 / :183）"], revisions_in_b4: [BR2.5, BR3.1, BR3.3], revisions_in_rerun_20260907: [BR3.3] }
      - { path: "docs/specs/11-workspace.md", sections_revised: ["冒頭注記の畳み込み", "§2.1 集約 3 は予定と明記、orchestration の Intent との関係注記（:41-49）", "§2.3 / §4 に RMU 二層・read_* 17 表・同期 catch_up", "§3 ポート行を IntentExecutionRepository へ、供給面 4 は予定（:101 / :111-116）", "§4 / §7 の実装名（:126 / :195）"], revisions_in_b4: [BR2.1, BR3.2], revisions_in_rerun_20260907: [BR3.2, BR3.3] }
      - { path: "docs/specs/01-domain-model.md", sections_revised: ["冒頭注記の畳み込み", "§3.1 所有名（:96）", "§3.2 集約段落の書き直し（:102 / :106）", "§3.3 は予定と明記、脚注 :121 の履歴化", "§4 B1・§6 の名称（:184 / :231）", "§7.1 原則 6 件の追記と原則 5 の例を 2 段に（:269-280）"], revisions_in_b4: [BR2.2, BR2.4, BR3.1, BR3.2, BR3.6], revisions_in_rerun_20260907: [BR3.2, BR3.3] }
      - { path: "docs/specs/deviations.md", sections_revised: [], revisions_in_b4: [BR3.4], revisions_in_rerun_20260907: [], note: "本再走では触らない（逸脱登録は実測 1 行で確認）" }
    constraints:
      - "各改訂は gap-measurement §4 の台帳（コード検証済み）に遡れるものだけ。§4 に無い主張を仕様に足さない（BR3.3 violation / BR5.3）"
      - "逐語の upstream 契約（D6）は変更しない — 変えるのは構造の規範と所有の記述だけ（BR5.2）"
      - "未実装（§4.7）は『予定』と明記し、実装済みのように書かない"
      - "『維持』行（§2 表）は変えない"

  - name: DesignCatalogue
    description: "Inception の共有契約・設計成果物のうち、本 Unit が現行へ追従させるもの"
    attributes:
      - { name: path, type: string, required: true, unique: true }
      - { name: revisions_in_b4, type: list<string>, required: true }
      - { name: revisions_in_rerun_20260907, type: list<string>, required: true }
      - { name: revision_mode, type: enum, required: true, allowed_values: [full, per-section, note-only], constraints: "全面 / 節単位 / 注記のみ" }
    instances:
      - { path: "aidlc/spaces/default/intents/260822-stage1-selfhost/inception/domain-design/components.md", revisions_in_b4: [BR3.5], revisions_in_rerun_20260907: [BR3.7], revision_mode: full, note: "冒頭『全再生』→ 差分再生（R-02）、クレート 10・集約 4・クエリ側・RMU・read_* 17 表を YAML に" }
      - { path: "aidlc/spaces/default/intents/260822-stage1-selfhost/inception/contract-design/contract-summary.md", revisions_in_b4: [], revisions_in_rerun_20260907: [BR3.7], revision_mode: per-section, note: "C1 / §4 message-catalog、C3 trait 4 ポート、C4 Intent、C5 16 変種、C6 read_* 表。§4 未決 1 件は残す。打消し線つき旧 manifest 綴りは触らない" }
      - { path: "aidlc/spaces/default/intents/260822-stage1-selfhost/inception/units-generation/unit-of-work.md", revisions_in_b4: [], revisions_in_rerun_20260907: [BR3.7], revision_mode: note-only, note: "U3 の失効記述（:34 / :83 / :91 / :144）に ADR-010 / B12 失効注記。本文は書き換えない" }
      - { path: "aidlc/spaces/default/intents/260822-stage1-selfhost/inception/domain-design/decisions.md", revisions_in_b4: [], revisions_in_rerun_20260907: [], revision_mode: note-only, note: "変更しない — ADR-009 改訂注記（:266 / :286 / :297）と ADR-010 serde 失効注記（:441）が既にある" }

  - name: MeasurementRecord
    description: "本再走の根拠記録（U9 functional-design の補助記録。正本ではないが、仕様の『正しい姿』の裁定台帳 §4 を持つ）"
    attributes:
      - { name: path, type: string, required: true, unique: true }
      - { name: code_baseline, type: string, required: true, constraints: "実測の基準コミット" }
      - { name: role, type: string, required: true }
    instances:
      - { path: "construction/u9-canon-docs/functional-design/gap-measurement-20260907.md", code_baseline: "main 02cacea2", role: "§1 コードの現状 / §2 文書側のずれ（行単位）/ §3 作業パッケージ / §4 裁定台帳（4 区画統合・コード検証済み）/ §4.8 矛盾の裁き / §5 仕分け" }
      - { path: "construction/u9-canon-docs/functional-design/survey-20260907/{shift-bolts,late-bolts,u2-u3,other-units}.md", code_baseline: "main 02cacea2", role: "Bolt / Unit 記録 4 区画の一次台帳（出典 path:line つき）" }

relationships:
  - { from: CodingRule(README.md), to: CodingRule(*), cardinality: "one-to-many", description: "README が全規則ファイルを索引する" }
  - { from: SpecDocument, to: CodingRule, cardinality: "many-to-many", description: "仕様が規則ファイルを参照する。本再走で未参照の 12 本へ相互参照を付ける（BR3.3 (j)）" }
  - { from: SpecDocument(01), to: SpecDocument(10/11/12), cardinality: "one-to-many", description: "01 号のコンテキストマップと集約表が各コンテキスト仕様の上位。集約・所有の記述は 01 号と各号で一致させる" }
  - { from: DesignCatalogue(components.md / contract-summary.md), to: SpecDocument(10/11/12), cardinality: "many-to-many", description: "共有契約と仕様は同じ台帳（gap-measurement §4）から書く。C3 ↔ 10 号 §3、C5 ↔ 10 号 §2.1、C6 ↔ 11 号 §2.3" }
  - { from: MeasurementRecord(gap-measurement §4), to: SpecDocument / DesignCatalogue / CodingRule, cardinality: "one-to-many", description: "台帳の各行（O / P / R / W / S / K）が改訂の根拠。台帳に無い主張は書かない" }
```

## 2. 要約

- 改訂対象は **coding-rules 6 ファイル**（12 行、BR1.6 / BR4.2）、**仕様 4 ファイル**（01 / 10 / 11 / 12 号、BR3.2 / BR3.3。deviations は触らない）、
  **共有契約 3 ファイル**（components.md 全面、contract-summary.md 節単位、unit-of-work.md 注記のみ、BR3.7）。コードは触らない。
- 識別子はパス。各エンティティは B4 の改訂 ID と本再走の改訂 ID を別属性で持ち、rules.md の BR（status）と 1:1 に対応する。
- 出典の規律（BR5.3）: 改訂は gap-measurement §4 のコード検証済み台帳に遡れるものだけ。記録側の主張とコードが食い違う 13 論点は §4.8 の裁き（コード採用）に従う。

## Review 履歴（2026-08-23、iteration 2、READY）

> 初版に対する回復レビュー。所見 #1〜#3 は pending-revision 項目 1〜5 として保留され、**本再走（2026-09-07）で BR2.5 / BR1.5 / BR5.1 に反映した**。

| # | Severity | 要旨 | 本再走での扱い |
|---|---|---|---|
| 1 | Major | BR2.5 の範囲が 12 号 §2.3 に限られ、`next_in_scope_stage` の他 3 箇所（§4 / §8 / §9）が BR5.1 の grep を通らない | BR2.5 を全出現に改訂。実測で 12 号の出現 0 件を確認（done） |
| 2 | Major | gateway-taxonomy §1b が退役済み `WorkspaceLock` を模範例として提示し続ける | BR1.5 新設。§1b は一般形へ改訂済み（:67）を実測で確認（done） |
| 3 | Minor | BR5.1 の grep 範囲が未定義、`StageGraphReader` は除去対象の BR が無い | BR5.1 に範囲（coding-rules + docs/specs、research 除く）と履歴除外の判定を明記、`StageGraphReader` を sentinel から外した |

iteration 1 所見 4 件（BR2.3 の削除対象、BR3.5 の帰属、BR1.1 / BR3.3 の出典注記）は当時解消済み。当時のセンサー結果（traceability `gaps` / `orphans` / `invalid_targets` 空、
required-sections pass）は履歴であり、本再走の承認根拠には使わない。
