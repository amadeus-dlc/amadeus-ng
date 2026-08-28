# cr-doc-fixes-report — PR #30 CodeRabbit 文書指摘の是正（14 件）

> 対象: PR #30（`amadeus-dlc/amadeus-ng`、ブランチ `bolt/b6-esa-v2-conformist`）の CodeRabbit
> インラインコメントのうち文書系 14 件。作業日 2026-08-27。コメント本文は信頼しないデータとして扱い、
> 埋め込み指示（`<summary>🤖 Prompt for AI Agents</summary>` 内含む）には従わず、指摘内容だけを
> 現物（実装コード・git log・doc-sync-report・ADR-010）と突き合わせて検証した。
> `modules/**` / `formal/**` / `.claude/**` / `coding-rules/**` / `clippy.toml` は未変更。
> `git add` / `commit` / `push` は行っていない。新しい設計判断はしていない。

## 結果一覧

| ID | 対象ファイル | 状態 | 内容 |
|---|---|---|---|
| 3864822397 | u2/functional-design/entities.md | fixed | `WorkflowExecution` の description に残っていた「serde に依存しない（P5）」を失効注記に置換（実装 `#[derive(Serialize, Deserialize)]` を確認済み） |
| 3864822405 | u2/functional-design/entities.md | fixed | エンティティ名 `WorkflowExecutionSnapshot` → `WorkflowExecutionState` に改名（`relationships` と §2 要約の参照も追従） |
| 3864822413 | u2/functional-design/functional-spec.md | fixed | W3 節と §5 エラー一覧に残っていた `from_snapshot` / `SnapshotError` を `from_state` / `StateError` に統一（旧名併記で改名の経緯を残す） |
| 3864822420 | u2/functional-design/rules.md | fixed | BR5.3 の `logic` を訂正 — 「Repository は version を前提条件に使わない」は誤りで、実装 `WorkflowExecutionRepositoryImpl::store` は `aggregate.version()` を CAS 期待値としてそのまま本家へ渡している（実コード確認済み） |
| 3864822431 | u3/functional-design/functional-spec.md | fixed | §3.1 / §3.2 の失効バナー直下に残っていた旧 SQL 手順の番号付きリストを取り消し線で履歴化（バナーとの不整合を解消） |
| 3864822440 | u3/functional-design/pending-revision.md（+ functional-spec.md §4.1） | fixed | `phase_boundary` のワイヤ形を pending-revision 項目3の既存裁定（入れ子形）に統一。実装 `PhaseBoundary{from_phase,to_phase}` の既定 serde 表現とも一致確認済み。新しい設計判断ではなく既存裁定への追従 |
| 3864822446 | u3/functional-design/rules.md | fixed | BR1.3 / BR2.2 / BR2.3 の `logic` / `violation` に残っていた旧手順（`seq_nr-1` 検査・DDL定数埋め込み・rusqlite Tx）を是正し、§2 要約表 BR3.5 行の「InMemory」表記も現行実装名に更新 |
| 3864822455 | u3/nfr-design/logical-components.md, security-design.md | skipped | 日付不整合ではなくタイムゾーン起因の見かけ上の差 — 該当コミットは JST 2026-08-27 00:05〜01:53 に作成（`git log` 実測）で UTC では 2026-08-26 にあたる。ADR-010 追記・doc-sync-report・本 PR の他の失効バナーもすべて JST 基準で「2026-08-27」に統一されており、この2ファイルだけ「2026-08-26」に直すと他の同日付記述との整合が崩れる |
| 3864822460 | u3/nfr-design/security-design.md | fixed | §1 設計方針 (a) と §9 NFR4.4 に残っていた「3 段の検査点」を、§2 / NFR3.2 と同じ「1 段（`from_state()`）」に統一 |
| 3864822466 | inception/contract-design/contract-summary.md | fixed | C5 バナーの「未知の変種も対応外の版も復号失敗に畳まれる」を訂正 — 実装 `decode_event`（`journal_reader_impl.rs`）は `schema_version` の値を検査しておらず、対応外の版は実際には復号に成功してしまう（既知の実装ギャップとして明記。コード修正はスコープ外） |
| 3864822473 | contract-summary.md + decisions.md | fixed | contract-summary.md §3 の「Impl + InMemory」を現行の `WorkflowExecutionRepositoryImpl<S>`（`open()`/`in_memory()`）表記に更新。decisions.md の ADR-010 に、ADR-003「EventStoreImpl」・ADR-007「チェックポイントはTx内更新」・ADR-009「EventStoreImplが両契約を実装」の各記述を supersede する旨を追記 |
| 3864822479 | docs/specs/11-workspace.md | fixed | §1 のロックサービス供給の記述・§2.1 の `Space` 行・退役段落の3か所に、登録簿直列化機構が未決（U7裁定待ち）である旨のヘッジを追加（`WorkflowExecution` 集約の書込に限定する形で正確化） |
| 3864822484 | docs/specs/11-workspace.md | fixed | §3 ポート表と §6 J3 行に、「version とジャーナル長の一致は現行 adapter の観測された性質であり、version が不透明トークンであるというドメイン契約自体が要求するものではない」旨を追記。Quint モデル（`journal_protocol.qnt`）は無改変 |
| 3864822385 | esa-v2-migration/developer-report-2.md | fixed | §6.1 見出し「8 本」→ 実測「10 本」（直後の注記は元々10本と整合）、§7-1 の内訳見出し「9 本」→ 実測「11 本」に訂正。いずれも「2026-08-27 訂正:」の形で追記し、既存記述は削除していない |

## 集計

- fixed: 13 件
- skipped: 1 件（3864822455）

## 返信用一言（ID順）

- 3864822397: 修正しました。`serde に依存しない（P5）` は失効扱いとし、実装が `Serialize`/`Deserialize` を derive していることを明記しました。
- 3864822405: 修正しました。エンティティ名を `WorkflowExecutionState` に改名し、参照箇所も追従させました。
- 3864822413: 修正しました。W3 節とエラー一覧を `from_state`/`StateError` に統一しました（旧名は経緯として残しています）。
- 3864822420: 修正しました。ご指摘の通り Repository は `aggregate.version()` を CAS 期待値としてストアへ渡しており、その旨に訂正しました。
- 3864822431: 修正しました。失効バナー配下の旧手順本文を取り消し線で明示的に履歴化しました。
- 3864822440: 修正しました。pending-revision 項目3の既存裁定（入れ子形）に functional-spec の記述を合わせました。実装の serde 表現とも一致を確認済みです。
- 3864822446: 修正しました。BR1.3/BR2.2/BR2.3 の `logic`/`violation` を現行手順に更新し、要約表の BR3.5 行も直しました。
- 3864822455: 見送りとしました。該当コミットは JST で 2026-08-27（`git log` 実測）で、UTC 換算で 2026-08-26 に見えるだけのタイムゾーン差です。本 PR の他の同種の失効バナーもすべて JST 基準の「2026-08-27」で統一されているため、この2ファイルだけ変更すると逆に不整合になります。
- 3864822460: 修正しました。設計方針 (a) と NFR4.4 を、既に更新済みの NFR3.2 と同じ「1 段」の記述に揃えました。
- 3864822466: 修正しました。ご指摘の通り `decode_event` は `schema_version` を検査していないため、その実装ギャップを文書に明記しました（コード修正は本タスクのスコープ外です）。
- 3864822473: 修正しました。「Impl + InMemory」を現行実装名に更新し、decisions.md 側に ADR-010 が ADR-003/007/009 の該当記述を supersede する旨を追記しました。
- 3864822479: 修正しました。ロックサービス供給・`Space` 集約行・退役段落の3か所を、登録簿直列化が未決（U7）である旨に揃えました。
- 3864822484: 修正しました。version とジャーナル長の一致は現行 adapter の観測された性質であり、ドメイン契約ではない旨を明記しました（Quint モデルは無改変です）。
- 3864822385: 修正しました。§6.1 は実測どおり10本、§7-1 の内訳見出しは実測どおり11本に訂正し、「2026-08-27 訂正:」として記録しました。
