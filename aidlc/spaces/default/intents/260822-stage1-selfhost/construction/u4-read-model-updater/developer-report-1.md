# B8 開発者報告 1 — CQRS 層の側分割 + U4 ReadModelUpdater

Conversation language: 日本語
担当: 委任先（Opus）/ ブランチ: `bolt/b8-u4-read-model-updater` / 報告日: 2026-08-29
検証はすべて `CARGO_TARGET_DIR=$PWD/target-delegate` で実行。push はしていない。

---

## 0. 要約

**B8 は完了**。側分割（4 クレート再編 + infrastructure 層）と U4 の RMU（二層構造・投影ライタ・
監査ブロック描画・シャード横断読取・投影規則 12 変種すべて）が揃い、固定裁定 1〜8 をすべて
実施した。**受入基準 1〜8 すべて PASS**（基準 8 は残り 2 点の未採取ゴールデンに限り明示エラー
で停止する設計 — §8 参照）。

途中 1 件、投影が `STAGE_STARTED` の `**Agent**:` を描けない実装不能点を発見し、固定裁定 8 に
従って読み替えずに止めて裁定を求めた。**オーナー裁定 A**（表示属性と走査結果を `Started`
イベントへ焼き込む）を受けて実装を完了している（§6）。

- コミット 16 本、90 files changed, +6,619 / −412
- テスト **622 → 738 件**（+116）
- ゴールデン検収: 監査ブロック描画**全 42 ブロック**バイト一致、投影**10 ケース**を
  `audit.md` + `state.diff` の両面バイト一致

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
  → **90 files changed, 6,619 insertions(+), 412 deletions(-)**（rename 検出あり）
- 新設クレート `core-query-read-model-updater` は **7,374 行**（うち約 3,200 行は移動、
  残りが新規実装＋テスト）
- テスト件数: **622 → 738 件（+116）**、コミット 16 本

---

## 2. 固定裁定 1〜8 の実施箇所

| # | 裁定 | 実施箇所 | 状態 |
|---|---|---|---|
| 1 | 後方互換ゼロ | 全クレートの `Cargo.toml` と `src/**`。再輸出・shim・`#[deprecated]` は 0 件 | 実施 |
| 2 | RMU は二層 | 取得ループ `orchestration/updater.rs:149`（`catch_up`）/ 純粋投影核 `workspace/projection.rs:201`（`project(entries, read_model)`）。投影核の署名にも本体にも `JournalReader`・接続・checkpoint が現れない | 実施 |
| 3 | `JournalReaderImpl` の挙動は移動で変えない | `orchestration/journal_reader_impl.rs`。rowid カーソル / `amadeus_projection_checkpoint` / (aid, seq_nr) アンカー照合 / busy_timeout 5000ms / CREATE なし接続はすべて素通しで移動。差分は `use` 文と doc の所在表現のみ。契約テスト 13 本が緑のまま追従 | 実施 |
| 4 | 投影出力は 0a 逐語契約 | `workspace/audit_block.rs:47`（`render_audit_block`）/ `:33`（`SHARD_HEADER` 19 バイト）/ `workspace/audit_shard.rs:48`（追記・ヘッダ）/ `workspace/state_file.rs:84`（tmp+rename）/ `workspace/projection.rs`（12 変種の投影規則）。冪等は `tests/read_model_updater_test.rs::regenerating_from_zero_twice_yields_identical_bytes` と `tests/projection_golden_test.rs::projecting_the_same_entries_from_the_same_state_twice_yields_the_same_bytes` | 実施 |
| 5 | シャード横断の位置付き読取 | 順序規則 `core-domain::workspace::find_all_events`（`audit_ordering.rs:120`）、列挙と連結 `workspace/audit_shard.rs:82`（`read_all`）。結合テスト `tests/cross_shard_read_test.rs` 5 本 | 実施 |
| 6 | U7 は実装しない | `modules/app/aidlc/src/main.rs` は従来どおりスタブ。RMU はテストから直接駆動している | 実施 |
| 7 | Quint 不変 / ITF のフェイク投影を実 RMU へ差し替え | Quint は 1 文字も触っていない（`formal/` 無変更、quint-gate 全緑）。`FakeProjection` を削除し、`catchup` ステップを実 `ReadModelUpdater::catch_up()` に置き換えた（`modules/app/aidlc/tests/journal_protocol_conformance.rs:356`） | 実施 |
| 8 | TDD / 矛盾は止めて裁定を求める | §5 に red の実例。実装不能点を発見した時点で作業を止めて委任者へ照会し、裁定 A を受けて完了させた（§6） | 実施 |

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
test result: ok.（全バイナリ ok、失敗 0）
TOTAL: 738 passed

### 5. bash scripts/quint-gate.sh
  [PASS] quint test --match 'r_.*' (formal/orchestration/stop_hook.qnt)
[PASS] quint gate: all steps green

### 6. CARGO_TARGET_DIR=$PWD/target-delegate bash scripts/coverage.sh --base origin/main
head line coverage: 98.47303704894318%
[PASS] absolute gate: head (98.47303704894318%) >= threshold (90.0%)
base (origin/main) line coverage: 98.42167957117331%
[PASS] relative gate: head (98.47303704894318%) >= base (98.42167957117331%) - tolerance (0.01)
```

| # | 基準 | 判定 |
|---|---|---|
| 1 | `cargo fmt --all --check` | **PASS** |
| 2 | `cargo clippy --workspace --all-targets -- -D warnings` | **PASS** |
| 3 | `cargo lint` | **PASS** |
| 4 | `cargo test --workspace` 全緑 | **PASS**（738 件） |
| 5 | `bash scripts/quint-gate.sh` | **PASS** |
| 6 | coverage 両ゲート | **PASS**（絶対 98.473% ≥ 90%、相対 98.473% ≥ 98.422% − 0.01） |
| 7 | プロダクトコードに unwrap/expect なし・`#[allow]` は理由必須 | **PASS**（下記） |
| 8 | 投影出力がゴールデンとバイト一致 | **PASS**（下記） |

### 基準 7 の詳細

プロダクトコードの `unwrap`/`expect` は 0 件（workspace lints が deny、clippy が緑）。
新設した `#[allow]` は理由つきのものだけである（`serde` 境界の往復確認に素の
`serde_json::to_string` を使う 2 か所と、テストの添字アクセス）。

### 基準 8 の詳細

1. **監査ブロック描画の全数一致** — `tests/audit_block_golden_test.rs`。
   `tests/golden/upstream-3c3146cf/**/audit.md` の **全 42 ブロック・24 イベント型**を読み取って
   `render_audit_block` で描き直し、**バイト完全一致**を確認。空シャードへの初回書込だけが
   ヘッダ行を持つ非対称も固定した。
2. **投影の両面一致（10 ケース）** — `tests/projection_golden_test.rs`。1 ドメインイベントから
   監査行と状態ファイル差分の**両方**を描き、`audit.md` と `state.diff` の実バイトと突合:

   | イベント | ゴールデン | 監査面 | 状態面 |
   |---|---|---|---|
   | `Started` | `cli/intent-create/classic-scope`（16 行） | ✓ | 骨格があれば ✓（§8-1） |
   | `GateOpened` | `cli/report/awaiting-approval` | ✓ | ✓ |
   | `GateRejected` | `cli/report/rejected`（2 行） | ✓ | ✓ |
   | `StageRevised` | `cli/report/revised` | ✓ | ✓ |
   | `GateApproved` | `cli/report/approved`（3 行） | ✓ | ✓ |
   | `StageSkipped` | `cli/skip/skipped`（2 行） | ✓ | ✓ |
   | `Jumped` | `cli/jump/execute-forward`（3 行） | ✓ | ✓ |
   | `Recomposed` | `cli/recompose/skip-one` | ✓ | ✓ |
   | `Parked` | `cli/park/park` | ✓ | ✓ |
   | `Unparked` | `cli/unpark/unpark` | ✓ | ✓ |
   | `StageCompleted` | **未採取**（§8-2） | 同型で実装 | 同型で実装 |
   | `AutonomyModeSet` | 行のフィールドが**未採取**（§8-2） | 暫定 | ✓（失敗文言が固定） |

   テストの計画は手写しせず、**upstream の出荷グラフ**（`stage-graph.json` 33 ノード）と
   `scope-grid.json` の classic 列から組む。手写しの値で合わせにいくと「テストに合わせた実装」
   になるためである。`reverse-engineering` を greenfield で畳むと in-scope は 25 になり、
   `- **Total Stages**: 25` と一致する — グラフ側の整合そのものが検証になっている。

3. **冪等（NFR3）** — 同じチェックポイントから 2 度流して同一バイトになることを、投影核と
   取得ループの両方で固定。

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

## 6. 止めて裁定を求めた事項と、その裁定（固定裁定 8）

### 6.1 発見した実装不能点

ゴールデンの実バイトでは、ほぼ全ての遷移の末尾に付く `STAGE_STARTED` 行が
`**Agent**: aidlc-design-agent` のようなフィールドを持つ。しかしこの値はドメインイベントから
導けなかった:

- `StageEntry`（`Started` が運ぶステージ 1 件）のフィールドは `slug` / `phase` /
  `plan_action` / `conditional` の 4 つだけで agent が無い
- agent は `StageNode::lead_agent()`（`core-domain/src/workflow_definition/stage_node.rs:347`）—
  **ワークフロー定義側**の材料
- ADR-008 により `WorkflowExecution` は定義を `definition_id` + `definition_revision` で
  **間接参照**しており、定義の詳細をイベントへ複製していなかった

同じ性質の欠落が状態ファイル側にもあった（`Active Agent` / `Next Action` のステージ表題、
`4.5 (incident-response)` のステージ番号）。さらに `Started` の 16 行は
`Project Type` / `Languages` / `Build System` のように **initialization 3 ステージの副作用
（ワークスペース走査結果）**を材料に取り、これもジャーナルには無かった。

固定裁定 8 に従い、読み替えずに作業を止めて選択肢 A / B / C を推奨順で提示した。

### 6.2 オーナー裁定 = A（実施済み）

**担当エージェント名・ステージ番号・ステージ表題は `Started` イベントの解決済み計画へ
焼き込む。走査結果 4 項目も同様。** 実装:

- `StageDisplay`（`core-domain/src/orchestration/stage_display.rs`）— 番号・表題・担当。
  `StageEntry` が持ち、`WorkflowExecution::start` が計画を解決する時点でグラフノードから
  焼き込む。表題と担当は `StateFieldValue` で**単一行を型で保証**する。
- `WorkspaceScan`（同 `workspace_scan.rs`）— プロジェクト種別・言語・フレームワーク・
  ビルドシステム。`Started` が持つ。
- 計画は**投影核の引数** `ResolvedPlan` として渡す（`workspace/resolved_plan.rs`）。表示属性を
  運ぶのは `Started` だけであり、差分投影のバッチにそれが入っているとは限らないためである。
  取ってくるのは取得ループの仕事（初回だけジャーナル先頭から引いて控える）で、投影核は
  相変わらず reader も接続も checkpoint も知らない — **二層は保たれている**。

**イベントを太らせる案は採らなかった**（遷移イベントごとに表示属性を持たせる形）。同じ事実が
ジャーナルに何度も転写され、`Started` の計画と食い違いうるためである。正本は 1 つでよい。

判断基準はオーナーが示した「投影がジャーナルだけで全監査行をバイト一致で描けること」で、
`tests/projection_golden_test.rs` の 10 ケースがそれを実測している。

### 6.3 ADR-008 との関係

`StageDisplay` の doc に明記した: ADR-008 の「定義を間接参照し詳細を複製しない」は**定義全体**
の複製を禁じたものであり、解決済み計画の表示属性はその限定的な例外である。運ぶのは描画に要る
3 値だけで、`consumes` / `produces` / `sensors` といった定義の本体は依然としてイベントに載らない。
ADR への追記は委任者が完了後の窓で行う。

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
8. **`AUTONOMY_MODE_SET` 行のフィールドキーを `Mode` とした**。ゴールデン未採取である（§8-2）。
9. **`AutonomyMode::as_state_field()` を domain に追加**。状態ファイルへ書く綴りは
   `from_state_field` の逆写像であり、読む綴りと書く綴りが割れると自分で書いた値を読み戻せなく
   なるため、往復忠実をテストで固定できる場所（domain）に置いた。
10. **書いてから checkpoint を進める順序**（§4.4）。C5 は冪等をチェックポイントに帰しているが、
    書込と前進のどちらが先かは書いていない。「台帳の欠落は重複より重い」で決めた。
11. **計画をイベントではなく投影核の引数にした**（§6.2）。裁定 A は「`Started` へ焼き込む」で
    あり、そこまでは裁定どおり。差分投影で `Started` が同じバッチに無いときにどう渡すかは
    裁定に書かれていないので、`ResolvedPlan` を引数にする形を選んだ（オーナーが示した判断基準
    「投影がジャーナルだけで描けること」を満たし、かつ二層構造を潰さない）。
12. **`StageDisplay` の型は `StageNode` の表現に合わせた**（`StageNumber` + 単一行の文字列 2 本）。
    `StageEntry` は `StageNode` の射影なので、同じ値に別の型を与えると 2 つの表現が生まれる。
13. **`StageCompleted` の `**Details**:` を `Stage <表題> completed` とした**。ゴールデン未採取
    のため、`GateApproved` の完了部（`Stage <表題> approved by gate`）と同型に置いた（§8-2）。

---

## 8. 仕様とのドリフト（doc-sync 向け）

### 8-1 / 8-2 — ゴールデン未採取のため実装が止まる 2 点（U1 の追加採取が要る）

| # | 内容 |
|---|---|
| 8-1 | **状態ファイルの骨格**。`Started` は本文が既にある状態ファイルへならフィールドを書けるが、9 セクション・31 フィールド行の**骨格そのもの**を起こすには upstream `state-template.md` の実バイトが要る。ゴールデンは差分（`state.diff`）しか持たない。骨格を推測して書くと 0a 逐語契約を静かに破るので `ProjectionError::ScaffoldTemplateUnavailable` で止める。あわせて `- **Stages to Execute**: ` / `- **Stages to Skip**: ` の 2 行は genesis で触らない — ゴールデンの実バイトは `2.1 (reverse-engineering — greenfield)` のように**畳まれた理由**を括弧内に持つが、`PlanAction` は EXECUTE / SKIP の 2 値しか持たず導けないため |
| 8-2 | **`StageCompleted`（非ゲート完了）の行**と **`AUTONOMY_MODE_SET` 行のフィールドキー**。前者は出荷グラフで非ゲートなのが initialization 3 ステージだけで、その 3 本は `Started` の投影が描くため、単独の `complete_stage` 経路の実バイトが無い。後者は `cli/set-autonomy` のゴールデンが失敗経路（`ERROR_LOGGED`）しか捉えていない。前者は `GateApproved` の完了部と同型、後者は `**Mode**:` を暫定で置き、いずれも doc コメントに未採取である旨を明記した |

### 8-3 以降 — 文書側の是正

| # | 対象 | 内容 |
|---|---|---|
| 3 | `coding-rules/cqrs-boundaries.md` | §機械強制の末尾に「**RMU はどちらが現れてもよい**」が残っている。2026-08-29 改訂の判定表は RMU 行に「コマンド側クレートは禁止」と書いており**矛盾**する。README の衝突規則 4（裁定日が新しいほうが勝つ）で後者を採用した。**委任者が完了後の窓で是正**すると連絡済み |
| 4 | `crate-structure-proposal.md` §2 の依存グラフ | RMU の依存に `audit-events` / `message-catalog` / `core-infrastructure` が抜けている（実装で必要になった。いずれも共有/層外であり側ではない） |
| 5 | 委任ブリーフ §その他の必読 | 「`render_audit_block` / `state_writers` は core-domain に**実装済み**」とあるが、実測では `state_writers` のみで **`render_audit_block` は未実装**だった。`AuditFieldKey`（11-workspace §2.2）・`find_all_events`（§2.3）・`with_checkbox_suffix`（§2.2）も同様に未実装で、本 Bolt で新規実装した |
| 6 | `docs/specs/11-workspace.md` §2.3 | `find_all_events` を「domain に残す」とあるが、実際にはシャード列挙・ファイル読取という I/O を伴う。純関数（domain）と I/O（投影）の分割を明記すると読み手が迷わない |
| 7 | `contract-summary.md` C5 | 裁定 A により `Started` の payload が拡張された（`StageEntry` へ `StageDisplay`、`Started` へ `WorkspaceScan`）。`projects_to` の yaml へ反映が要る。§4 の未解決項目「`Started` の投影の厳密な行順」は本 Bolt で確定した（16 行、`cli/intent-create` が正本） |
| 8 | `contract-summary.md` C5 rules | 「同一シャード内で直接行と投影行がどちらの順で現れるべきか」は未定義のまま（契約レビュー所見 2 が指摘済み・未解消）。本 Bolt では投影が自分の描いた分を追記するだけなので抵触していない |
| 9 | ADR-008 | 「定義を間接参照し詳細を複製しない」に、解決済み計画の表示属性という限定的な例外が加わった（オーナー裁定 2026-08-29）。**委任者が追記**すると連絡済み |
| 10 | 層の一覧を持つ文書（`docs/specs/01-domain-model.md` ほか） | `modules/infra-io` → `modules/core/infrastructure`（`core-infrastructure`）の改名と `harness-infrastructure` の新設が反映されているか要確認 |

## 9. 申し送り

1. **残る作業は文書側だけ**である。コードの受入基準 1〜8 はすべて PASS で、固定裁定 1〜8 も
   すべて実施済み。§8 の 8-1 / 8-2 は**ゴールデンの追加採取（U1）**が要る項目であり、採取が
   済めば投影側は数十行で埋まる（止まる箇所は `ProjectionError` の変種 1 つと doc コメントに
   局所化してある）。
2. **並行コミットの注意**: 作業初期に委任者側のコミット `7ba62ba` へ、私が `git mv` で
   ステージしていたファイル移動 2 件（`event_manifest.rs` / `store_path.rs`）が巻き込まれた。
   以後は委任者がコミット凍結を宣言し、再発していない。
3. **`tools/lint/src/check.rs`** のテスト内パス定数 2 本を新レイアウトへ追随させた（`cargo lint`
   自身のテストが参照するパス。検出ロジックには影響しない）。
4. 検証用の `target-delegate/` が未追跡で残っている（`.gitignore` は `/target` のみ）。委任者が
   掃除すると連絡済み。
