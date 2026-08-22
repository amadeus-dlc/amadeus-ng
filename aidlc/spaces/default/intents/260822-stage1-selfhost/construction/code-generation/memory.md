# code-generation — 観察日誌（Construction 3.5、unit-major）

> ステージ実行中の解釈・逸脱・トレードオフ・未確定事項を ISO 8601 タイムスタンプ付きで追記する。手編集はしない。

## Interpretations

- 2026-08-22T12:15:00Z — Plan Approval に先立ち、計画の形を左右する 2 点（Bolt ブランチと aidlc 記録のコミット方法 / ゴールデン配置）を質問票の Q1・Q2 として先に人間へ問う; project.md の是正学習「上流成果物間の矛盾は人間へ裁定」（C7 の `tests/goldens/` と既存 `tests/golden/upstream-3c3146cf/` の並立）に従う
- 2026-08-22T12:15:00Z — 本ハーネス（2.6.54）の CLI には Plan Approval 指紋の機械ガードが無い（`grep fingerprint .claude/tools` は review 指紋のみ）; ステージ定義どおり `[Approval Fingerprint]` と `Approve Plan` の儀式は守り、非ゲート質問として `aidlc-log.ts decision/answer` で記録する

## Deviations

## Tradeoffs

## Open questions

- 2026-08-22T12:15:00Z — upstream ピン `3c3146cf` の dist ツールは raw.githubusercontent.com から取得可能（HTTP 200 実測）; 採取スクリプトはネットワーク前提になる（A3 はオーナー環境での実行を許容）
