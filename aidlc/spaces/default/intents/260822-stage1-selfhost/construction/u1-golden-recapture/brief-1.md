# B10 委任ブリーフ 1 — U1 ゴールデン追加採取と投影の完成

Conversation language: 日本語
委任先モデル: Opus（upstream 実挙動の探索 + 逐語互換の完成 — 複雑）
最終責任: Fable 5 メインセッション（全 diff レビュー・検証の独立再実行・受入判定）

## 目的

B8 が「ゴールデン未採取」で明示エラー / 暫定に留めた 4 点の実バイトを、ピン留め upstream
（`3c3146cf`、v2.6.40）から採取し、RMU 投影を完成させる。**推測バイトの持込は引き続き禁止** —
採れないものは cases-missing.json 方式で理由を記録して残す。

## 必読

- `construction/u4-read-model-updater/developer-report-1.md` §8-1/8-2（欠落 4 点の正確な定義）
- `scripts/goldens/recapture-cli.sh` / `capture-cli.ts`（採取基盤 — ピン検証・正規化・ケース定義の流儀）
- `tests/golden/upstream-3c3146cf/README.md`（バイト不変の掟）と既存ケース群
- `tests/golden/upstream-3c3146cf/cli/set-autonomy/state-field-absent/case.json`
  （成功経路が無い理由の記録 — 状態ファイルに Construction Autonomy Mode 行が無い）
- coding-rules 全 17 規則（特に domain-services / no-backward-compatibility）

## 採取対象（4 点）

1. **状態ファイルの骨格**（9 セクション・31 フィールド行）: intent-create 直後の
   `aidlc-state.md` **全文バイト**を新アーティファクト（例: `state-full.md`）として記録する。
   upstream dist にテンプレートファイルが存在するならそれも併取（distributed 資産として
   README の表に追記）。骨格と同時に **`- **Stages to Execute**:` / `- **Stages to Skip**:` の
   畳み理由付き実バイト**（`2.1 (reverse-engineering — greenfield)` 形式）もこれで確定する
2. **非ゲート `StageCompleted` の単独経路**: initialization ステージを `report --result
   completed`（または upstream の実際の動詞）で完了させ、STAGE_COMPLETED 監査ブロックと
   state.diff を採る。既存 `cli/report` ケースが既にこれを含むなら、その旨を確認して流用
3. **`AUTONOMY_MODE_SET` 成功経路**: 状態ファイルに Construction Autonomy Mode 行を
   upstream 自身の手順で生えさせてから `set-autonomy --mode autonomous` を成功させ、
   監査ブロック（フィールドキーの実綴り）と state.diff を採る。**行を手で書き足すのは禁止** —
   upstream のどの動詞 / どの遷移がこの行を作るかを探索し、その手順ごと capture-cli.ts の
   ケースにする（探索の結果「ピンでは再現不能」なら cases-missing.json に理由を記録し、
   フィールドキー暫定のままである旨を doc に残す）
4. 上記の途中で見つかる他の未採取実バイトがあれば同様に採る（捏造しない、が唯一の掟）

## 実装（採取後）

- `ProjectionError::ScaffoldTemplateUnavailable` を撤去し、genesis の骨格生成を実バイトで実装
- `Stages to Execute` / `Stages to Skip` の投影を実装（PlanAction 2 値から導けない畳み理由は
  `Started` の解決済み計画（StageDisplay 等）に材料があるか確認し、**足りなければ止めて報告**
  — イベント拡張は裁定が要る）
- `StageCompleted` の `**Details**:` 暫定と `AUTONOMY_MODE_SET` の `**Mode**:` 暫定を
  実バイトへ置換（採れた場合）
- 新ゴールデンをテストに接続（両面バイト一致の流儀は B8 の 10 ケースに倣う）

## 所有ファイル・規律

`scripts/goldens/**` / `tests/golden/**` / `modules/**` / 報告書
`construction/u1-golden-recapture/developer-report-1.md`。docs・aidlc（報告書以外）・
formal・coding-rules 禁止。push 禁止。`git add -A` 禁止（明示パス）。検証は
`CARGO_TARGET_DIR=$PWD/target-delegate`。既存ゴールデンのバイトは 1 バイトも変更禁止
（追加のみ）。ピンは動かさない。committer は意味単位・日本語「b10: 」接頭辞。
**私は完了報告までコミットしない。**

## 受入基準

1〜7 は B8 と同一（fmt workspace + tools/lint / clippy / lint / test / quint-gate /
coverage 相対 / unwrap 0）。追加:
8. 新採取ゴールデンに provenance（case.json 相当）が揃い、`recapture-cli.sh` の再実行で
   `captured_at` 以外の差分が出ないこと（再現性）
9. `ScaffoldTemplateUnavailable` の grep 0 件（採取成功時）、または cases-missing.json に
   理由記録（不能時）
10. 投影ゴールデン検収が B8 の 10 ケース + 新ケースで全両面一致
