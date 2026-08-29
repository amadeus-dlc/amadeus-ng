# B12 委任ブリーフ 1 — 集約 WorkflowExecution → Intent の改名

Conversation language: 日本語
委任先モデル: Opus（機械的改名だが範囲が広く、改名対象外との切り分け判断を含む）
最終責任: Fable 5 メインセッション（全 diff レビュー・検証の独立再実行・受入判定）

## 裁定（オーナー確定 2026-08-29。変更禁止）

**集約 `WorkflowExecution` を `Intent` に改名する。** 根拠:

1. intent の生きた状態（ステージ位置・承認・park・スコープ・進行）を持ち、遷移し、判断する
   のはこの集約そのものであり、ドメインの一次名詞「intent」の実体はこの集約である。
   `intents.json` の行 `{uuid, slug, dirName}` は一覧用の索引にすぎない。
2. 監査語彙 `WORKFLOW_*` はリードモデルのバイト列であり、リードモデルは要件の違う別データ
   （オーナー裁定）。その語彙は集約の名前を縛らない。
3. 「workflow」の名は定義側（`WorkflowDefinition` = ステージグラフ + スコープグリッド）の
   持ち物。現状は workflow の語が定義と実行の二重語義になっており、`Intent` が
   `WorkflowDefinition` を参照して進む、と直せば語義が一対一に戻る。
4. これにより `IntentId` は Entity + Id 法則（ID は必ずエンティティ名 + Id）にそのまま適合する。

## 改名の一族（すべて同時・機械的）

| 旧 | 新 |
|---|---|
| `WorkflowExecution`（集約） | `Intent` |
| `WorkflowExecutionEvent` | `IntentEvent` |
| `WorkflowExecutionState` | **`IntentSnapshot`**（訂正 2026-08-29 — 状態を担うのは集約であり「State」を名乗らせない。正体はスナップショットの直列化形（memento）。あわせて**クレート内私有へ降格**— ドメイン外から使う箇所ゼロを実測済み。serde の `into`/`try_from` 文字列パスも追随） |
| `WorkflowExecutionStateBuilder` | **`IntentSnapshotBuilder`**（オーナー確定 2026-08-29 — 名前が示すとおり組むのは `IntentSnapshot`。`build()` は従来どおりスナップショットを返し、検査は `from_state` / `TryFrom` 側が担う。可視性はスナップショットと同じくクレート内私有） |
| `WorkflowExecutionRepository`（ポート） | `IntentRepository`（gateway-taxonomy「集約名 + Repository」に自動追従） |
| `WorkflowExecutionRepositoryImpl` | `IntentRepositoryImpl` |
| `InMemoryWorkflowExecutionRepository`（テスト fake） | `InMemoryIntentRepository` |
| `RehydratedWorkflowExecution` | `RehydratedIntent` |
| `AGGREGATE_TYPE_NAME = "WorkflowExecution"`（本家 trait `type_name`） | `"Intent"` |
| ファイル名 `workflow_execution*.rs` | `intent*.rs` 系へ（`git mv` を使う） |

- 集約内のフィールド `intent_id` とアクセサ `intent_id()` は **`id` / `id()` へ**（`Intent { intent_id }` は冗長。
  スナップショットの serde フィールド名が変わるが、ジャーナル・スナップショットはクローンごとの
  使い捨てランタイム＝gitignore 済みで互換問題なし。`no-backward-compatibility.md` どおり互換シムも置かない）。
- `RepositoryError::NotFound { intent_id }` は「探した intent の id」という材料名なので**そのままでよい**。
- `IntentId` は**改名しない**（正しい名前だったのはこちら）。`intent_id.rs` の見出し doc
  「集約 `WorkflowExecution` の識別子」は「集約 `Intent` の識別子（`intents.json` の uuid・
  記録ディレクトリの id8）」へ是正。
- doc コメント内の「集約 WorkflowExecution」「再構成される WorkflowExecution」等の散文も全て追随。

## 改名対象外（触ったら違反）

- **`WorkflowDefinition` 一族**（`WorkflowDefinitionId` / `WorkflowDefinitionRepository` / …）— workflow の名の正当な持ち主
- ドメインイベントの**変種名**（`Started` / `GateOpened` / `GateApproved` / …）と payload 構造
- `IntentId`・`IntentDirName`
- 監査語彙（`WORKFLOW_STARTED` 等）と投影出力 — **外形は 1 バイトも変えない**
- `tests/golden/**`（1 バイト不変）、`formal/**`（Quint モデルは Rust 型名を参照しない）、
  `docs/**`、`coding-rules/**`（正本の失効注記はメインセッションが実施）、
  `aidlc/**`（本ブリーフ・報告書を除く）、`.claude/**`

## 所有ファイル・規律

- 書いてよい: `modules/**`（上記対象のみ）、報告書
  `aidlc/spaces/default/intents/260822-stage1-selfhost/construction/intent-aggregate-rename/developer-report-1.md`
- `git add -A` 禁止（明示パス。`git mv` 後の状態確認を怠らない）。**push 禁止**。
  検証は `CARGO_TARGET_DIR=$PWD/target-delegate`。コミットは意味単位・日本語・`b12: ` 接頭辞。
- 固定フィクスチャ（ITF・ゴールデン・逐語アサート）に旧名や `"intent_id"` フィールドの
  **バイトが埋まっていた場合は止めて報告**（事前 grep では検出 0 件だが、発見したら独断で
  書き換えない）。

## 受入基準（すべて緑）

1〜7. B11 と同一（fmt / clippy / cargo lint / `cargo test --workspace` 退行 0 / quint / coverage 相対 / unwrap 0）
8. **外形不変**: `tests/golden/**` 差分 0、投影ゴールデン 19 本無改変で全緑
9. `grep -rn "WorkflowExecution" modules/ --include='*.rs'` が **`WorkflowDefinition` 文脈を除き 0 件**
   （`WorkflowExecution` の部分一致で `WorkflowDefinition` は引っかからないので実質 grep 0 件）
10. `git log --follow` でファイル履歴が追えること（`git mv` を使った証跡）
11. 報告書: 改名対応表の実測、フィクスチャ確認の結果、迷った点

## 補記 — 旧名の由来と、将来の再分割トリガー（オーナー考察 2026-08-29）

旧名 `WorkflowExecution` の意図は、おそらく「`WorkflowDefinition` とその実行」という対で
あり、「Intent を実行したときのランタイム文脈」を表したかったもの。ここで
`WorkflowDefinition` は「計画」ではなく**全 intent 共通のプロセス定義（カタログ）** —
stage-graph + scope-grid + scopes — である点に注意（オーナー指摘 2026-08-29 で言い直し）。
**静的な計画（この intent 向けに解決済みの EXECUTE/SKIP 列・scope・request）は `Started`
イベントが運び、実行時文脈（cursor・checkbox・park 等）ともども `Intent` 集約が持つ** —
「Intent = 静的な計画 + 実行時文脈」が正しい整理。recompose（計画の作り直し）も
`Recomposed` イベントとして Intent に起き、定義側は不変（`definition_revision` でピン）。静的な Intent +
実行時の WorkflowExecution という分割の絵に近いが、**静的側の Intent は一度もモデルとして
彫られなかった**（台帳行と IntentId だけが存在）— ID があるのにエンティティが無い混乱の根。

この分割が働きを持つのは「**1 つの Intent に複数の実行がありうる**」要件（再実行・リプラン
別走行など）が現れたときで、そのとき初めて `ExecutionId ≠ IntentId` となり両モデルが別々の
同一性を持つ。現システムは 1 intent = 1 実行・静的情報は `Started` イベントが運ぶため、
分割は代金だけで配当が無い。**`Intent` への統一は現要件では正しい形**であり、上記要件が
現れた時点が `Intent`（静的）/ `WorkflowExecution`（実行ごと）へ分け直すトリガーである。
