# ハンドオフ — b45: park の完全実装 — 集約ガード再設計（再スタンプ許容）+ `ParkUseCase` + 逐語 3 形（2026-09-04）

対象: GitHub #74（#7 キュー 4）。設計書: [`b45-park-complete/design.md`](b45-park-complete/design.md)。
前段: [`handoff-b44.md`](handoff-b44.md)（app が `read_*` 経路で描く。park 未配線のため parked 系テストは行を直接置いていた）。

## やったこと
- **集約 `IntentExecution::park` の受理述語を park 専用に**（`intent_execution.rs`）: 取り違え → autonomy（`RefusedUnderAutonomy`）→
  `Status::is_running`（Completed は `NotRunning`）。`parked_active()` は見ない = **再スタンプ許容**（park 済みへの park は
  `Parked` を再 emit、位置は同じカーソル）。`accepts_commands()`（BR1.0）は他コマンド共有のため不変。
- **Quint `engine_loop.qnt` v2.3**: `actPark` を `Running or WorkflowParked` へ緩和、witness `w_repark`、不変条件
  `parked_marker_status`（`status' = Running` 変異の既存検出穴を塞ぐ — 統合判断、オーナー確認待ち）、
  ITF フィクスチャ `trace-0x303`（再スタンプ経路）、準拠テストに合成アクション `repark` の網羅アサート。
  mutation 検査の表は `b45-park-complete/design.md` §8。
- **`ParkUseCase` / `ParkError`**（command use-case、新規 2 ファイル）: find → `park` → store、`Conflict` は 1 回だけ再構成から
  再試行、CQS で戻り値なし。エラーは封筒 3 変種（連鎖を切らない）。
- **app**（`runtime.rs` / `wording.rs`）: `Request::Park` を本物へ。失敗はすべて `error` directive `Cannot park the workflow: <detail>`
  （exit 0 — upstream の `handlePark` と同じ層）。逐語 2 形（autonomous / Completed）は `wording` の定数、その他は
  失敗の `Display` を中継。成功は `catch_up` 後に `FindExecutionUseCase` で `read_execution.parked_at_slug` を読み
  `Directive::Parked` を描く（投影直後に行が無いのは自己防衛拒否 = exit 1）。b29 の未配線ハンドラと文言を撤去。
- **テスト**: 新規 24 本（domain 3・use-case 13・app 統合 7・app 単体 4）。CLI ゴールデン `cli/park/park` の 3 値バイト一致。
  b44 が行を直接置いていた parked 系 3 本を `park` の実駆動へ置換（handoff-b44 の約束を履行）。
- **仕様**: `docs/specs/10-orchestration.md` §10 に「park の実装ノート」（受理順序・再スタンプ・順序 3 は構造的に発生不能で
  逸脱ではない・失敗の中継形）。逸脱台帳は追記なし。

## 積み残し（記録のみ、起票しない）
- `unpark` 面（`aidlc-state unpark` 相当）は本 Bolt の対象外（#74 は park）。`next --resume` on parked は分岐 2.6 の文言で `unpark` を案内するが、その動詞はまだ配線されていない。
- `parked` directive の `narration`（`Pausing here with everything saved. ...`）は directive 全般の既知の欠落（`cli_golden_test.rs` の記載）で、本 Bolt では扱わない。
- 未鋳造ワークスペースでの `park` の文言（`No workflow execution to park. Run `next` first.`）は upstream に対応する逐語が無い（upstream は状態ファイル不在時に `readStateFile` の失敗文を中継する）。
- **ワークフロー完了がリード面へ投影されない（b45 で判明、対象外）**: 全ステージを report で畳んでも状態ファイルは
  `- **Status**: Running` のままで、`WORKFLOW_COMPLETED` の監査ブロックも出ない。書き面（集約 `Status::Completed`）は
  正しく、park の Completed 拒否テストは書き面の証拠で判定している。RMU の投影の穴なので #7 キュー 5（#73 report）で扱う。
- **不変条件 `parked_marker_status` の採否**: 統合判断で追加した（モデルヘッダの簡約規約をそのまま不変条件化）。
  オーナーが退けるなら、モデル・gate の 1 行ずつを戻せば足りる。
- **`in-memory` ダブルが `Conflict` 以外の store 失敗を返せない**ため `park_use_case.rs` の `Err(other)` 腕 1 行が未カバー。

## 次
#7 キュー 5: #73 report の 13 段ガード・`--single`・`--skeleton-stance`・`--user-input`（B10 述語はレシートの鮮度検査のみ、#51 = A）。
