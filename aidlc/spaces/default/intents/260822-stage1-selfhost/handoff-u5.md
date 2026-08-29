# ハンドオフ — B10 完了・park（2026-08-29）

次セッションの再開: `/aidlc --resume aidlc/spaces/default/intents/260822-stage1-selfhost/handoff-u5.md を読んでから進めてください。`

## 現在地

- main = `f7ef90f`（PR #35 / Bolt B10 まで着地）。直近の着地: B6 `45c323c`（esa v2）→ B7 `ef8b305`（esa v3 = EventEnvelope）→ 収束ルール `8f9b99f` → B8 `2b895c8`（CQRS 側分割 + U4 RMU）→ B9 `99d73c2`（shared 全解体）→ B10 `f7ef90f`（ゴールデン 28 ケース + 投影完成）
- 形式ワークフローは `functional-design` で park 済み（B6〜B10 は AIDLC-off の委任 Bolt 運用 — オーナー裁定「AIDLC より私の指示が強い」に基づく。**未通過ゲートを通過扱いにしない**記録方針は維持）
- クレート構成（最終形）: `core-command-{domain,use-case,interface-adapter}` / `core-read-model-updater`（RMU = 中間、両側依存可）/ `core-infrastructure`（言語拡張 + canon_json）/ `harness-{claude,infrastructure}` / `app/aidlc`。**modules/shared は消滅**。domain のリポジトリ内依存ゼロ

## 次の作業（オーナー未選択 — 再開時に確認）

1. **（推奨）U5: report ユースケース**（Bolt 計画 B7 相当）— 再水和 → decide → store → 投影キャッチアップの定型確立。`StoreVersion` newtype 化の申し送り回収（B7 developer-report-2 §4-b）
2. U6: next・continue（21 分岐ラダー）— U5 の定型後
3. 小粒整備: 規則監査残り（Major 4 / Minor 5 — CONSISTENCY-AUDIT-2026-08-24.md）、機械化ロードマップ（struct-literal-once lint 等）、U2 積み残し（primary constructor 化・`with_*` 改名・state_writers の型収容）

## 申し送り（既知の未確定・将来タスク）

- **U7 が骨格を書く**（裁定 A / ADR-009 改訂 4）: 正本 = `tests/golden/upstream-3c3146cf/cli/intent-create/classic-scope/state-full.md`（102 行）。RMU は差分適用のみ（`ScaffoldMissing`）
- `AUTONOMY_MODE_SET` 成功経路はピン到達不能（cases-missing.json に証明）。`**Mode**:` は暫定
- recompose の `- **Completed**:` 書換の要否は実バイトで判別不能のまま（B10 report §7-3）。`jump/execute-forward-to-conditional` の投影検収未接続（同 §7-4）
- RMU 内部 `state_writers` の自由関数群 → 型収容の同族是正（B9 申し送り）
- U1 ピン更新・macOS CI・main push トリガーは後続 intent（team.md スコープ注記）

## 効いている運用規約（このセッションで確立）

- **収束ルール**（project.md Corrections 登録済み）: 毎 push = 常設監視 + unresolved×non-outdated 全数 sweep + 実否検証 + 返信→resolve + 「CI green ∧ unresolved 0 ∧ 全返信 ∧ bot レビュー完了」を最新 head 再実測 → merge queue
- **AI 裁定マージ権限**（同登録済み）: 収束条件を満たせば人間の個別承認なしで投入可（オーナー包括承認 2026-08-29）
- 委任の規律: ブリーフに固定裁定・所有ファイル・受入基準・`git add -A` 禁止・`CARGO_TARGET_DIR=$PWD/target-delegate`・push 禁止。**委任報告は鵜呑みにせず全ゲートを独立再実行**（B10 で provenance 陳腐化を検出した実績）。委任中は委任者のコミット凍結（`git mv` 巻き込み事故 2 回の教訓）
- 規則正本は 18 本（coding-rules/README 索引）。直近追加: infrastructure-layer（言語拡張のみ・RPC/DB 禁止）、domain-services（最後の手段 — 型の関連メソッドへ収める / OOUI）

## 本家 event-store-adapter-rs

- `=3.0.0` ピン（Conformist・腐敗防止層なし）。**本家リポジトリへの接触は禁止**（オーナー明言「本家に勝手に報告するな」）
