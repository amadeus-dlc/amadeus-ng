# 別 scope を名指した直接 execute (jump) の検証

裁定 jump-contract Q1 = A の foreign-scope 部分 — 本家 2.7.1 (固定コミット
`a277af218f0df7f325d3b8be7b6d90fce2c5bd40`、`core/tools/aidlc-jump.ts` 257–296, 308–375)
の直接 execute 契約どおり、`--scope classic` 等で名指された別 scope の静的な列で
到達可否と読み飛ばしを導く slice の検証記録である。

## 実装した変更

- 集約 `IntentExecution` に `cursor_foreign_scoped: bool` を追加
  (`modules/core/command/domain/src/orchestration/intent_execution.rs`)。
  「`--scope` を名指した直接 execute (`Jumped` が `JumpScope` を運ぶ) が、自 scope の
  実効計画では EXECUTE でない位置へカーソルを置いた」事実を保持する。
  `cursor_synchronized` (TaskUpdate **同期**由来) とは由来が違うため別フィールドとし、
  混同はフィールド doc で禁止を明記した。
  - `apply_jump`: `self.cursor = target;` の直後で
    `jumped.scope().is_some() && !self.in_scope(target)` を立てる。
  - `advance_from`: カーソルが次の in-scope へ進む枝で偽へ戻す。
  - `mutate` の `TaskSynchronized` 腕: カーソルは同期が置き直すので偽へ戻す。
  - `check_invariants` の `cursor_in_scope`:
    `accepts_commands() && !cursor_synchronized && !cursor_foreign_scoped && !in_scope(cursor)`。
  - genesis (`From<(Started, occurred_at)>`) は false。アクセサ
    `pub const fn cursor_foreign_scoped()` を追加。
- コマンド側スナップショット DTO
  (`modules/core/command/interface-adapter/src/orchestration/dto/intent_execution_dto.rs`):
  `cursor_synchronized` の隣に `#[serde(default)]` 付き `cursor_foreign_scoped` を追加し、
  encode (`of`) と decode (`to_domain` → `IntentExecution::new`) を結線。欄が無い行は
  「そのような跳躍をしていない」という正規の意味で読む (後方互換の緩和ではない)。
- DTO テスト
  (`modules/core/command/interface-adapter/src/orchestration/dto/tests.rs`):
  `GENESIS_SNAPSHOT` に `"cursor_foreign_scoped":false` を追加し、欄を落としても
  false で読める serde(default) テスト
  (`a_snapshot_row_without_the_cursor_foreign_scoped_field_reads_as_not_foreign_scoped`)
  を追加。
- RMU 投影 (`modules/core/read-model-updater/src/workspace/projection.rs`):
  `Jumped` の読み飛ばし・巻き戻し導出が自 scope の実効計画 (`effective_action`) だけを
  見ていたので、`jumped.scope()` が在るときは別 scope の静的な列 (`JumpScope::contains`)
  で membership を判定する `in_jump_plan` へ一本化した (集約の `jump_plan` と同じ規則)。
  借用衝突のため後方ジャンプ腕は「対象の収集 → 書き換え」の 2 段に組み替えた (導出内容は
  不変)。
- Quint モデル `formal/orchestration/engine_loop.qnt` (v2.9、ヘッダに変更記録を追記):
  - immutable な `var foreignPlan: int -> PlanAction` を init で nondet に選ぶ
    (stage 0 は Execute 固定 — `plan` と同じ規律)。全アクションで `foreignPlan' = foreignPlan`。
  - 新 var `foreign: bool`。`synced' = false` の箇所には `foreign' = false`、
    `synced' = synced` の箇所には `foreign' = foreign` を全腕に配り、`actTaskSync` は
    `foreign' = false`。
  - 新アクション `actForeignJumpForward`: `actJumpForward` と同じ枠組みで、到達可否と
    介在の読み飛ばしの membership を `inScope` から `foreignPlan.get(u) == Execute` へ
    置き換えたもの。`foreign' = not(inScope(t))`、`synced' = false`、レビュー試行の
    全リセットと `affirmed' = false` を含む。`lastAction' = "foreign_jump"`。
  - `cursor_in_scope` を
    `(status == Running and not(synced) and not(foreign)) implies (effectivePlan(cursor) == Execute)` へ。
  - `review_attempt_floor` / `practices_receipt_floor` のジャンプ列挙へ `foreign_jump` を
    追加 (同じ全リセットを行うため。名前は不変)。
  - witness `val w_foreign_jump = foreign` を追加。
- `scripts/quint-gate.sh`: ENGINE_LOOP の witness 一覧へ `w_foreign_jump` を追加
  (不変条件一覧は名前が不変なので変更なし — 確認済み)。
- ITF 準拠テスト
  (`modules/core/command/domain/tests/engine_loop_conformance.rs`):
  `ModelState` に `foreign: bool` (欄が無い旧 trace は false で読む) と
  `foreign_plan: Vec<PlanAction>` (欄が無い旧 trace は自計画の写しで読む) を追加し、
  `assert_projection` で `agg.cursor_foreign_scoped()` と突き合わせる。replay の match に
  `"foreign_jump"` 腕を追加 (trace の foreign plan から EXECUTE の slug 集合を集めて
  `JumpScope::new("foreign", StageSlugSet::new(...))` を組み、`JumpDirection::Forward` で
  `agg.jump(...)` を呼ぶ)。アクション網羅リストに `"foreign_jump"` を追加。
  合成計画 (自 scope) の組み直しは既存経路のまま変えていない。
- ITF フィクスチャ `tests/conformance/fixtures/engine_loop/trace-0xd0d.itf.json` を採取
  (下記「採取した witness / trace」参照)。

## Red (修正前の再現)

いずれも修正前の作業木で実行し、指示どおりの失敗を確認した。

1. `cargo test -p core-command-domain --lib a_foreign_scope_execute_uses_that_scopes_plan_for_reachability_and_skips`
   → FAILED。
   `apply_event: invariant violated — cursor_in_scope`
   (`intent_execution.rs:2406` で panic)。別 scope classic では EXECUTE の到達点が
   自 scope の実効計画では SKIP のため、不変条件が跳躍後の状態を拒否していた。
2. `cargo test -p aidlc --test jump_contract a_foreign_scope_execute_derives_skips_from_that_scopes_static_plan`
   → FAILED。子プロセス (`aidlc-jump`) の stderr に同じ
   `apply_event: invariant violated — cursor_in_scope` が出て exit 101。

## Green (修正後)

| 検査 | コマンド | 結果 |
|---|---|---|
| 対象 1 | `cargo test -p core-command-domain --lib a_foreign_scope_execute_uses_that_scopes_plan_for_reachability_and_skips` | ok (1 passed) |
| 対象 2 | `cargo test -p aidlc --test jump_contract` | ok (10 passed) — Red 時は `a_foreign_scope_execute_derives_skips_from_that_scopes_static_plan` のみ失敗、他 9 件は通過していた |
| 集約全体 | `cargo test -p core-command-domain --lib` | ok (721 passed) |
| ITF 準拠 | `cargo test -p core-command-domain --test engine_loop_conformance` | ok (1 passed、16 fixture) |
| RMU | `cargo test -p core-read-model-updater` | ok (全スイート緑) |
| 整形 | `cargo fmt --all --check` | exit 0 |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 |
| 独自 lint | `cargo lint` | exit 0 |
| Quint ゲート | `bash scripts/quint-gate.sh` | 全 28 ステップ PASS (witness `w_foreign_jump` を含む) |

## 採取した witness / trace

- witness 反転判定: `quint run formal/orchestration/engine_loop.qnt --seed 0x303
  --max-samples 2000 --max-steps 40 --invariant "not(w_foreign_jump)"` →
  `[violation]` (= `foreign` が立つ経路が実在)。quint-gate でも同じ判定で PASS。
- ITF trace 採取: seed `0xd0d` (既存 fixture の seed 0x101/0x202/0x303/0x404/0x505/
  0x606/0x707/0x808/0x909/0xa1/0xb2/0xc3/0xd4/0xe5/0xf6 と重複しない hex)。

  ```sh
  quint run formal/orchestration/engine_loop.qnt --seed 0xd0d \
    --max-samples 2000 --max-steps 40 --invariant "not(w_foreign_jump)" \
    --out-itf target/tmp/cand.itf.json
  ```

  出力の `#meta.source` を `formal/orchestration/engine_loop.qnt` に正規化して
  `tests/conformance/fixtures/engine_loop/trace-0xd0d.itf.json` へ配置した。
  遷移列は `init → report_stale → set_autonomy → task_sync → foreign_jump` で、
  最終状態は cursor 3 (自実効計画では SkipPlan、foreignPlan では Execute) に
  `foreign = true` が立つ。

## 採取時に見つかった既存の空隙 (本 slice では直さず報告)

seed `0xa0a` で最初に採取した trace は
`… task_sync (cursor 3、実効 SKIP → synced) → report_rejected → report_revised →
jump_redo …` を含み、replay が `jump_redo` 腕で `InvalidTarget(StageIndex(3))` に
なった。モデルの `actJumpRedo` は `status == Running, cursor != 0` だけを要求し、
同期で計画外に立ったカーソルへの redo を許すが、集約の `jump_execute_guard` は
別 scope 無しの跳躍に自計画の EXECUTE を要求するため拒否する。これは v2.8
(TaskUpdate 同期) で `synced` が入ってから存在する空隙で、本 slice の変更に依らない
(trace-0x909 はたまたまこの組合せを踏んでいない)。別 scope 扱いの jump 契約とは
独立の話題なので、本 slice ではモデル・実装のどちらも変えず、採取 seed を 0xd0d に
変えて回避した。モデルを実装へ揃える (`actJumpRedo` に `not(synced) or inScope(cursor)`
相当のガード) かどうかは裁定事項として残す。

## 残課題

- `cargo test --workspace --no-fail-fast`: exit 0 (94 スイートすべて `test result: ok`、
  FAILED なし)。他担当起因の先行失敗は無い。
- 上記「採取時に見つかった既存の空隙」(同期済み計画外カーソルでの redo) は裁定待ち。
