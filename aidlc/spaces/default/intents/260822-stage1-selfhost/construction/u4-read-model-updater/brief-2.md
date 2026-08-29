# B8 委任ブリーフ 2 — 側分割 + U4 の仕様同期（doc-sync）

Conversation language: 日本語
委任先モデル: Sonnet（境界明確な定型作業）
最終責任: Fable 5 メインセッション（全 diff レビュー・検収 grep 再実行・受入判定）

## 目的

B8 のコード変更（CQRS 側分割 + infrastructure 層 + U4 RMU + 裁定 A のイベント拡張）で生じた
docs/specs・contract の語彙ドリフトを解消する。コードは読み取り専用。

## 必読（この順で）

1. `construction/u4-read-model-updater/developer-report-1.md` — §1 クレート対応表、
   §8 ドリフト一覧（あなたの作業対象は 8-3 表の #6 / #7 / #10 と、下記の横断 sweep）
2. `construction/u4-read-model-updater/crate-structure-proposal.md` — 構成の正本
3. B6/B7 の doc-sync-report（同期スタイルの前例 — 日付付き失効注記
   「~~旧~~ → **失効（2026-08-29 / Bolt B8）**: 新」）

## 作業内容

1. **旧クレート名の横断 sweep**: `grep -rn "core-use-case\|core-interface-adapter\|infra-io"
   docs/specs/` のヒットを全処理（新名: `core-command-use-case` /
   `core-command-interface-adapter`（コマンド側）、`core-query-read-model-updater`（クエリ側 —
   旧 interface-adapter のうち JournalReaderImpl と state_file_io の行き先）、
   `core-infrastructure`。**どちらの側へ行ったかを文脈で正しく判定**すること — 機械置換禁止）
2. **`docs/specs/11-workspace.md`** — §2.3 の `find_all_events`「domain に残す」へ分割注記
   （順序付けの純関数 = domain / シャード列挙とファイル読取 I/O = 投影側 — 報告書 §7-7）。
   §2.3 の `state_writers` / `render_audit_block` 転居は「実施済み（B8）」へ。層の一覧・
   クレート名も更新
3. **`docs/specs/01-domain-model.md` / `10-orchestration.md` / `deviations.md` ほか** —
   クレート名 sweep の一環 + `StageEntry` への `StageDisplay`（番号・表題・担当）と `Started` の
   `WorkspaceScan` 追加（裁定 A・ADR-008 追記 2026-08-29 参照）を、集約・イベント語彙に触れる
   記述へ反映
4. **`inception/contract-design/contract-summary.md` C5** — `Started` payload 拡張
   （`StageDisplay` / `WorkspaceScan`）の追記、§4 未解決項目のうち「`Started` の投影の厳密な
   行順」を「B8 で確定（16 行、`cli/intent-create` ゴールデンが正本）」として消し込み。
   C3 のクレート名（core-use-case 所有 → core-command-use-case、JournalReader 系 → RMU 所有）
   追記。C6 の実装クレート名更新
5. **`inception/units-generation/unit-of-work.md`** — U3 / U4 の記述にあるクレート名・
   「embedded」表記を実態（U4 = `core-query-read-model-updater` 独立クレート、実装済み）へ
   失効注記で追随

## 所有ファイル（これ以外に書くな）

- `docs/specs/**`
- `$R/inception/contract-design/contract-summary.md`（$R = intent record）
- `$R/inception/units-generation/unit-of-work.md`
- 報告書: `$R/construction/u4-read-model-updater/doc-sync-report.md`

**禁止**: `modules/**`・`formal/**`・`Cargo.*`・coding-rules・memory・decisions.md（ADR は
委任者が追記済み）への変更。push 禁止。**`git add -A` 禁止**（明示パスで add）。cargo 実行不要。

## 検収

- `grep -rn "core-use-case\|core-interface-adapter\|infra-io" docs/specs/` が 0 件
  （`~~...~~` 取り消し線内と「core-command-use-case」等の新名への部分一致は許容 —
  判定には `grep -rnE "core-use-case|core-interface-adapter|infra-io" docs/specs/ |
  grep -vE "command|query|~~"` を使う）
- 固定トークン（BR/FR/C 番号・YAML キー・`READY` 等）は英語のまま
- 報告書: 変更ファイル一覧・各 1 行要点・検収 grep 実行結果・迷った点
- コミット 1 本「b8: 仕様同期 — ...」
