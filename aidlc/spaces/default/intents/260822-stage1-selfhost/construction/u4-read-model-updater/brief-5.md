# B8 委任ブリーフ 5 — 仕様同期 第 2 パス（是正後構造への差分同期）

Conversation language: 日本語
委任先モデル: Sonnet（境界明確な定型作業）
最終責任: Fable 5 メインセッション（全 diff レビュー・検収 grep 再実行・受入判定）

## 目的

第 1 パス（`e16e7f8`）後に確定した是正 — (1) ドメインは `modules/core/command/domain` =
`core-command-domain`（コマンド側の持ち物） (2) RMU は**中間**（どちらの側でもない。
`modules/core/read-model-updater` = `core-read-model-updater` へ改名） (3) クエリ側クレート
（将来）はドメインに絶対依存しない — を docs/specs と契約・Unit 記録へ反映する。
第 1 パスの成果は温存し、**差分だけ**を重ねる。

## 必読（この順で）

1. `coding-rules/cqrs-boundaries.md`（是正済みの正本 — 判定表と図式が最終形）
2. ADR-009 の「2026-08-29 改訂 2」（decisions.md — 誤導出の経緯込み）
3. `construction/u4-read-model-updater/crate-structure-proposal.md`（§1/§2 が最終形）
4. `developer-report-2.md`（改名の実施内容と 16 箇所コメント是正の語彙）
5. あなた自身の第 1 パス報告（`doc-sync-report.md`）— 触った箇所一覧が差分同期の起点

## 作業内容

1. **クレート名の横断 sweep**: docs/specs 全体 + 所有の intent 記録で
   `core-domain` → `core-command-domain`（パスは `modules/core/command/domain`）、
   `core-query-read-model-updater` → `core-read-model-updater` へ。第 1 パスで書いた追記
   ブロック内の名前も対象。B6/B7 と同じ失効注記様式（2026-08-29 / Bolt B8）だが、
   **第 1 パースで今日書いたばかりの追記ブロックは in-place 修正でよい**（同日内の是正）
2. **RMU の位置づけ**: 「クエリ側」と断ずる記述を「中間（コマンド側とクエリ側の両方に
   依存できる）」へ。クエリ側の定義は「将来のリードモデル読取・クエリ API 層 — ドメイン
   絶対依存禁止」
3. **`docs/specs/01-domain-model.md` §7 / `00-policy.md` A8** — 層構成の記述を最終形
   （command/{domain,use-case,interface-adapter} + read-model-updater + infrastructure +
   harness/infrastructure + shared）へ
4. **`docs/specs/11-workspace.md` §2.3** — 第 1 パスで書いた find_all_events 分割注記の
   クレート名を更新（分割の内容自体は不変）
5. **contract-summary C3/C5/C6・unit-of-work U3/U4** — 第 1 パス追記内のクレート名・
   位置づけを更新

## 所有ファイル（これ以外に書くな）

- `docs/specs/**`
- `$R/inception/contract-design/contract-summary.md`
- `$R/inception/units-generation/unit-of-work.md`
- 報告書: `$R/construction/u4-read-model-updater/doc-sync-report-2.md`

**禁止**: `modules/**`・`formal/**`・`Cargo.*`・coding-rules・memory・decisions.md・
crate-structure-proposal.md（是正済み）。push 禁止。`git add -A` 禁止。cargo 不要。

## 検収

- `grep -rnE "core-query-read-model-updater" docs/specs/ <所有 2 記録>` → 0 件（取り消し線内許容）
- `grep -rnE "\bcore-domain\b" docs/specs/ <所有 2 記録>` → 0 件（取り消し線内・
  「core-command-domain」への部分一致を除外して判定）
- RMU を「クエリ側」と現在形で断ずる記述 → 0 件
- 報告書: 変更ファイル一覧・各 1 行要点・検収 grep 実行結果・迷った点
- コミット 1 本「b8: 仕様同期 第 2 パス — ...」
