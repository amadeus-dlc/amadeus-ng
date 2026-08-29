# ハンドオフ — B12 完了・park（2026-08-30）

次セッションの再開: `/aidlc --resume aidlc/spaces/default/intents/260822-stage1-selfhost/handoff-u6.md を読んでから進めてください。`

## 再開直後の最優先タスク

**PR #38（B12）の収束**（park 時点で作成済み・収束ルール未完）: 常設監視 → unresolved×non-outdated 全数 sweep → 実否検証 → 返信 resolve → 「CI green ∧ unresolved 0 ∧ 全返信 ∧ bot 完了」を最新 head 再実測 → merge queue（AI 裁定マージ権限あり）。

## 現在地

- main = `f70dc28`（B11 / PR #37 まで着地）。**B12 = `bolt/b12-intent-aggregate-rename` ブランチ（push 済み・PR #38）** — 33+ コミット、全ゲート緑（838 テスト・coverage 98.619%・ゴールデン差分 0）
- B12 の中身: 集約 `WorkflowExecution` を **`Intent` 集約 + `IntentExecution` 集約**へ分割（1 intent : n 実行、`IntentExecutionId` 新設）、`WorkflowDefinition` も集約規則適合、**ドメインの永続化中立化**（serde / ESA / manifest 全撤去 → adapter `wire/` + RMU `wire/` が側ごと所有）、`CommitVerdictUseCase` はリポジトリ 2 本保持（`IntentRepository` ポート前倒し新設）
- 検収の要: ワイヤ形式バイト不変（書き手/読み手が同一リテラルを独立保持）・投影ゴールデン 19 本無改変・upstream 外形 1 バイト不変

## このセッションで確立した設計正典（coding-rules、21 本 + 既存増補）

- **aggregate-references.md**（新設）: 集約は他集約を ID 参照。埋め込み禁止。判断材料は `&` パラメータ + id 照合ガード。イベント上の写しは歴史
- **aggregate-commands.md**（新設）: コマンドとファクトリは (集約, イベント) を返す（無ければ Repository で永続化できない）。再構成（from_snapshot / apply / from_artifacts / from_material）はイベントを生成しない。CQS の「Command は戻り値なし」は集約に不適用
- **domain-persistence-neutrality.md**（新設）: domain に serde 属性・ストア trait・ワイヤ判別子・DTO 双子を書かない（機械強制 = `[dependencies]` に serde/ESA 不在）。**概念/機構の線引き**: AI-DLC の概念（監査語彙 86 語・StorePath 等のワークスペース配置）は domain に残る — 消費場所でなくユビキタス言語が所有を決める
- **use-case-rules §2b**（増補）: execute 引数は集約 ID + VO のみ（集約インスタンス禁止）。書込ユースケースはリポジトリを保持し内部で使う。旧 I8 機構（Controller が集約を & で渡す）は失効 — 読取専用の型保証は find 系のみの読取専用ポート注入へ
- gateway-taxonomy 増補: Repository は自集約のみ・署名は自集約 ID のみ・再生材料は自ストリームの誕生イベントから内部復元

## 集約モデルの最終形

| 集約 | ID | genesis | イベント |
|---|---|---|---|
| `Intent`（静的な intent） | `IntentId` | `create → (Intent, IntentEvent::Created)` | Created |
| `IntentExecution`（1 回の実行） | `IntentExecutionId` | `start(id, intent, at) → (…, Started)` | Started ほか 11 種（`Started` は intent を丸ごと運ぶ = BR2.2） |
| `WorkflowDefinition`（プロセス定義） | `WorkflowDefinitionId` | `define → (…, Defined)` | Defined（将来 `ScopeComposed` 等 — 実ファイル 3 点はこの集約のリードモデル、オーナー承認済み方針） |

## 次の作業候補（PR #38 収束後）

1. **（推奨）U6: next・continue** — 21 分岐ラダー。設計条件は確定済み: `next_decision` は集約クエリ（domain 実装済み）、読取専用ポート注入（§2b — 旧 I8 機構は使わない）、execute 引数は ID + VO
2. **裁定 6 の追随 PR**（小粒）: `CorruptCause` をポート契約から退避 → `source` 連鎖へ（`u5-report-use-case/decisions-1.md` 裁定 6）
3. specs 4 本（01/10/11/12）の改名全文追従（バナー注記済み・本文は旧名のまま）
4. FR2.2（着手前に「B10 述語」の指す先のオーナー確認が必要 — `decisions-1.md` 申し送り (a)）

## 申し送り（主要な未決）

- **U7 の設計点**: `IntentRepository` 実物実装（読み先 = intent 自身のジャーナル導入）、intent → 最新実行の解決、「生きた実行は同時に 1 つ」不変条件、クエリサイド（`core/query/{interface-adapter, use-case}` — adapter にコントローラ/ゲートウェイ、use-case 層は必要と立証されるまで作らない）、RMU 非同期投影（合流点 2 つ: .md はプロセス終了前・クエリ表は読む直前）、SQLite クエリ表投影
- intent / 定義のジャーナルを起こす際は**別 manifest 値**が要る（B12 報告 §6）
- 「実行ごとに計画を再解決」要件が出たら stages を実行の開始材料へ移す再裁定（B12 改訂 5 注記）
- 詳細は `construction/intent-aggregate-rename/brief-1.md`（改訂 1〜10）・`developer-report-1.md`・`command-domain-audit/audit-1.md`・`u5-report-use-case/decisions-1.md`

## 効いている運用規約（変更なし + 追加）

- 収束ルール・AI 裁定マージ権限（project.md Corrections 登録済み）
- 委任の規律: ブリーフに固定裁定・所有ファイル・受入基準、報告は鵜呑みにせず全ゲート独立再実行。**行き違い対策: 委任先は着手前に受信箱を読み切る**（B12 で 3 回交差した教訓）
- 規則正本は **21 本**（+ README）。オーナーの設計裁定は即・正典化し、誤適用の経緯ごと規則に記録する
