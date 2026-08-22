# code-generation — 観察日誌（Construction 3.5、unit-major）

> ステージ実行中の解釈・逸脱・トレードオフ・未確定事項を ISO 8601 タイムスタンプ付きで追記する。手編集はしない。

## Interpretations

- 2026-08-22T12:15:00Z — Plan Approval に先立ち、計画の形を左右する 2 点（Bolt ブランチと aidlc 記録のコミット方法 / ゴールデン配置）を質問票の Q1・Q2 として先に人間へ問う; project.md の是正学習「上流成果物間の矛盾は人間へ裁定」（C7 の `tests/goldens/` と既存 `tests/golden/upstream-3c3146cf/` の並立）に従う
- 2026-08-22T12:15:00Z — 本ハーネス（2.6.54）の CLI には Plan Approval 指紋の機械ガードが無い（`grep fingerprint .claude/tools` は review 指紋のみ）; ステージ定義どおり `[Approval Fingerprint]` と `Approve Plan` の儀式は守り、非ゲート質問として `aidlc-log.ts decision/answer` で記録する

## Deviations

- 2026-08-22T12:40:00Z — 委任ブリーフはルール束・計画・テスト手順を逐語で連結した記録内ファイル `developer-brief-1.md`（86KB）にまとめ、委任プロンプトでは先頭 2 行のマーカーと「全文 Read」指示 + 要点再掲にとどめた; ステージ定義の「ルール束を逐語で貼る」をファイル経由で満たす（トークン節約、Fable 5 委任方針）
- 2026-08-22T12:40:00Z — Bolt B1 は worktree を使わず `main-sync` から `bolt/b1-u1-canon-json-goldens` を切った（Q1 = A、PR 直列・aidlc 記録を先頭コミットに同乗）; `aidlc-bolt.ts start --name B1 --batch 1`（worktree なし）で BOLT_STARTED を記録

## Tradeoffs

- 2026-08-22T12:40:00Z — 委任を 2 回（Step 1〜16 / Step 17〜19）に分け直列化; 1 回にまとめると文脈が長くゴールデン採取の品質が落ちるため。同一承認・同一指紋の下で行う

## Open questions

- 2026-08-22T12:15:00Z — upstream ピン `3c3146cf` の dist ツールは raw.githubusercontent.com から取得可能（HTTP 200 実測）; 採取スクリプトはネットワーク前提になる（A3 はオーナー環境での実行を許容）
