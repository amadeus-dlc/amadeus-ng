# jump redo の計画外カーソルに対する不整合の裁定

## 背景

foreign-scope jump slice の検証中に発見された既存の空隙（本 slice への変更には非依存。v2.8 の synced 導入以来）。

- Quint モデル `formal/orchestration/engine_loop.qnt` の `actJumpRedo` は、計画外（TaskUpdate 同期で実効 SKIP に立った）カーソルへの redo を許す。
- 集約の `jump_execute_guard` は、別 scope 無しの跳躍に自計画 EXECUTE を要求するため、同じ redo を拒否する。

モデルが許し実装が拒否する方向の差であり、実装の振る舞いを変えずに存在する。 upstream（本家 2.7.1）の該当観測は確かめていない。

## Q1: この不整合をどう解消するか

- A. モデルを実装へ揃える — `actJumpRedo` にガードを追加し、計画外カーソルへの redo をモデル側でも不達にする。実装は変えない。
- B. 実装をモデルへ揃える — 計画外カーソルへの redo を受理するよう `jump_execute_guard` を緩める。振る舞いの変更を伴う。
- C. 今回は扱わない — 既知の空隙として記録だけ残し、U2 の残りを優先する。
- X. Other (please specify)

[Answer]: A. モデルを実装へ揃える

利用者の回答: 構造化質問で「A. モデルを実装へ揃える」を明示選択（2026-09-10）。
