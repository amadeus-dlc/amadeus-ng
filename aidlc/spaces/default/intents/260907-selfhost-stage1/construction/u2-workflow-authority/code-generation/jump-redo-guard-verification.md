# 計画外カーソルへの redo に対するモデル側ガードの検証

裁定 [jump-redo-guard-gap-questions.md](jump-redo-guard-gap-questions.md) Q1 = A
（モデルを実装へ揃える）の実装記録である。実装の振る舞いは変えていない。

## 背景（差の所在）

- Quint モデル `formal/orchestration/engine_loop.qnt` の `actJumpRedo` は
  `status == Running` と `cursor != 0` だけを要求していた。そのため TaskUpdate 同期
  （`synced`）や別 scope の跳躍（`foreign`）が実効 SKIP へ置いたカーソルに対する redo を
  許していた。
- 集約 `IntentExecution::jump_execute_guard`
  （`modules/core/command/domain/src/orchestration/intent_execution.rs:1759-1775`）は、
  別 scope を名指さない跳躍に `jump_plan(None)` = 自実効計画の EXECUTE 位置集合を要求し、
  外れた位置は `CommandError::InvalidTarget` を返す。
- モデルが許し実装が拒否する向きの差である。upstream（本家 2.7.1）の該当観測は確かめて
  いないため、裁定どおり実装を正としてモデルを狭めた。

## 実装した変更

- `formal/orchestration/engine_loop.qnt`
  - `actJumpRedo` に `inScope(cursor)` を追加（`inScope(s)` は
    `effectivePlan(s) == Execute`。集約の `jump_plan(None)` と同じ規則）。
  - 不変条件 `jump_redo_on_plan` を追加:
    `(lastAction == "jump_redo") implies (effectivePlan(cursor) == Execute)`。
  - 到達性 witness `w_jump_redo` を追加（`lastAction == "jump_redo"`）。ガードが redo
    経路そのものを死なせていないことを固定する。
  - ヘッダへ v2.10 の変更記録を追記。
- `scripts/quint-gate.sh`
  - engine_loop の不変条件一覧へ `jump_redo_on_plan` を追加。
  - witness 一覧へ `w_jump_redo` を追加。

Rust 側（集約・DTO・投影・ITF 準拠テスト）は変更していない。差はモデル側にしか無い。

## Red（ガード追加前）

不変条件 `jump_redo_on_plan` だけを先に足し、`actJumpRedo` は未修正の状態で実行した。

```sh
quint run formal/orchestration/engine_loop.qnt --seed 0x1a2b3c \
  --max-samples 2000 --max-steps 40 --invariant jump_redo_on_plan
```

結果は `[violation] Found an issue`。反例の最終状態は `cursor: 3`、
`overlay: 3 -> SkipPlan`（= 実効 SKIP）、`synced: true` であり、
「同期で計画外に立ったカーソルへの redo」という報告どおりの経路だった。

## Green（ガード追加後）

| 検査 | コマンド | 結果 |
|---|---|---|
| 型検査 | `quint typecheck formal/orchestration/engine_loop.qnt` | exit 0 |
| 不変条件 19 本（新条件を含む） | `quint run … --seed 0x1a2b3c --max-samples 2000 --max-steps 40 --invariants no_run_stage_for_skip cursor_in_scope jump_redo_on_plan …` | `[ok] No violation found` |
| Quint ゲート全体 | `bash scripts/quint-gate.sh` | 全 29 ステップ PASS（`w_jump_redo` を含む） |

`w_jump_redo` が PASS した（負形式の判定で violation = 経路実在）ことにより、
ガード追加後も redo は到達可能である。

## 既存 ITF フィクスチャとの整合

`tests/conformance/fixtures/engine_loop/` のうち `jump_redo` を含む trace は 6 本
（`trace-0x101` / `0x808` / `0xb2` / `0xc3` / `0xa1` / `0xd4`）である。これらは旧モデル
（ガード追加前）で採取されたが、いずれも集約が受理する位置での redo だけを含む。根拠は、
同じ作業木で実施した `cargo test --workspace --no-fail-fast` が **exit 0**（`engine_loop_conformance`
を含む）だったことである。集約は以前からガードを持っていたので、計画外の redo を含む trace が
あれば replay が `InvalidTarget` で失敗していた。

したがって今回のモデル側の絞り込みは、記録済みの trace を 1 本も無効にしていない。

## 残課題

- upstream 2.7.1 の該当観測（計画外カーソルへの redo を本家がどう扱うか）は未確認のまま
  である。裁定 A は「実装を正とする」判断であって、本家観測との突き合わせではない。
