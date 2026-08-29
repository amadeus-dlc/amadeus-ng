# B8 開発者報告 2 — 是正版の最小手戻り（改名・移動 2 件）

Conversation language: 日本語
担当: 委任先（Opus）/ ブランチ: `bolt/b8-u4-read-model-updater` / 報告日: 2026-08-29
検証はすべて `CARGO_TARGET_DIR=$PWD/target-delegate` で実行。push はしていない。

---

## 0. 要約

**ブリーフ 4 は完了**。固定裁定 1〜3 をすべて実施し、**受入基準 1〜9 すべて PASS**。

コード変更は改名・移動 2 件の機械的追随のみで、依存構造・型・投影・テストの挙動は不変。
テスト件数は報告 1 の完了時点と同じ **738 件全緑**（増減なし = 挙動不変の裏取り）。

- コミット 2 本（`de88d43` / `393e28f`）、60 files changed, +164 / −160、うち 26 件はリネーム検出
- 差分の実体は「クレート名・パス名の綴り替え」と「相対パス 1 階層ぶんの補正」だけ

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

## 2. 所有ファイル外の変更 1 件（要確認）

`tools/lint/src/check.rs` — 6 行（パス定数 3 本）。ブリーフ 4 の所有ファイル一覧
（`Cargo.toml` / `Cargo.lock` / `modules/**` / 報告書）には含まれないが、**改名に必須の追随**
であるため実施した。

理由: `cargo lint` の `checkbox-vocabulary` ルールは語彙所有者 1 ファイル
（`CHECKBOX_OWNER`）だけを免除する設計で、この定数が `modules/core/domain/src/workspace/
checkbox.rs` を指している。ドメインを移した時点で免除が外れ、所有者ファイル自身が違反として
検出され **`cargo lint` が赤**（所見 2 件）になった。受入基準 3 と両立しない。

同種の追随はブリーフ 1 のときにも実施済み（当時はアダプタのパス定数 2 本）で、そのときと
同じ扱いにした。変更は定数の綴り替えのみで、ルールの検出ロジックには触れていない。
`tools/lint` の自己テスト 28 件は緑。

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

### 7.2 doc コメント 5 箇所が是正前の理屈のまま（**未修正・要判断**）

固定裁定 3（改名以外のコード変更禁止）を厳格に守り、**手を付けていない**。旧クレート名・旧パス
の綴り替えは指示どおり済ませたが、以下は綴りではなく**説明の中身**が是正裁定で覆った箇所で、
機械的追随の範囲を超えると判断した。修正してよいか判断をください（コメントのみ・挙動不変）。

| 場所 | 現在の記述 | 是正後の事実 |
|---|---|---|
| `modules/core/read-model-updater/Cargo.toml:12-13` | 「共有層 (両側が依存してよい唯一の層)」「コマンド側クレート (`core-command-*`) はここに現れてはならない」 | `core-command-domain` はコマンド側であり共有層ではない。RMU は**中間の特権**として依存してよい。**コメントが直下の依存宣言を自己否定している** |
| `modules/core/read-model-updater/src/lib.rs:1` | 「**クエリ側の全実体**」 | RMU は中間（どちらの側でもない） |
| `modules/core/read-model-updater/src/lib.rs:25` | 「依存は共有層（`core-command-domain`）と…」 | 同上（共有層ではない） |
| `modules/core/read-model-updater/tests/support/mod.rs:1-14` | 「クエリ側の試験装置」「クエリ側クレートの `Cargo.toml` にコマンド側クレートを書くことは禁止された」「共有層 (`core-command-domain`) の型であり、両側が使ってよい」 | 禁止は解けている。ただし**この試験装置の設計自体は今も妥当**（本家が書いた行を読む、が試験対象に忠実 — 理由の後半は生きている） |
| `modules/core/command/domain/src/orchestration/event_manifest.rs:15` | 「綴りの正本は**両側が依存してよい唯一の層**に置く」 | ドメインはコマンド側。manifest の正本がここにあることは判定表が追認しているが、理由の書き方が旧位置 |

いずれも「RMU がクエリ側だった前提」で書かれている。**依存構造そのものは是正後の正本と
一致している**ので、直すべきは説明文だけである。指示をもらえれば綴りだけ直す追加コミットを
出す（推定 15 行以内）。

### 7.3 `tools/lint` のパス定数（§2）

所有ファイル外の変更 6 行。承認が要るなら差分は小さく、戻すと `cargo lint` が赤になる。

### 7.4 U1 ゴールデン未採取の 2 点

報告 1 §8 に挙げた `state-template.md` 実バイトと単独 `StageCompleted` 行は本 Bolt でも未解消。
改名では変わらない。
