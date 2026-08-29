# doc-sync-report — Bolt B8（CQRS 側分割 + U4 ReadModelUpdater）後の設計文書同期

> 実施日 2026-08-29。ブランチ `bolt/b8-u4-read-model-updater`。
> 正とした順序: (1) `brief-2.md`（作業内容 1〜5）、(2) `developer-report-1.md` §1（クレート対応表）・
> §8（ドリフト一覧 #6・#7・#10）・§6（オーナー裁定 A の実装内容）、(3) `crate-structure-proposal.md`、
> (4) `decisions.md` ADR-008 追記（2026-08-29・Bolt B8）、(5) B6/B7 の `doc-sync-report.md`（同期
> スタイルの前例）、(6) 実装の現物（`modules/core/domain/src/orchestration/{stage_display,
> workspace_scan,stage_entry,workflow_execution_event}.rs` — フィールド名 `display` / `scan` の
> 実測確認に使用）。
>
> **新しい設計判断はしていない。** 確定済みの裁定（側分割・crate 改名・オーナー裁定 A）と実装の
> 実態を反映し、失効箇所には B6/B7 と同じ家内書式（`~~打ち消し~~ → **失効（日付・Bolt）**: 新`）で
> 日付付き注記をした。既存の記録は一行も削除していない。コード・`formal/**`・`Cargo.*`・
> coding-rules・memory・decisions.md（ADR は委任者が追記済み）は未変更。`git add` は明示パスのみ、
> `commit` は 1 本、`push` はしていない。

## 1. 変更したファイルと要点

| # | ファイル | 主な変更 |
|---|---|---|
| 1 | `docs/specs/00-policy.md` | D4 行・A8 行の `infra-io` を `core-infrastructure`（`modules/core/infrastructure`）への改名・移動として失効注記。`harness-infrastructure` 新設も追記 |
| 2 | `docs/specs/01-domain-model.md` | §3.2 orchestration 集約の説明にオーナー裁定 A（`StageEntry` への `StageDisplay`、`Started` への `WorkspaceScan`）を追記。Domain Primitive 候補に `StageDisplay` / `WorkspaceScan` を追加。§7 クリーンアーキテクチャ写像原則の `infra-io` 2 箇所を `core-infrastructure` 改名として失効注記 |
| 3 | `docs/specs/10-orchestration.md` | §2.1 `WorkflowExecution` の `stages`（`StageEntry`）に `display` フィールド追加を失効注記。`Started` のドメインイベント記述に `scan`（`WorkspaceScan`）フィールド追加を追記 |
| 4 | `docs/specs/11-workspace.md` | §2.3 冒頭・表の `render_audit_block` / `state_writers` の「投影 API へ転居予定」を「実施済み（B8、`core-query-read-model-updater`）」へ更新。`find_all_events` を「ドメインに残る」から「順序付けの純関数（domain）とシャード列挙・ファイル読取 I/O（投影側）に分割」へ訂正（開発者報告 §7-7 が根拠） |
| 5 | `.../inception/contract-design/contract-summary.md` | C3 に Bolt B8 追記（`core-use-case`→`core-command-use-case`、`core-interface-adapter`→`core-command-interface-adapter`、`JournalReader` 系の RMU への移動完了）。C5 に `Started` payload 拡張（`display`/`scan`）の追記と、実装済み yaml payload 行の更新。C6 の `journal_reader_impl.rs` パスを RMU クレートへ更新。§4 未解決項目のうち「`Started` の投影の厳密な行順」を「B8 で確定（16 行、`cli/intent-create/classic-scope` ゴールデンが正本）」として消し込み（`GateApproved` の phase 境界は分離して未解決のまま残置） |
| 6 | `.../inception/units-generation/unit-of-work.md` | U3 責務説明の `core-interface-adapter` を側分割の実態（`WorkflowExecutionRepositoryImpl` は `core-command-interface-adapter`、`JournalReaderImpl` は RMU）へ失効注記。U4 の `state_file_io` 転生・境界（入力＝ジャーナル読取 API）・実装ノートに、RMU が独立クレート `core-query-read-model-updater` として実装済みである旨と「embedded」表記の意味（デプロイ形態であってクレート独立性の否定ではない）を追記 |

**6 ファイル**（`git diff --stat` 実測: 66 insertions / 22 deletions）。

## 2. 検収 grep の実行結果

ブリーフ指定の acceptance grep をそのまま実行した:

```text
$ grep -rn "core-use-case\|core-interface-adapter\|infra-io" docs/specs/ | wc -l
4

$ grep -rnE "core-use-case|core-interface-adapter|infra-io" docs/specs/ | grep -vE "command|query|~~"
（0 件・exit 1）
```

4 件のヒットはすべて `docs/specs/00-policy.md`（D4・A8 行）と `docs/specs/01-domain-model.md`
（§7 原則 1・2、2 箇所）の `infra-io` で、いずれも `~~取り消し線~~` の中である（フィルタ後 0 件で
**PASS**）。`core-use-case` / `core-interface-adapter` は横断 sweep の結果 `docs/specs/` 配下からは
0 件（元々ヒットなし）。

固定トークン（BR/FR/C 番号・YAML キー・`READY` 等）は変更していない。`BR5.3` は差分の前後で
出現数 1 のまま（行の移動ではなく同一行への追記）、新規に引用した `FR1.1` は
`requirements.md:38` に実在する既存トークンの再利用である。

## 3. 迷った点

1. **`stages` の追加フィールド名を `stage_display` ではなく `display` とした**（`contract-summary.md`
   C5 payload 行・`10-orchestration.md`）。ブリーフ文面は「`StageEntry` への `StageDisplay`」としか
   書いておらず型名は明示されているがフィールド名までは書いていない。コードは読み取り専用の指示
   だが、正確性のため `modules/core/domain/src/orchestration/stage_entry.rs`（`display: StageDisplay`）
   と `workflow_execution_event.rs`（`Started.scan: WorkspaceScan`）を実測確認し、フィールド名は
   `display` / `scan`（`workspace_scan` ではない）と判明したため、そちらを採用した。型名
   （`StageDisplay` / `WorkspaceScan`）は括弧書きで併記し、読者が両方から追える形にした。
2. **`unit-of-work.md` の U3 定義文にある旧来の EventStore 独自スキーマ記述（journal/snapshot/
   checkpoint 3 表・`InMemoryWorkflowExecutionRepository`）はあえて手を付けなかった**。これは
   ADR-010（Bolt B6/B7、event-store-adapter-rs 乗り換え）由来のドリフトであり、B8 が生んだもの
   ではない。ブリーフの作業内容 5 は「U3 / U4 の記述にある**クレート名・「embedded」表記**」に
   スコープを絞っており、それ以外の EventStore 記述の全面是正は範囲外と判断した（見つけたことは
   ここに記録する — 後続の doc-sync か別 intent での是正候補）。
3. **`00-policy.md` の D4/A8 は「確定した決定」の記録セクションだが、`infra-io` 改名の失効注記を
   入れた**。これらは 2026-08-22 時点の決定を記録する historical セクションで書き換え対象か迷った
   が、ブリーフの sweep 対象 grep（`docs/specs/`）に無条件でヒットしており、所有ファイルの範囲内
   でもあるため、B6/B7 と同じ「打ち消し線 + 失効注記」で追記した（決定そのものは変えず、現行の
   クレート名を並記する形）。
4. **`contract-summary.md` C6 の `journal_reader_impl.rs` パス更新は、周囲の 2026-08-27（Bolt B6）
   ブロッククォート内への入れ子の打ち消し線になった**。B7 の doc-sync-report が同種の入れ子
   （`~~v2.0.0~~ → v3.0.0`）を許容している前例に倣い、同じ形式で追記した。

## 4. 引き継ぎ事項

- `contract-summary.md` §4 の未解決項目のうち `GateApproved` の phase 境界（PHASE_VERIFIED の要否）
  は今回のスコープ外のため未解決のまま残した。
- `unit-of-work.md` U3 の EventStore 独自スキーマ記述の全面是正（上記迷った点 2）は、範囲外と
  判断したため未着手。次回の doc-sync か別途の是正 Bolt で拾うことを推奨する。
