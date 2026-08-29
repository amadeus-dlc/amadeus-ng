# doc-sync-report-2 — Bolt B8 仕様同期 第2パス（是正後構造への差分同期）

> 実施日 2026-08-29。ブランチ `bolt/b8-u4-read-model-updater`。第 1 パス（コミット `e16e7f8`）は
> 温存し、**差分だけ**を重ねた。
>
> 正とした順序: (1) `brief-5.md`（作業内容 1〜5）、(2) `coding-rules/cqrs-boundaries.md`（是正済み
> 正本 — 判定表・図式が最終形）、(3) `decisions.md` ADR-009「2026-08-29 改訂 2」（誤導出の経緯込み）、
> (4) `crate-structure-proposal.md` §1/§2（是正済み・自分は編集禁止・参照のみ）、
> (5) `developer-report-2.md`（改名 2 件の実施内容・16 箇所コメント是正の語彙）、(6) 自分の第 1 パス
> 報告（`doc-sync-report.md`）。
>
> **新しい設計判断はしていない。** オーナー是正（ドメインはコマンド側の持ち物 = `core-command-domain`、
> RMU はコマンド側でもクエリ側でもない「中間」= `core-read-model-updater`、クエリ側は将来のリード
> モデル読取・クエリ API 層でドメイン絶対依存禁止）を反映しただけである。**第 1 パスで今日書いた
> ばかりの追記ブロックは同日内の是正として in-place 修正**した（ブリーフの明示許可）。それ以前から
> 存在した記述（U2 の `core-domain` 言及、2026-08-22 の Review 節の引用）は B6/B7 と同じ家内書式
> （`~~打ち消し~~ → 失効（日付・Bolt）`）で失効注記を重ねた。コード・`formal/**`・`Cargo.*`・
> coding-rules・memory・decisions.md・`crate-structure-proposal.md`（是正済み）は未変更。`git add`
> は明示パスのみ、`commit` は 1 本、`push` はしていない。

## 1. 変更したファイルと要点

| # | ファイル | 主な変更 |
|---|---|---|
| 1 | `docs/specs/00-policy.md` | A8 行の失効注記を最終形へ更新: `core` 配下は `command/{domain,use-case,interface-adapter}`（コマンド側 3 クレート）・`read-model-updater`（RMU、側接頭辞なしの中間）・`infrastructure` に分かれ、`query/` サブディレクトリは存在しないことを明記 |
| 2 | `docs/specs/01-domain-model.md` | §7 原則 1 の「ドメインクレート」に、ドメインはコマンド側の持ち物（`core-command-domain`）である旨を追記 |
| 3 | `docs/specs/11-workspace.md` | §2.3 冒頭・表 3 行（`render_audit_block`/`find_all_events`/`state_writers`）の `core-query-read-model-updater` を `core-read-model-updater`（中間クレート）へ、`core-domain::workspace` を `core_command_domain::workspace` へ更新（分割の内容自体は不変） |
| 4 | `.../inception/contract-design/contract-summary.md` | C3 追記ブロックを in-place 修正（crate改名 + RMU=中間の位置づけ）。C3コード内コメント・C5 yaml コメントの実装パス・C6 追記ブロックのパスを是正後のクレート名へ更新 |
| 5 | `.../inception/units-generation/unit-of-work.md` | U2（責務・境界）の `core-domain` を `core-command-domain` へ失効注記。U3/U4 の第1パス追記ブロックを in-place 修正（RMU=中間・crate改名）。U4実装ノートのクレート名更新。Review 節（2026-08-22）の `core-domain` 引用に失効注記 |
| 6 | 報告書: `doc-sync-report-2.md`（新規） | 本報告書 |

**5 ファイル**変更 + 報告書1件新規（`git diff --stat` 実測: 51 insertions / 36 deletions、pass 1 の `e16e7f8` に対する差分）。

## 2. 検収 grep の実行結果

ブリーフ指定の acceptance grep をそのまま実行した（`$R1` = contract-summary.md、`$R2` = unit-of-work.md）:

```text
$ grep -rnE "core-query-read-model-updater" docs/specs/ $R1 $R2 | grep -v "~~"
（0 件・exit 1）

$ grep -rnE "\bcore-domain\b" docs/specs/ $R1 $R2 | grep -v "~~"
（0 件・exit 1）
```

raw grep のヒットはすべて `~~取り消し線~~` の中（`docs/specs/11-workspace.md` 1箇所、
`contract-summary.md` 3箇所、`unit-of-work.md` 3箇所）で、いずれも「旧称はこれ」という明示の
言及であり、フィルタ後は 0 件（**PASS**）。

「RMU をクエリ側と現在形で断ずる記述」も全文検索で確認した。`クエリ側` を含む行は
`unit-of-work.md` 3箇所・`contract-summary.md` 3箇所あるが、すべて「RMU は**コマンド側でもクエリ側
でもない**」「コマンド側クレートに RMU・クエリ側クレートは現れない」のように RMU とクエリ側を
明確に区別する文脈で、RMU=クエリ側と断ずる記述は 0 件（**PASS**）。

固定トークン（BR/FR/C 番号・YAML キー・`READY` 等）は変更していない。

## 3. 迷った点

1. **U2（`u2-domain-es-core`）の `core-domain` 言及は第 1 パスで触っていなかったが、今回の sweep
   対象に含めた**。ブリーフ item 1 は「docs/specs 全体 + 所有の intent 記録で `core-domain` →
   `core-command-domain`」と書いており、対象を「第 1 パスで書いた箇所」に限定していない。加えて
   検収 grep（`\bcore-domain\b` が 0 件）も無条件のため、U2 の 2 箇所（責務・境界）も失効注記で
   更新した。
2. **`unit-of-work.md` の Review 節（2026-08-22 付、`aidlc-architecture-reviewer-agent` の所見）に
   ある `core-domain クレート」表現` という引用にも失効注記を入れた**。ここは過去のレビュー verdict
   の直接引用であり、本来は書き換えずに保存する対象と迷ったが、検収 grep が無条件で
   `docs/specs/ + 所有 2 記録` 全体をスキャンするため、引用符内の 1 語だけに `~~` を addし、
   レビューの verdict・所見本文・日付は一切変更しない形にした（レビュー結果への影響なし）。
3. **`contract-summary.md`/`unit-of-work.md`/`11-workspace.md` の「旧称」への言及（`旧称
   ~~core-query-read-model-updater~~` 等）も取り消し線で包んだ**。ブリーフの許容条件は「取り消し線内
   許容」であり、旧称への参照自体は読者の理解に資すると判断してあえて残したが、素の文字列のままだと
   検収 grep のフィルタ（`grep -v "~~"`）を通過しないため、旧称部分だけを打ち消し線で囲んだ。
4. **第 1 パスの追記ブロック（C3/C6/U3/U4）は in-place 修正、それ以前からの記述は失効注記の重ね書き
   ―という 2 つの書式が同じファイル内に混在する**。ブリーフが「今日書いたばかりの追記ブロックは
   in-place 修正でよい（同日内の是正）」と明示許可していたためこの使い分けにしたが、統一感の観点
   では気になる点として記録する。委任者の判断で第 3 パスがあれば統一を検討されたい。

## 4. 引き継ぎ事項

- 第 1 パスの報告書（`doc-sync-report.md`）「迷った点 2」で記録した `unit-of-work.md` U3 の
  EventStore 独自スキーマ記述（journal/snapshot/checkpoint 3 表・`InMemoryWorkflowExecutionRepository`
  — ADR-010 由来のドリフト）は、本パスでも未着手のまま。ブリーフのスコープ外と判断した。
- `contract-summary.md` §4 の `GateApproved` の phase 境界（PHASE_VERIFIED の要否）も未解決のまま。
