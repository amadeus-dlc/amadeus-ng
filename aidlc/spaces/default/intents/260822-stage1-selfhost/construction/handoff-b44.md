# ハンドオフ — b44（Bolt 3 後半）: クエリ側の旧経路を全撤去、app を `read_*` 経路へ配線、lint 2 本（#47 折り込み）（2026-09-04）

前段: [`handoff-b43.md`](handoff-b43.md)（Bolt 3 前半 — DAO 1 表 1 引当）。設計の正本は
[`query-side-audit/read-model-spec.md`](query-side-audit/read-model-spec.md) §7「クエリ側から
消えるもの」・§9 移行段階 3。裁定: オーナー 2026-09-03 = B「現状のキューのまま」（#7 キュー 3 を
b43 / b44 の PR 2 本で消化）。

## やったこと
- **(W) app を新経路へ配線**（`3fafa934` → リベース後 `5fdae8d6`）: `next` / `continue` を旧
  `NextUseCase` / `ContinueUseCase`（`aidlc-state.md` と配布 3 ファイルの逆パース）から、b43 の
  DAO 1 表 1 引当の経路へ切り替えた。コントローラ `app/src/turn.rs` は構文的ルーティングだけ
  （状態の値で決まる分岐を 1 つも持たない）、プレゼンタ `app/src/directive_drawing.rs` は行 →
  directive（相対パスの絶対化・1 行 JSON の展開・continue_token の組立、判断は無い）、逐語文言は
  `app/src/wording.rs`。**`ReadModelDaos`（新）** — 1 要求 = 1 読取専用接続を 12 実装が `Rc` で
  分け合う（b43 積み残し「`ReadModelStore` の共有」を解消）。`ScopeDao::find_all` /
  `FindSteeringUseCase` / `SteeringDeliveryView` を新設。
- **(D) 旧経路の全撤去**（`0b21e12b` → `e49e04ba`、69 ファイル・-14,618 行）: 旧ユースケース 2 本、
  判断型 6 本、steering の分割・パック 4 本、旧ポート 3 本と DTO 族（`port/workflow_view/` 23 型・
  `port/execution_view/`）、逆パース 2 本と `*DaoImpl` 3 本・in-memory ダブル 3 本、
  `workspace_layout.rs`。判断・導出・選択・文言組立は移設ではなく**削除**（判断は集約、計算結果は
  RMU が行に書いてある）。重複していたテスト 2 本（`interface-adapter/tests/golden_parity_test.rs`
  = command 側に同名 8 本、`engine_loop_ladder_conformance.rs` = b38 で domain 側へ復帰済み）と、
  対象ごと消えた内部構造のテスト 2 本を削除。
- **(G) CLI ゴールデンの配線**（`dd61ab05` → `de24001e`）: `modules/app/aidlc/tests/cli_golden_test.rs`
  が `tests/golden/upstream-3c3146cf/cli/` の採取済み stdout と app の実出力を突き合わせる。
  `continue/invalid-token` はバイト一致、`next/start` はキー集合一致、`continue/load-steering` は
  キー集合の差（既知の欠落 2 + 任意 3）を両向きで固定。駆動できない 3 ケース
  （`next/no-active-intent` / `next/stage-jump-print` = 逸脱台帳 #1 / `next/after-approval`）は
  理由をテストのモジュール doc に明記。
- **誕生分岐の是正**（`5943742b` → `bb23f843`）: fresh なワークスペースで定義がまだ投影されて
  いない場合に、最初の `next` の前に RMU が配布束を投影する（`runtime.rs` / `read_model_updater.rs`）。
  これで (W)〜(G) の各段で唯一赤だった `the_birth_print_names_a_command_the_receiving_surface_accepts`
  が緑になり、`next_branches.rs`（534 行）で `next` のルーティングと park を固定した。
- **`cargo lint` 2 本（GitHub #47 折り込み、README 機械化ロードマップ実装済み 6 本）**:
  `port-naming`（use-case 層の `pub trait` はコマンド側 `XxxRepository` / クエリ側 `XxxDao` のみ）と
  `command-side-io`（`modules/core/command/**` の `*_repository_impl.rs` 以外に fs / 乱数 /
  プロセス / ネットワークの I/O が現れたら所見）。正本は `gateway-taxonomy.md`（§1d 新設・
  「機械強制の候補」1 を実装済みへ）。導入時点の既存違反は 0 件。実装は `tools/lint/src/check.rs`（+661 行）: `port-naming` は `visit_item_trait` で無制限 `pub` の trait だけを見る（R4 と同じ可視性境界）、`command-side-io` は `use` ツリーの平坦化（`use std::{fs, io};` / `use std::fs::{self, File};` / glob）と式・型・パターン中の完全修飾パスの 2 経路で検出し、同一行の 2 件目以降は畳む。式中の裸の 1 セグメント（`rand` という変数名）は誤検出しないよう 2 セグメント以上を要求。テストは新規 23 本（R6 赤 4 + 緑 5、R7 赤 6 + 緑 8。射程外・除外の緑例には「同じソースが射程内では鳴る」対を添付）、lint クレートの自己テスト計 69 本全緑、スクラッチの仮ルートでバイナリを実走して両ルールが鳴ること・`intent_repository_impl.rs` の `use std::fs;` が沈黙することも確認済み（サブエージェント実測）。
- **テスト増強（PR 相対カバレッジゲート）**: リベース直後の実測は絶対 98.15%（32,128 行中 594 行未カバー）で
  絶対ゲート（90%）は PASS だが、main（99.10%）相対で 0.95pp 落ちて FAIL だった — 高カバレッジの
  旧経路 1.4 万行を消し、新配線（`turn.rs` 68% / `wording.rs` 77% / `directive_drawing.rs` 88%）が
  薄かったため。分岐ごとに 1 テストの方針で **158 本（+3,346 行）**を追加: `turn.rs` インライン 90
  （一時ワークスペースに定義 3 入力と memory 層を書き、runtime を駆動して各 `decision_kind` の
  行き先と文言を固定）、`wording.rs` 15、`directive_drawing.rs` 13、`runtime.rs` 14、
  `tests/next_branches.rs` 19、`tests/intent_lifecycle.rs` 5、query 側契約テストに `ScopeDao::find_all`
  の検収（綴り順・未取込は空・failing / empty ダブル）、`append_only.rs` 1。結果 **99.11%**
  （34,179 行中 305 行未カバー）で base 99.10% を上回り、許容誤差に頼らず PASS
  （`scripts/coverage.sh --base origin/main`、`PROPTEST_RNG_SEED` 固定）。
- **b43 積み残しの消化**: `CLAUDE.md` § Fable 5 Delegation Policy の二重記載を memory 層
  （`project.md ## Mandated`）への参照 1 段落に置換。`cargo doc` で赤になる intra-doc リンク
  （既存 3 + b44 の移設で壊れた 2）と private 参照の警告 2 を修正 — `cargo doc --workspace --no-deps`
  が警告 0 で通る（CI は `cargo doc` を回さないので、これは今回の実測のみ）。
- **memory 層**（`2e8aeba3` → `d644430b`）: 「コミットは作業ツリー全体を回収する — `git add` を
  パスで絞らない」を `project.md ## Corrections` へ登録（オーナー規律 2026-09-03）。
- **main へのリベース**（2026-09-04）: ブランチ上の Codex CLI 統合コミット（`6f1125aa`）は main 側
  #97 / #98 が上位互換なので落とした。監査シャード `8fc90228c64e` の衝突 6 回は追記専用ログとして
  「b44 側イベント（08:27〜14:30Z）→ main 側ブロック（15:04Z〜）」の順に結合し、時系列の単調性を
  機械確認した。`aidlc-state.md` は main 側（#99、16:20Z の paused 記録）を採用。

## 積み残し（記録のみ、起票しない）
- **`read_next_jump_phase.next_jump_id` FK 化**（b43 から継続、Fable B.4 任意）: 現状は
  `target_index` の自然キー引当（UNIQUE 索引なので裁定上は適法）。揃えるなら RMU 側 約 31 行。
- **CLI ゴールデンの駆動不能 3 ケース**: `next/no-active-intent`（state なしの誕生分岐は今回の
  是正で成立するようになったが、採取時のワークスペースを再現する fixture が無い）、
  `next/stage-jump-print`（逸脱台帳 #1 — マルチコール正準形 `aidlc-jump resolve` を名指すため
  バイト一致は設計上ありえない）、`next/after-approval`（配布の scope identity 11 ファイルが
  vendored されていない）。
- **到達不能コードの所見（テスト増強で判明、プロダクションは未変更）**: (a) `cli/request.rs::parse_next`
  は `NextTurnInput` の 5 観測（`read_only` / `noun_token` / `env_default_scope` /
  `kiro_latch_bare_next` / `records_exist_without_cursor`）を一度も立てないため、分岐 0（Kiro
  ラッチ）・1・1b〜1d（読取専用・名詞トークン）・`AWS_AIDLC_DEFAULT_SCOPE` 段と `invalid_env_scope`・
  `birth_group` の intent-pick はどの argv でも到達しない（`request.rs` の doc は読取専用・名詞文法だけ
  を後続 Bolt としており、環境変数と Kiro ラッチには触れていない）。いずれも `NextTurnInput` を直接
  組む単体テストで固定した。(b) `turn.rs:46-49` の `pre_guard` 呼出は `runtime.rs:163` と重複し CLI
  経由では発火しない。(c) `turn.rs:362`（`DEFAULT_SCOPE` のパース失敗腕）、`433-436`（scope-change 行が
  非 slug）、`622-627` / `633-639`（同じ DAO を先に引く 605 行で潰れる読取失敗腕）は到達不能。
  (d) `runtime.rs:717` の `harness_name` が `const fn` で `"claude"` を返すため `definition_id` /
  `compiled_definition_id` は失敗し得ず、447 / 449 / 545 / 547 の `map_err` は死にコード。
  (e) `runtime.rs:405-414` の `record_name::compose` 失敗腕も到達不能。— これらは #7 キュー 11
  （マルチコール CLI 配線）で `parse_next` が観測を立てるようになった時点で生きるものと、単純に
  削るべきものが混在する。削るのは判断が要るので b44 では触らず記録に留める。
- **#47 §4 のセンサー補完**（意味レベルの疑義は報告のみ・非ブロック）は未着手 — lint 2 本で
  機械判定できる範囲だけを着地させた。
- **`cargo doc` を CI に載せるか**は未裁定（今回は手元実測のみ。`rustdoc::broken_intra_doc_links =
  deny` は workspace lints にあるが、CI 3 ジョブは `cargo doc` を回さない）。

## 次
#7 キュー 4 以降: #74 park の完全実装 → #73 report の 13 段ガード・`--single`・
`--skeleton-stance`・`--user-input` → #72 set-autonomy → #71 WorkspaceScanner → #77 →
#82 → #53 → クリティカルパス 4〜6。
