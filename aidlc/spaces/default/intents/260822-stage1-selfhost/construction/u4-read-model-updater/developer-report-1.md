# B8 開発者報告 1 — CQRS 層の側分割 + U4 ReadModelUpdater

Conversation language: 日本語
担当: 委任先（Opus）/ ブランチ: `bolt/b8-u4-read-model-updater` / 報告日: 2026-08-29
検証はすべて `CARGO_TARGET_DIR=$PWD/target-delegate` で実行。push はしていない。

---

## 0. 要約

側分割（4 クレート再編 + infrastructure 層）は**完了**。U4 の RMU は**二層構造・投影ライタ・
監査ブロック描画・シャード横断読取が完成**し、投影規則（イベント → 行）は 12 変種中 6 変種が
ゴールデン逐語一致で検収済み、残り 6 変種は**未裁定のため意図的に未実装**である（誤ったバイトを
書く代わりに明示エラーで止める）。受入基準 1〜7 は PASS、基準 8 は**部分 PASS**。

未裁定の内容と選択肢は §6 に記載し、着手時点で委任者へ照会済みである（固定裁定 8 に従い、
読み替えずに止めた）。

---

## 1. クレート対応表

| 旧 | 新 | パッケージ名 | 側 |
|---|---|---|---|
| `modules/core/domain` | 同左 | `core-domain` | 共有（両側が依存してよい唯一の層） |
| `modules/core/use-case` | `modules/core/command/use-case` | `core-command-use-case` | コマンド |
| 〃（読取語彙 5 型） | `modules/core/query/read-model-updater` | `core-query-read-model-updater` | クエリ |
| `modules/core/interface-adapter` | `modules/core/command/interface-adapter` | `core-command-interface-adapter` | コマンド |
| 〃（`journal_reader_impl` / `state_file_io`） | `modules/core/query/read-model-updater` | 同上 | クエリ |
| `modules/infra-io` | `modules/core/infrastructure` | `core-infrastructure` | 層外（言語拡張） |
| （新設） | `modules/harness/infrastructure` | `harness-infrastructure` | 層外（憲章 doc のみ） |

**後方互換ゼロ**（固定裁定 1）: 旧クレート名の再輸出・shim・`#[deprecated]` は 1 つも置いて
いない。改名は呼出側の一斉修正で行った。`grep -rn "core_use_case\|core_interface_adapter\|infra_io"
modules/` は 0 件。

### 移動した型の行き先（構成案 §3 の裁定どおり）

| 部品 | 行き先 | 備考 |
|---|---|---|
| `StorePath` | `core-domain::workspace`（`store_path.rs:21`） | 「space → ストアの場所」はワークスペースの語彙 |
| `EVENT_MANIFEST` | `core-domain::orchestration`（`event_manifest.rs:18`） | 書く側と検める側が同じ正本を見る |
| `CorruptCause` | **側ごとに専用 enum へ分割** | コマンド側 4 変種 / クエリ側 3 変種。片側にしか意味の無い変種を持ち込まない |
| `store_failure`（`io_kind`） | **側ごとに複製** | コマンド側だけが `io_kind_of_source`（本家の箱を開ける経路）を持つ |
| `state_writers` | RMU の投影 API へ転居 | 11-workspace §2.3「描画は投影の責務」 |
| `state_file_io` | RMU の `workspace/state_file.rs` へ転生 | 投影ライタ（状態ファイル面） |

### 行数増減

- `git diff --shortstat $(git merge-base origin/main HEAD) HEAD -- modules Cargo.toml Cargo.lock tools`
  → **76 files changed, 4290 insertions(+), 337 deletions(-)**（rename 検出あり）
- 新設クレート `core-query-read-model-updater` は **6,026 行**（うち約 3,200 行は移動、
  約 2,800 行が新規実装＋テスト）
- テスト件数: **622 → 708 件（+86）**

---

## 2. 固定裁定 1〜8 の実施箇所

| # | 裁定 | 実施箇所 | 状態 |
|---|---|---|---|
| 1 | 後方互換ゼロ | 全クレートの `Cargo.toml` と `src/**`。再輸出・shim・`#[deprecated]` は 0 件 | 実施 |
| 2 | RMU は二層 | 取得ループ `orchestration/updater.rs:149`（`catch_up`）/ 純粋投影核 `workspace/projection.rs:201`（`project(entries, read_model)`）。投影核の署名にも本体にも `JournalReader`・接続・checkpoint が現れない | 実施 |
| 3 | `JournalReaderImpl` の挙動は移動で変えない | `orchestration/journal_reader_impl.rs`。rowid カーソル / `amadeus_projection_checkpoint` / (aid, seq_nr) アンカー照合 / busy_timeout 5000ms / CREATE なし接続はすべて素通しで移動。差分は `use` 文と doc の所在表現のみ。契約テスト 13 本が緑のまま追従 | 実施 |
| 4 | 投影出力は 0a 逐語契約 | `workspace/audit_block.rs:47`（`render_audit_block`）/ `:33`（`SHARD_HEADER` 19 バイト）/ `workspace/audit_shard.rs:48`（追記・ヘッダ）/ `workspace/state_file.rs:84`（tmp+rename）。冪等は `tests/read_model_updater_test.rs::regenerating_from_zero_twice_yields_identical_bytes` と `tests/projection_golden_test.rs::projecting_the_same_entries_from_the_same_state_twice_yields_the_same_bytes` | 実施（描画）/ 部分（投影規則、§6） |
| 5 | シャード横断の位置付き読取 | 順序規則 `core-domain::workspace::find_all_events`（`audit_ordering.rs:120`）、列挙と連結 `workspace/audit_shard.rs:82`（`read_all`）。結合テスト `tests/cross_shard_read_test.rs` 5 本 | 実施 |
| 6 | U7 は実装しない | `modules/app/aidlc/src/main.rs` は従来どおりスタブ。RMU はテストから直接駆動している | 実施 |
| 7 | Quint 不変 / ITF のフェイク投影を実 RMU へ差し替え | Quint は 1 文字も触っていない（`formal/` 無変更、quint-gate 全緑）。**ITF の投影差し替えは未実施** — 差し替え先の実 RMU が `Started` を描けないため（§6） | 部分（Quint 側は実施） |
| 8 | TDD / 矛盾は止めて裁定を求める | §5 に red の実例。実装不能点を発見した時点で作業を止めて委任者へ照会した（§6） | 実施 |

---

## 3. 受入基準 1〜8 の実行ログ

```text
### 1. cargo fmt --all --check
(exit=0)

### 2. cargo clippy --workspace --all-targets -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.05s
（error/warning の出力なし）

### 3. cargo lint
(exit=0)

### 4. cargo test --workspace
test result: ok.（39 バイナリすべて ok、失敗 0）
TOTAL: 708 passed

### 5. bash scripts/quint-gate.sh
  [PASS] quint test --match 'r_.*' (formal/orchestration/stop_hook.qnt)
[PASS] quint gate: all steps green

### 6. CARGO_TARGET_DIR=$PWD/target-delegate bash scripts/coverage.sh --base origin/main
base (origin/main) line coverage: 98.42167957117331%
[PASS] relative gate: head (98.48551554107598%) >= base (98.42167957117331%) - tolerance (0.01)
（絶対ゲート 90% 床も PASS）
```

| # | 基準 | 判定 |
|---|---|---|
| 1 | `cargo fmt --all --check` | **PASS** |
| 2 | `cargo clippy --workspace --all-targets -- -D warnings` | **PASS** |
| 3 | `cargo lint` | **PASS** |
| 4 | `cargo test --workspace` 全緑 | **PASS**（708 件） |
| 5 | `bash scripts/quint-gate.sh` | **PASS** |
| 6 | coverage 両ゲート | **PASS**（相対 98.486% ≥ 98.422% − 0.01、絶対 90% 床クリア） |
| 7 | プロダクトコードに unwrap/expect なし・`#[allow]` は理由必須 | **PASS**（下記） |
| 8 | 投影出力がゴールデンとバイト一致 | **部分 PASS**（下記） |

### 基準 7 の詳細

プロダクトコードの `unwrap`/`expect` は 0 件（workspace lints が deny、clippy が緑）。
新設した `#[allow]` は 1 つだけで、理由を付けてある:

- `workspace/projection.rs`（`mod recomposed_row` の `#![allow(dead_code, reason = "…")]`）
  — `Recomposed` の状態面が未裁定のあいだ本体から呼ばれないため。`expect` ではなく `allow`
  なのは、テストビルドでは実際に使われ `expect` が unfulfilled になるからである（その理由も
  コメントに書いた）。

なお移動前から存在した `#[allow]`（`journal_reader.rs:24` の `async_fn_in_trait` ほか）は
いずれも既存の理由付きであり、内容を変えていない。

### 基準 8 の詳細（部分 PASS の内訳）

**PASS した面**:

1. **監査ブロック描画の全数一致** — `tests/audit_block_golden_test.rs`。
   `tests/golden/upstream-3c3146cf/**/audit.md` の **全 42 ブロック・24 イベント型**を読み取って
   `render_audit_block` で描き直し、**バイト完全一致**を確認。空シャードへの初回書込だけが
   ヘッダ行を持つ非対称も固定した（42 ブロック中ヘッダ 1 件）。
2. **投影の両面一致（6 変種）** — `tests/projection_golden_test.rs`。1 ドメインイベントから
   監査行と状態ファイル差分の**両方**を描き、`audit.md` と `state.diff` の実バイトと突合:

   | イベント | ゴールデン | 監査面 | 状態面 |
   |---|---|---|---|
   | `GateOpened` | `cli/report/awaiting-approval` | ✓ | ✓ |
   | `GateRejected` | `cli/report/rejected` | ✓（2 行） | ✓ |
   | `StageRevised` | `cli/report/revised` | ✓ | ✓ |
   | `Parked` | `cli/park/park` | ✓ | ✓ |
   | `Unparked` | `cli/unpark/unpark` | ✓ | ✓ |
   | `Recomposed` | `cli/recompose/skip-one` | ✓（行のみ、ユニットテスト） | 未裁定 |

3. **冪等（NFR3）** — 同じチェックポイントから 2 度流して同一バイトになることを、投影核と
   取得ループの両方で固定。

**未達の面**: 残る 6 変種（`Started` / `StageCompleted` / `GateApproved` / `StageSkipped` /
`Jumped` / `Recomposed` の状態面）は §6 の未裁定事項により未実装。

---

## 4. 実装の要点

### 4.1 二層構造（固定裁定 2）

- **投影核** `project(entries: &[JournalEntry], read_model: &mut ReadModel)` — 入口は
  ドメインイベント 1 本。集約・Repository・ストアのエラーは現れない。
- **リードモデル** `ReadModel` — 2 面の非対称を型に出した。状態ファイルは**置換**（現在値を
  持つので読んで書き換える）、監査シャードは**追記**（台帳なので既存行を読まず、この回に足す
  バイト列だけを持つ）。
- **投影の記憶はリードモデルにある** — 後続行の `**Scope**:` は状態ファイルの
  `- **Scope**:` から読む。差分投影で `Started` が同じバッチに入らなくても成り立ち、別の場所に
  状態を持たないので冪等が壊れない。
- **取得ループ** `catch_up()` — checkpoint → `events_after` → 投影核 → リードモデル書込 →
  `advance_checkpoint`。

### 4.2 行偽造不能性を型で保証した

`core-domain::workspace` に `AuditFieldKey` / `AuditFieldValue` / `AuditFields` を新設した
（11-workspace §2.2 の Domain Primitive、未実装だった）。3 つの行偽造を**描き手の規律ではなく
型**で構成不能にしている:

- 第二の `**Event**:` 行 → `AuditFieldKey::parse` が `Event` を拒否
- 第二の `**Timestamp**:` 行 → `AuditFields` が `Timestamp` を受理して破棄（upstream の
  「受理して黙って捨てる」と同じ観測挙動を、捨てる位置をコレクション側へ寄せて実現）
- 値に混ぜた改行 → `AuditFieldValue::of` が行終端をエスケープ済みにする

`AuditFields` は挿入順を保つ第一級コレクションである（並びが観測面なので `BTreeMap`/`HashMap`
では表現できない。同じキーを二度置くと位置は最初のまま値だけ差し替わる = JS のプロパティ
再代入と同じ意味論）。

### 4.3 監査行のタイムスタンプはイベントの発生時刻

upstream は追記時の壁時計（`new Date().toISOString()`）を書くが、投影は冪等でなければ
ならない（NFR3 — 同じ checkpoint から何度流しても同一バイト）。壁時計を読むと再生成のたびに
バイトが変わるので、**ジャーナル行が運ぶイベントの発生時刻**を秒精度 ISO 8601 で書く。
upstream 側の観測面（`<TS>` に正規化される秒精度 ISO 8601）は変わらない。

### 4.4 書いてから進める（at-least-once）

`catch_up` はリードモデルをディスクへ落としてから checkpoint を進める。逆順にすると書込直前の
クラッシュで**監査行が永久に失われる** — 台帳にとって欠落は重複より重い、という選択である。
書込後・前進前に落ちた場合は同じ差分を再実行することになり、状態ファイルは同じ位置へ落ち着く
（冪等）が、監査シャードには同じブロックがもう一度並ぶ。この非対称は doc コメントに明記した。

### 4.5 `EventType::heading()` は 1 件ずつ逐語

`audit-events` に 86 語の見出しを追加した。語形はワイヤ綴りから機械変換**できない**:
`STAGE_COMPLETED → Stage Completion`（名詞化）なのに `UNIT_COMPLETED → Unit Completed`
（過去分詞）、`RECOMPOSED → Plan Recomposed`（語幹に無い語が付く唯一の例）。この非一様性
そのものをテストで固定した（`the_irregular_headings_are_the_ones_a_mechanical_conversion_would_miss`）。
抽出は `docs/specs/research/golden-3c3146cf-audit.md` §1 の逐語表から機械的に行い、86/86 の
キー・見出しがともに distinct であること、既存 86 語のワイヤ綴り集合と完全一致することを
突合してから流し込んだ（転記ミスの余地を残さないため）。

---

## 5. TDD の実例（red → green）

- **`render_audit_block` の行偽造テスト**: 最初に書いた表明が `block.matches("**Event**: ").count() == 1`
  で red。落ちた出力を読んで、**偽造の条件は「行頭が `**Event**:` の行が 2 本現れる」こと**で
  あり部分文字列の出現数ではないと分かったため、`lines().filter(|l| l.starts_with(...))` へ
  是正して green。upstream の読み手も複数行正規表現で行頭に錨を打つので、こちらが正しい述語である。
- **ゴールデン全数テスト**: 「ゴールデンにヘッダ行は無い」という前提で書いて red。
  `cli/intent-create/classic-scope/audit.md` だけが `# AI-DLC Audit Log\n` を持つと判明し、
  「空シャードへの初回書込だけがヘッダを持つ」という非対称を**テストの表明として追加**して green。
  さらにブロック数の期待値が 41 で red → 実測 42 が正しく、42 全部がバイト一致していた。
- **park マーカーの挿入位置**: `with_field_or_insert`（upstream `setOrInsertField`）を流用した
  実装がゴールデンと食い違うことをテストで検出。`cli/park/park/state.diff` の実バイトはマーカーが
  **空行のあと・次の `## ` 見出しの直前**にあり、`setOrInsertField`（空行より前へ巻き戻す）とは
  別の書き手だと判明したため、専用に実装し直した。

---

## 6. 止めて裁定を求めた事項（固定裁定 8）

### 6.1 `STAGE_STARTED` の `**Agent**:` がイベントから導けない

ゴールデンの実バイトでは、ほぼ全ての遷移の末尾に付く `STAGE_STARTED` 行が
`**Agent**: aidlc-design-agent` のようなフィールドを持つ。しかしこの値はドメインイベントから
導けない:

- `StageEntry`（`Started` が運ぶステージ 1 件、`core-domain/src/orchestration/stage_entry.rs:12`）の
  フィールドは `slug` / `phase` / `plan_action` / `conditional` の 4 つだけで agent が無い
- agent は `StageNode::lead_agent()`（`core-domain/src/workflow_definition/stage_node.rs:347`）—
  **ワークフロー定義（ステージグラフ）側**の材料
- ADR-008 により `WorkflowExecution` は定義を `definition_id` + `definition_revision` で
  **間接参照**しており、定義の詳細をイベントへ複製していない

同じ性質の欠落が状態ファイル側にもある:

- `cli/skip/skipped/state.diff` の `Active Agent` / `Next Action: Execute Refined Mockups`
  （ステージ表題）
- `cli/recompose/skip-one/state.diff` の `Stages to Execute` / `Stages to Skip` に現れる
  **ステージ番号**（`4.5 (incident-response)`）と各行の EXECUTE/SKIP 接尾辞
- `cli/intent-create/classic-scope/audit.md` の 16 行が要求する `Project Type: Greenfield` /
  `Languages` / `Build System` / `Details: 4 in-scope phase dirs + verification/ …` —
  これらは initialization 3 ステージの**副作用（ワークスペース走査・スキャフォールド結果）**で
  あり、ジャーナルには存在しない

これは contract-summary §4 が **U4 へ持ち越した未解決項目**（`contract-summary.md:435`）に
該当する。委任者へ選択肢 A（RMU にクエリ側の定義読取ポートを新設）/ B（`StageEntry` に agent を
持たせイベントへ焼き込む）/ C（繰り延べ）を推奨順で提示し、照会済みである。

**裁定が降りるまでの扱い**: 誤ったバイトを書く代わりに
`ProjectionError::DefinitionLookupRequired`（`workspace/projection.rs:75`）で止める。
`the_events_that_need_the_workflow_definition_stop_instead_of_writing_wrong_bytes` が
「止まったなら状態面に手を付けていない」ことも併せて固定している。

### 6.2 固定裁定 7（ITF のフェイク投影差し替え）が 6.1 に依存する

`journal_protocol` の ITF トレースは `Started` / `StageCompleted` / `GateApproved` を含むため、
フェイク投影を実 RMU に差し替えると 6.1 の未裁定に当たって止まる。**Quint モデルは 1 文字も
変えておらず**（`formal/` 無変更、quint-gate 全緑）、ITF 準拠テスト自体も緑のままだが、
差し替えは 6.1 の裁定後に行う必要がある。

---

## 7. 独自解釈（裁定を仰がずに決めた点）

構成案・coding-rules・ゴールデンから一意に決まらず、規則の優先順で自分で裁定した点を列挙する。

1. **両側を駆動するテストの置き場は合成ルート**。`journal_protocol_conformance.rs` と
   `crash_reconstruction_test.rs` はコマンド側とクエリ側の両方を使う。cqrs-boundaries が
   「両側を知ってよいのは合成ルートだけ」と定めているので `modules/app/aidlc/tests/` へ移した。
   統合テストのモジュールはクレートを跨げないため、`tests/support/` は必要な分だけ複製した。
2. **`JournalReaderImpl` のテストは本家ストアに行を書かせる**。コマンド側 Repository を
   dev-dependency に取ると機械判定（`Cargo.toml` に相手の側が現れる）で違反になるため、
   `EventEnvelope` + `EventStoreForSqlite` で直接書く形へ書き換えた。試験対象への忠実さも上がる
   （`JournalReaderImpl` が結合しているのは本家の `journal` 表であって我々の Repository ではない）。
3. **RMU の dev-dependency に `event-store-adapter-rs` を入れた**。本家スキーマへのピン留め
   ガードは実物の DDL を観測しなければ意味を持たない。本家は「側」ではないので禁止事項に
   当たらないと判断した。
4. **`CorruptCause` は両側とも同じ型名のまま**。概念は同一（「行は読めたがドメインへ写せない
   理由」）で閉集合だけが違う。ubiquitous-language に照らして名前を変えるほうが不正確になる。
5. **`Clock` はコマンド側 interface-adapter に置いた**。構成案に記載が無い。`occurred_at` を
   供給するのはコマンド側のユースケースであり、投影は時刻をイベントから採る（§4.3）ので
   クエリ側には要らない。
6. **`AuditFieldKey` / `AuditFieldValue` / `AuditFields` は core-domain に置いた**。
   11-workspace §2.2 が `AuditFieldKey` を workspace 文脈の Domain Primitive（E2）と定めており、
   §2.3 が「行終端エスケープによる行偽造不能性」をドメインに残すと明記しているため。
7. **`find_all_events` の分割**。11-workspace §2.3 は domain に残すと言い、unit-of-work U4 は
   横断読取を U4 の責務と言う。順序付けの**純関数**を domain に、シャード列挙とファイル読取
   （I/O）を投影側に置くことで両立させた（§2.3 が `find_all_events` を「ドメインサービス
   （純関数）」の表に載せていることが根拠）。
8. **`AUTONOMY_MODE_SET` 行のフィールドキーを `Mode` とした**。ゴールデン未採取である（§8）。
9. **`AutonomyMode::as_state_field()` を domain に追加**。状態ファイルへ書く綴りは
   `from_state_field` の逆写像であり、読む綴りと書く綴りが割れると自分で書いた値を読み戻せなく
   なるため、往復忠実をテストで固定できる場所（domain）に置いた。
10. **書いてから checkpoint を進める順序**（§4.4）。C5 は冪等をチェックポイントに帰しているが、
    書込と前進のどちらが先かは書いていない。「台帳の欠落は重複より重い」で決めた。

---

## 8. 仕様とのドリフト（doc-sync 向け）

| # | 対象 | 内容 |
|---|---|---|
| 1 | `coding-rules/cqrs-boundaries.md` | §機械強制の末尾に「**RMU はどちらが現れてもよい**」が残っている。2026-08-29 改訂の判定表は RMU 行に「コマンド側クレートは禁止」と書いており**矛盾**する。README の衝突規則 4（裁定日が新しいほうが勝つ）で後者を採用したが、文面の是正が要る |
| 2 | `crate-structure-proposal.md` §2 の依存グラフ | RMU の依存に `audit-events` / `message-catalog` / `core-infrastructure` が抜けている（実装で必要になった。いずれも共有/層外であり側ではない） |
| 3 | 委任ブリーフ §その他の必読 | 「`render_audit_block` / `state_writers` は core-domain の workspace 文脈に**実装済み**」とあるが、実測では `state_writers` のみ実装済みで **`render_audit_block` は未実装**だった（本 Bolt で新規実装した）。`AuditFieldKey` も 11-workspace §2.2 に載っているが未実装だった |
| 4 | `docs/specs/11-workspace.md` §2.3 | `find_all_events` を「domain に残す」とあるが、実際にはシャード列挙・ファイル読取という I/O を伴う。純関数（domain）と I/O（投影）の分割を明記すると読み手が迷わない |
| 5 | `contract-summary.md` C5 | `AUTONOMY_MODE_SET` 行のフィールドが未定義。`cli/set-autonomy` のゴールデンは失敗経路（`ERROR_LOGGED`）しか捉えておらず、成功時の行が採取されていない。**U1 の追加採取が要る**（実装は暫定で `**Mode**: autonomous\|gated`） |
| 6 | `contract-summary.md` C5 rules | 「同一シャード内で直接行と投影行がどちらの順で現れるべきか」は未定義のまま（契約レビュー所見 2 が指摘済み・未解消）。本 Bolt では投影が自分の描いた分を追記するだけなので抵触していない |
| 7 | `docs/specs/11-workspace.md` §2.3 の表 | `state_writers` の行き先が「投影 API」と書かれているが、移動先クレート名（`core-query-read-model-updater` の `workspace` モジュール）まで書くと追跡しやすい |
| 8 | ADR-009 / cqrs-boundaries | 側分割に伴い `modules/infra-io` が `modules/core/infrastructure` へ改名された（2026-08-29 の infrastructure-layer 裁定）。層の一覧を持つ文書（`docs/specs/01-domain-model.md` など）に旧名が残っていないか要確認 |

---

## 9. 申し送り

1. **§6.1 の裁定**が最優先。降り次第、投影規則の残り 6 変種と ITF 差し替え（固定裁定 7）を
   同じ Bolt で仕上げられる。`ProjectionError::DefinitionLookupRequired` の分岐を実装で
   置き換えるだけの形にしてある（`recomposed_row` モジュールは監査行の逐語を検証済みのまま
   保存してあり、裁定後に `project_one` の分岐を戻せばよい）。
2. **並行コミットの注意**: 作業中に委任者側のコミット `7ba62ba` へ、私が `git mv` で
   ステージしていたファイル移動 2 件（`event_manifest.rs` / `store_path.rs`）が巻き込まれた。
   `git mv` は仕様上インデックスへステージするためである。移動自体はブランチに載っており実害は
   無いが、コミット境界が混ざっている。
3. **`tools/lint/src/check.rs`** のテスト内パス定数 2 本を新レイアウトへ追随させた（`cargo lint`
   自身のテストが参照するパス。検出ロジックには影響しない）。
