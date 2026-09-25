//! **コマンド側でもクエリ側でもない中間** — ReadModelUpdater（U4）。
//!
//! RMU の仕事はジャーナル（コマンド側が書いた事実）を読んでリードモデル（`aidlc-state.md` と
//! 監査シャード）を書くことであり、どちらの側にも属さない。したがって RMU は**両側に依存できる
//! 唯一のクレート**である（2026-08-24 原裁定 / 2026-08-29 是正、
//! `coding-rules/cqrs-boundaries.md` 判定表）。リードモデルを**読む**側 — クエリ API の
//! ユースケース層 — はまだ存在せず、それが起きたときは別クレートになる（そちらは
//! `core-command-domain` に**絶対依存しない**）。
//!
//! # 二層構造（2026-08-28 裁定）
//!
//! RMU は 2 つの層でできている。
//!
//! 1. **取得ループ** [`orchestration::OrchestrationReadModelUpdater`] — チェックポイントを読み、
//!    `events_after` で差分を引き、投影核へ渡し、`advance_checkpoint` で前進する。
//!    ストレージ・接続・チェックポイントを知るのはこちらだけである。
//! 2. **純粋投影核** [`workspace::project`] — ドメインイベントの列とリードモデルだけを
//!    受け取り、リードモデルを作成・更新する。`JournalReader`・SQLite 接続・チェックポイントを
//!    **一切知らない**。
//!
//! 二層を潰してはならない（`coding-rules/cqrs-boundaries.md` 禁止パターン）。投影核が取得の
//! 都合を知ると、投影の規則だけを単体でテストできなくなる。
//!
//! # 更新の入口は 1 つの契約に揃える
//!
//! 取得ループは投影の単位ごとに複数ある（取得ループ本体・構造化面だけ・テスト契約・計画指紋・
//! Code Generation 開始可否・runtime-graph・停止制御・心拍・自己診断・承認ランタイム）。その
//! すべてが共通契約 [`orchestration::ReadModelUpdater`] を実装し、更新の入口は
//! `update_read_models()` 1 つである。更新はコマンド（戻り値なし）で、描く対象は更新器を
//! 組むときに束ねる。型名は `…ReadModelUpdater` で揃え、`cargo lint` の
//! `read-model-updater-contract` が名前と実装の対応を検査する。
//!
//! # リードモデルは 2 系統ある
//!
//! 1. **Markdown 面（系統 (1)）** — [`workspace`] が描く `aidlc-state.md` と監査シャード。
//!    人と upstream ツールがそのまま読むファイルであり、逐語の互換面である。
//! 2. **構造化面（系統 (2)）** — [`read_tables`] が描く SQLite の `read_*` 表。CLI の読取
//!    コマンド（`next` / `continue` / 将来の `--status` / `doctor`）が 1 回の引当で答えを
//!    得るための**非正規化リードモデル**である。行の値は集約のクエリの答えの写しであり、
//!    クエリ側が系統 (1) を逆パースして自分で計算することは禁じられている
//!    （`coding-rules/cqrs-boundaries.md` 規則 6 の 2026-09-02 追記）。
//!
//! 2 系統は同じ取得ループが 1 回の更新で両方描く。構造化面の行の差し替えと
//! チェックポイントの前進は 1 トランザクションに閉じる（裁定 §3）。
//!
//! # 集約ごとに独立した投影単位がある
//!
//! [`read_tables`] の 17 表とは別に、**自分の manifest だけを投影する**小さな投影単位が
//! 2 つある。どちらも自前のチェックポイント表を持ち、取得ループと投影核の二層は同じである。
//! 心拍 (`HookHealth`) は毎回全履歴から再投影する。自己診断は「ジャーナルを読む → 投影 →
//! 表の DAO で更新する」形へ移行済みで (Issue #153 の PR1 —
//! `coding-rules/read-model-updater-structure.md`)、処理したシーケンス番号 (ジャーナル上の
//! 位置) より後の事実だけを読み、触れた集約の行と番号を 1 つのトランザクションで確定する。
//!
//! | 投影単位 | manifest | 表 | チェックポイント |
//! | --- | --- | --- | --- |
//! | [`orchestration::HookHealthReadModelUpdater`] | `hook-health-event/1` | `read_hook_health` | `hook_health_projection_checkpoint` |
//! | [`orchestration::WorkspaceDoctorReadModelUpdater`] | `workspace-doctor-event/1` | `read_doctor_report` / `read_doctor_check` | `workspace_doctor_projection_checkpoint` |
//!
//! 自己診断 (`aidlc --doctor`) の 2 表は次の形である。`read_doctor_report` は診断対象ごとに
//! 1 行で、主キーは集約 id (`WorkspaceDoctorId`)、自然キー `target`
//! (`spaces/<space>/intents[/<record>]`) に UNIQUE インデックスを張る。列
//! `passed` / `failed` / `exit_code` は**集約のクエリの答えを焼き込んだ**もので、クエリ側は
//! 数えない。`read_doctor_check` は表示順の 1 行 1 レコードで、主キーは自然キー
//! (`report_id` × `position`) から導いた代理キー、FK 列 `report_id` が報告を指し、
//! `check_id` / `passed` / `label` / `fix` は集約が決めた行の写しである。DAO は 1 表 1 引当で
//! 引き、ユースケースが FK をたどって View を組む。
//!
//! 診断のイベントは合成ルートが開いた**一時ストア**(プロセス内の共有キャッシュ SQLite) に
//! 置かれることがある — C7 が「診断は正本を修復・再初期化しない」「初回状態でファイルを
//! 一切作らない」と定めるためであり、投影の手順は実ファイルのストアと 1 行も違わない。
//!
//! # 構造化面の表の形 — 単一主キー + FK + インデックス
//!
//! `read_*` 表は**基本的な関係モデリング**で設計する（オーナー裁定 2026-09-03。本クレート doc が
//! 正本。旧 `docs/specs/11-workspace.md` §4.1 は 2026-09-07 に削除した）。
//!
//! - **主キーは 1 列 `id`**。複合主キーにしない。集約そのものを表す 3 表
//!   （`read_definition` / `read_intent` / `read_execution`）の `id` は集約 id そのもので、
//!   それ以外は**自然キーの正準 JSON のダイジェスト**から導いた決定的な代理キーである
//!   （`"$a:$b"` のような連結文字列は使わない — 区切り文字が値に現れると衝突する）。
//! - **自然キーの列は残し、UNIQUE インデックスで重複を止める**。主キーが代理キーになった
//!   ぶん、自然キーを守るのはこのインデックスだけである。
//! - **関連行は FK 列で指す**（`run_stage_id` / `steering_plan_id` / `intent_id` /
//!   `definition_id` …）。SQLite の `FOREIGN KEY` 句は書かない — steering の 2 表は別
//!   トランザクションで差し替わるので、参照整合の DB 強制は投影順序と衝突する。対が
//!   揃っていることは投影核の契約テストが固定する。
//! - **クエリ側が `WHERE` に置く列にはセカンダリインデックス**を張る。DAO は **1 表 1
//!   引当**（JOIN しない）で、ユースケースが FK をたどって表ごとに引き View を組む
//!   （判断は無い — null の FK は「無し」である）。
//!
//! # 材料はジャーナルだけではない — steering の面
//!
//! 構造化面のうち **steering の 2 表**（`read_steering_plan` / `read_steering_part`）だけは、
//! ジャーナルではなく**参照入力**（active-space の memory 層にある規則ファイル）から作る。
//! 規則は人が編集するのでイベントを 1 件も伴わず、ジャーナルの走査位置と無関係に変わる。
//! したがってこの面は:
//!
//! - `as_of`（走査位置）を持たず、代わりに `source_digest`（読んだ規則ファイル群の
//!   ダイジェスト）で「どの参照入力から作られたか」を名乗る。
//! - 取得ループの**先頭**で毎回読み比べられ、`source_digest` が動いたときだけ差し替わる
//!   （ジャーナル差分が空でも見る）。
//! - チェックポイントとは**別のトランザクション**で書かれる。
//!
//! 読取は [`orchestration::SteeringSource`]、分割とパックは [`read_tables::SteeringTables`]
//! である（二層構造はジャーナル側と同じ — 読む層と計算する層を潰さない）。
//!
//! # なぜ依存が `core-command-domain` 1 つで足りるのか
//!
//! 中間であることと両側を広く使ってよいことは別である。投影核の入口はドメインイベント 1 本に
//! 絞ってあり、コマンド側の他の型（集約の再水和・Repository・ストアのエラー）は入口に現れない。
//! したがってコマンド側から要るのはドメインイベントを持つ `core-command-domain` だけで、
//! `core-command-use-case` と `core-command-interface-adapter` は 1 つも要らない。読取側の契約
//! （`JournalReader` / `ProjectionName` / `GlobalSeqNr`）と SQLite 実装は RMU 自身が所有する。
//! 依存の少なさは規律ではなくクレート分離で保つ — 増やせば `Cargo.toml` に現れる。

#![forbid(unsafe_code)]

pub mod orchestration;
pub mod read_tables;
pub mod workspace;

#[cfg(test)]
#[path = "../../../../tests/support/review_fixture.rs"]
mod review_test_fixture;
