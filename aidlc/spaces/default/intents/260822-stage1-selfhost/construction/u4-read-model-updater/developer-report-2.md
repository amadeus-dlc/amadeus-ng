# B8 開発者報告 2 — 是正版の最小手戻り（改名・移動 2 件）

Conversation language: 日本語
担当: 委任先（Opus）/ ブランチ: `bolt/b8-u4-read-model-updater` / 報告日: 2026-08-29
検証はすべて `CARGO_TARGET_DIR=$PWD/target-delegate` で実行。push はしていない。

---

## 0. 要約

**ブリーフ 4 は完了**。固定裁定 1〜3 をすべて実施し、**受入基準 1〜9 すべて PASS**。

コード変更は改名・移動 2 件の機械的追随のみで、依存構造・型・投影・テストの挙動は不変。
テスト件数は報告 1 の完了時点と同じ **738 件全緑**（増減なし = 挙動不変の裏取り）。

- コミット 4 本（`de88d43` 改名 1 / `393e28f` 改名 2 / `86d10a6` doc コメント是正 5 箇所 /
  `3a61d28` doc コメント最終スイープ 16 箇所）
- 改名 2 本の実体は「クレート名・パス名の綴り替え」と「相対パス 1 階層ぶんの補正」だけで
  60 files changed, +164 / −160（うち 26 件はリネーム検出）
- `86d10a6` / `3a61d28` はコメントのみ 15 ファイル（いずれも委任者承認済み。§7.2 / §7.2b）

---

## 1. 改名対応表

| 旧パス | 新パス | 旧パッケージ | 新パッケージ | 側 |
|---|---|---|---|---|
| `modules/core/domain` | `modules/core/command/domain` | `core-domain` | `core-command-domain` | コマンド |
| `modules/core/query/read-model-updater` | `modules/core/read-model-updater` | `core-query-read-model-updater` | `core-read-model-updater` | **中間** |

`modules/core/query/` は空になったので削除した。後方互換はゼロ（旧名 alias・shim・
`#[deprecated]` を一切置かず、呼出側を一斉修正）。

**追随した箇所**（機械的なもののみ）:

| 種別 | 内容 |
|---|---|
| workspace | `Cargo.toml` のメンバー 2 行と依存エイリアス 2 行、`Cargo.lock` |
| Rust 識別子 | `core_domain::` → `core_command_domain::`（39 ファイル）、`core_query_read_model_updater::` → `core_read_model_updater::` |
| doc コメント | 上記と同じ綴りが doc コメント・モジュール説明に現れる箇所（固定裁定 3 の指示どおり同時更新。旧パス表記の残存 0 件を grep で確認） |
| 相対パス | `engine_loop_conformance.rs` の適合フィクスチャ（1 階層**深く**なるため `../../../` → `../../../../`）、`projection_golden_test.rs` / `audit_block_golden_test.rs` のゴールデン（1 階層**浅く**なるため `../../../../` → `../../../`） |

## 2. 所有ファイル外の変更 1 件（**承認済み**）

`tools/lint/src/check.rs` — 6 行（パス定数 3 本）。ブリーフ 4 の所有ファイル一覧
（`Cargo.toml` / `Cargo.lock` / `modules/**` / 報告書）には含まれないが、**改名に必須の追随**
であるため実施した。

理由: `cargo lint` の `checkbox-vocabulary` ルールは語彙所有者 1 ファイル
（`CHECKBOX_OWNER`）だけを免除する設計で、この定数が `modules/core/domain/src/workspace/
checkbox.rs` を指している。ドメインを移した時点で免除が外れ、所有者ファイル自身が違反として
検出され **`cargo lint` が赤**（所見 2 件）になった。受入基準 3 と両立しない。

同種の追随はブリーフ 1 のときにも実施済み（当時はアダプタのパス定数 2 本）で、そのときと
同じ扱いにした。変更は定数の綴り替えのみで、ルールの検出ロジックには触れていない。
`tools/lint` の自己テスト 28 件は緑。**委任者承認済み**（改名に必須の隣接修正としてブリーフ 1
と同じ扱い）。

## 3. ブリーフ 3 の破棄確認

ブリーフ 3（RMU の wire parse 化）の**中核には着手していない**。撤回の連絡を受けた時点で
書きかけの実装は無く、破棄すべきコミット・未コミット編集も無い。

着手～撤回までに実際に行ったこと:

| # | 行為 | 現在の状態 |
|---|---|---|
| 1 | 上流 3 文書（`cqrs-boundaries.md` / ADR-009 / 構成案）の読み込み | 痕跡なし（読取のみ） |
| 2 | RMU のドメイン依存 19 箇所・語彙の使用者マップの実測 | 痕跡なし（読取のみ） |
| 3 | wire 形式の逆算 — `modules/app/aidlc/tests/wire_dump_scratch.rs` を一時作成し 12 変種＋境界値の実直列化 JSON をダンプ | **削除済み**（コミットしていない。作業ツリーに残骸なし） |
| 4 | 固定裁定 1（ドメイン改名） | **ブリーフ 4 でも生きるため保持**（`de88d43`） |
| 5 | 固定裁定 2〜6（依存除去・自前 parse 型・コントラクトテスト・語彙移動・`StoreLocation`） | **未着手**（矛盾検出により停止したため） |

3 で得た wire 形式の実測（外部タグ付き enum、`Unparked` は裸文字列、`project_type` のみ
小文字綴り 等）は、いま使い道が無いが、将来クエリ側クレートを新設して wire parse が必要に
なったときに再測定なしで参照できる。本報告に残す価値があると判断して §6 に付記した。

## 4. 受入基準（すべて自分で実行）

| # | 基準 | 結果 |
|---|---|---|
| 1 | `cargo fmt --all --check` | **PASS**（差分なし） |
| 2 | `cargo clippy --workspace --all-targets -- -D warnings` | **PASS**（exit 0、警告 0） |
| 3 | `cargo lint` | **PASS**（exit 0、所見 0。`tools/lint` 自己テスト 28 件緑） |
| 4 | `cargo test --workspace` | **PASS**（**738 passed / 0 failed**） |
| 5 | `bash scripts/quint-gate.sh` | **PASS**（`[PASS] quint gate: all steps green`） |
| 6 | `bash scripts/coverage.sh --base origin/main` | **PASS** — 絶対 98.473% ≥ 90.0% / 相対 98.473% ≥ 98.4217% − 0.01 |
| 7 | プロダクトコードの `unwrap`/`expect` 0 件 | **PASS**（下記） |
| 8 | 旧名 grep 0 件 | **PASS**（`grep -rn "core-query-read-model-updater\|modules/core/domain\b\|\"core-domain\"" modules/ Cargo.toml` = **0**） |
| 9 | ゴールデン検収緑 | **PASS** — 監査ブロック 42 ブロック（1 テスト）/ 投影 10 ケース両面一致（13 テスト）/ upstream load パリティ（9 テスト） |

すべて **最終コミット `3a61d28` の時点**で再実行した結果である（doc コメント是正の後に測り直した）。

**基準 7 の測り方**: 機械的な正本は基準 2 である（`[workspace.lints.clippy]` の
`unwrap_used = "deny"` / `expect_used = "deny"` に対し `clippy.toml` の
`allow-unwrap-in-tests` / `allow-expect-in-tests` がテストのみ許容する）。これが exit 0 で
通っている時点でプロダクトコードの `unwrap`/`expect` は 0 である。

補助として `#[cfg(test)]` 区間を除外する自前走査も掛けた。検出 4 件はすべて
`modules/shared/canon-json/src/parse.rs`（206〜375 行の `#[cfg(test)] mod` 内）で、走査器が
桁 0 の閉じ括弧で区間を早期に抜ける癖による**偽陽性**。本 Bolt では触れていない既存コード
であり、実質 0 件。

## 5. 挙動不変の裏取り

固定裁定 3（改名以外のコード変更禁止）を満たしていることの機械的な根拠:

- テスト件数が報告 1 の完了時点と**同一の 738 件**（追加も削除もしていない）
- カバレッジ head が **98.473%** で報告 1 の完了時点と同値
- ゴールデン検収（42 ブロック + 10 ケース両面バイト一致）が無改修で緑 — 投影の出力バイトが
  1 バイトも変わっていないことの直接証拠
- ITF 準拠（`journal_protocol` / `engine_loop`）と Quint ゲートが無改修で緑
- 差分 60 ファイル中の実体は綴り替えと相対パス補正のみ（`git diff` で確認可能）

## 6. 付記 — ブリーフ 3 で実測した wire 形式（将来のクエリ側クレート用）

コマンド側の実直列化から逆算した結果。推測ではなく `serde_json::to_string` の実出力である。
いま参照する必要はないが、再測定のコストを避けるために記録する。

- 外部タグ付き enum: `{"<変種名>": { … }}`。材料を持たない `Unparked` だけは**裸の文字列**
  `"Unparked"` になる（オブジェクトではない）
- `Option` は欠落ではなく `null` として出る（`"next_stage":null`）。空 `Vec` は `[]`
- 閉集合の綴りは 2 系統ある: `phase` / `plan_action` / `direction` / `mode` は Rust の変種名
  そのまま（`"Initialization"` / `"Execute"` / `"Backward"` / `"Autonomous"`）だが、
  `scan.project_type` だけ**小文字**（`"brownfield"` / `"greenfield"`）— `stage-graph.json` の
  正準綴りに合わせる `#[serde(rename_all = "lowercase")]` が付いているため
- 新形の値オブジェクトは素の文字列に潰れる: `StageSlug` / `WorkflowDefinitionId` /
  `DefinitionRevision` / `StateFieldValue` は `try_from = "String"` の newtype、
  `StageNumber` は `into = "String"` により生表現 `"4.5"` の文字列
- `StageDisplay` は `{"number","name","lead_agent"}`、`StageEntry` は
  `{"slug","phase","plan_action","conditional","display"}`、`WorkspaceScan` は
  `{"project_type","languages","frameworks","build_system"}`（空文字は構築時に `"Unknown"` へ）
- 型判別子（manifest 列）は `"workflow-execution-event/1"`

## 7. 未処理として引き継ぐもの

本 Bolt の範囲外だが、改名によって顕在化した／据え置いたもの。

### 7.1 正本との整合は確認済み

着手時点の `cqrs-boundaries.md` は冒頭（RMU は中間）と判定表（RMU はクエリ側の全実体・
ドメイン絶対禁止）が食い違っていたが、**完了時点で是正済み**であることを確認した。現在の
判定表と本 Bolt 後のクレート構成は一致している:

| 判定表の行 | 実装 |
|---|---|
| `core-command-domain`（コマンド） | `modules/core/command/domain` ✓ |
| `core-command-use-case`（コマンド） | `modules/core/command/use-case` ✓ |
| `core-command-interface-adapter`（コマンド実装） | `modules/core/command/interface-adapter` ✓ |
| `core-read-model-updater`（**中間**） | `modules/core/read-model-updater` ✓ |
| クエリ側クレート（将来） | 未作成 — ブリーフ 4 が他の変更を禁じているため作らない |
| 共有層 | `core-infrastructure` / `audit-events` / `message-catalog` ✓ |

### 7.2 doc コメント 5 箇所の是正（**完了** — 委任者承認 `86d10a6`）

是正裁定で説明の中身が覆った 5 箇所を実態へ揃えた。コメントのみでコード・挙動は不変。

| 場所 | 直した内容 |
|---|---|
| `read-model-updater/Cargo.toml:12` | 「共有層（両側が依存してよい唯一の層）」「コマンド側クレートはここに現れてはならない」が直下の `core-command-domain` 依存を**自己否定**していた。中間の特権として依存できる旨と、将来のクエリ側クレートには書けない旨へ差し替え |
| `read-model-updater/src/lib.rs:1` | 「クエリ側の全実体」→「コマンド側でもクエリ側でもない中間」。クエリ API 層は将来別クレートになる旨を追記 |
| `read-model-updater/src/lib.rs`（節見出し） | 「なぜコマンド側クレートが `Cargo.toml` に無いのか」→「なぜ依存が `core-command-domain` 1 つで足りるのか」。理由を「禁止だから」から「投影核の入口がイベント 1 本だから use-case / interface-adapter が要らない」へ |
| `read-model-updater/tests/support/mod.rs:1-14` | 本家に行を書かせる理由から「禁止だから」を除去（禁止は解けた）。「試験対象に忠実だから」という**今も生きている理由**だけを残した |
| `command/domain/.../event_manifest.rs:13-17` | 照合側を「クエリ側」→「中間である RMU」。正本の置き場所の理由を「両側が依存してよい唯一の層」→「RMU が中間として依存できるので写しを作らずに済む」へ |

### 7.2b 最終スイープ（**完了** — 委任者承認 `3a61d28`）

承認 5 箇所を直したあと全面走査し、RMU を「クエリ側」と呼ぶ記述を他に 14 箇所検出した。当初は
doc-sync 第 2 パスへ申し送ったが、doc-sync の所有ファイルは `docs`/`aidlc` のみで `modules/**`
を触れないため拾えない旨の指摘を受け、**同一クラスの残件処理として最終スイープを承認**された。
追加走査で 2 箇所（`journal_read_error.rs` / `app/aidlc/Cargo.toml`）が同種と判明し、**計 16
箇所**を 1 コミットで是正した。コメントのみ・挙動不変。

**呼称のみ（規則本文は真のまま）**: `command/use-case/src/lib.rs` /
`command/interface-adapter/src/lib.rs` / 同 `orchestration/mod.rs` — 「クエリ側」→「中間クレート
RMU」。あわせてコマンド側が守る規則を「**RMU を `Cargo.toml` に書いたら違反**」と明示した
（是正後の判定表では、コマンド側の依存に現れて違反になるのはクエリ側だけでなく RMU も含む）。

**合成ルートに置く理由の書き直し**: `app/aidlc/tests/journal_protocol_conformance.rs` /
`crash_reconstruction_test.rs` — 「両側を `Cargo.toml` に書いてよいのは合成ルートだけ」は失効
したので、現に成り立つ理由へ差し替えた: コマンド側は RMU を書けない → 置けるのは RMU 自身か
合成ルート → **実際に結線される場所で駆動するほうが観測として忠実**だから合成ルート。

**根拠が覆っていた 4 箇所（複製という判断自体は維持）**:

| 場所 | 直した理由づけ |
|---|---|
| `read-model-updater/.../corrupt_cause.rs`・`store_failure.rs` | 失効した禁止根拠（「両側は互いを知らないので共有すれば相手を `Cargo.toml` に書くことになる」）を落とし、今も生きている設計理由だけを残した — 「実際に起きうる変種だけを各面が持つ（無用な変種は『この面ではありえない』という情報を消す）」＋ 正本の「エラー分類・I/O 写像は側ごとに専用化」 |
| `command/use-case/.../corrupt_cause.rs`・`command/interface-adapter/.../store_failure.rs` | **コマンド側から見れば禁止根拠は今も真**（コマンド側の依存に RMU が現れたら違反）なので、禁止根拠を捨てずにその形へ書き直した |

**その他 2 箇所**: `read-model-updater/.../journal_read_error.rs` は相手型へ rustdoc リンクを
張らない理由を「両側は互いを知らない」→「RMU の依存に `core-command-use-case` が無いので
そもそも張れない」へ。`app/aidlc/Cargo.toml` は「合成ルートだけが**両側**を知る」→「合成ルートは
**すべて**を知る（コマンド側 3 + 中間の RMU + 共有層を配線する）」へ。

スイープ後の残骸走査は 0 件（「クエリ側」の残存 4 箇所はいずれも是正後の正しい用法 — 将来の
クエリ側クレートを指すか、「RMU はクエリ側でない」と述べている箇所）。

**再検証（`3a61d28` 時点）**: fmt PASS / clippy `-D warnings` exit 0 / `cargo lint` 所見 0 /
`cargo doc --workspace --no-deps` 警告 0 / `cargo test --workspace` 738 passed, 0 failed。

### 7.3 `tools/lint` のパス定数（§2）— **承認済み・対応不要**

### 7.4 U1 ゴールデン未採取の 2 点

報告 1 §8 に挙げた `state-template.md` 実バイトと単独 `StageCompleted` 行は本 Bolt でも未解消。
改名では変わらない。
