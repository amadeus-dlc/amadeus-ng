# ハンドオフ — b43（Bolt 3 前半）: クエリ側を「DAO 1 表 1 引当 → ユースケースが FK をたどって View を組む」へ（2026-09-03）

設計書: [`b43-query-dao-lookup/design.md`](b43-query-dao-lookup/design.md)（§0-2 / §0-5 / §2 / §3 に「改訂 2026-09-03（裁定 — JOIN 解体）」）。裁定: オーナー 2026-09-03（`project.md` Corrections 最終行 / `cqrs-boundaries.md` 規則 6「表の形と読み方」— 主キー 1 列 `id`・UNIQUE・FK、**DAO は 1 表 1 引当、JOIN しない**）。独立再検討: Fable 5（`b43-fable-review.md`、統合側の控え — JOIN 9 + EXISTS 1 の全違反判定、`next_answer` の実害、是正設計 B.1〜B.5、lint 着手条件の充足）。

## やったこと
- **テーブル**（`49fece23`）: `read_*` 17 表を単一主キー `id` + 自然キー UNIQUE 索引 + FK 列へ（`row_id.rs` で決定的な代理キー）。
- **DAO 12 実装 = 12 表、各 1 `FROM`**。JOIN 9 + EXISTS 副問合せ 1 を全解体。`ContinuationDao` / `SteeringFirstPartView` / 両 in-memory を撤去、`SteeringPlanDao` / `SteeringPartDao` / `JumpPhaseDao` を port / impl / in-memory の 3 組で新設。`RunStageDao` に `find_by_id` / `find_bound`（自然キー 3 列 + 束縛 2 列 — `read_run_stage_digests` が非 UNIQUE のため）、`ExecutionDao` に `find_by_state_binding`、`JumpDao::find_phase` → `find_by_target`。
- **View は行の写し**（`id` を運ぶのは他の行の FK が指す先のときだけ、FK 列を運ぶのはユースケースが次の鍵にするときだけ — `read_view/mod.rs`）。組み立て View `NextTurnView` / `ContinuationView` はユースケース側 `orchestration/`（DAO の戻り値ではないので `port/` ではない）。
- **ユースケースが FK をたどる**: `next` は最大 5 引当（answer → execution → run_stage（NULL 可）→ plan（別 Tx、不在 = None）→ first_part）、`continue` は 3、`--phase` ジャンプは 2。同一 Tx（ジャーナル由来 15 表）の NOT NULL FK が引けなければ `ReadModelReadError::broken_projection()`（`InvalidData`）。判断は無い。
- **実害の是正**: 旧 `next_answer_dao_impl.rs` は RMU が書いた FK `run_stage_id`（RunStage 決定のときだけ非 NULL）を無視して自然キーで `read_run_stage` を結合し直し、`parked` の答えに RMU が「無し」と書いた run-stage を付けて返していた。`Fixture::parked()` で `bare`（`parked` / `stage_slug` 有り / `run_stage_id` NULL）と `reentry`（`run-stage` / FK 有り）の対を実データで固定。
- **`cargo lint` に `dao-single-table`（R5）**: クエリ側 interface-adapter の SQL リテラル（素の文字列 / `concat!` / `macro_rules!` 本体内 `concat!`）が 1 文で `read_*` 2 表以上・JOIN・副問合せを読んだら所見。赤例 4 + 緑例 6。README 機械化ロードマップ「更新 2026-09-03」（実装済み 4 本）。`run_stage_columns.rs` を const にせず `macro_rules!`（WHERE 句を受けて文全体を組む）にしたのは、`format!` 組立にすると lint の視野から消えるため（マクロ本体に JOIN を注入すると検出することを実証）。
- **memory 層**: Fable 5 委譲ポリシーを `project.md ## Mandated` へ登録（`CLAUDE.md` は Task/Agent 委譲時に配送されない — `stage-graph.json` の `rules_in_context` は memory/ の 4 本のみ）。`docs/CLAUDE.md` の同文は未削除（二重記載、次で整理）。

## 積み残し（記録のみ、起票しない）
- **`ReadModelStore` の共有**: DAO 実装ごとに読取専用接続を開くため、多段引当の断面一貫性は保証されない（同一プロセスで `catch_up` → 読取の現運用では実害なし）。b44 で「1 要求 = 1 `ReadModelStore` を DAO 群で共有」（不変共有なので `Rc` で足りる — `interior-mutability.md`）。
- **`read_next_jump_phase.next_jump_id` FK**（Fable B.4 任意、RMU 約 31 行）: 現状は `target_index` の自然キーで `read_next_jump` を引く（UNIQUE 索引なので裁定上は適法）。「関連行は FK 列で指す」に揃えるなら RMU 側。
- **壊れた intra-doc リンク 3 件（既存、b43 範囲外）**: `command/domain/.../intent_execution.rs:657`（`IntentExecution::crossed_phase_boundary`）、同 `intent_execution_id.rs:19`（`super::uuid_v7`）、`read-model-updater/.../journal_reader.rs:19`（`JournalEntry`）。`rustdoc::broken_intra_doc_links = deny` は `cargo doc` でしか発火せず、CI 3 ジョブは `cargo doc` を回さないため赤になっていない。私有アイテムへのリンク警告 2 件も同様（`CommitVerdictUseCase::is_stale_re_report` / `SecretFile::mint`）。
- **`docs/CLAUDE.md` § Fable 5 Delegation Policy** の二重記載を解消（memory 層への 1 行参照に置換）。

## 次
b44（Bolt 3 後半）: 旧 `NextUseCase` / `ContinueUseCase` 経路と両パーサの削除、app のコントローラ / プレゼンタを `NextTurnView` / `ContinuationView` 消費へ（`runtime.rs:40-44, 711-736` は旧ユースケースのまま）、golden 配線し直し、lint 2 本（#47）。
