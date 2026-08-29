# B12 開発者報告 1 — 集約 `WorkflowExecution` → `Intent` の改名

対象ブリーフ: [`brief-1.md`](brief-1.md)
ブランチ: `bolt/b12-intent-aggregate-rename`（origin/main 基準、**push なし**）
検証の `CARGO_TARGET_DIR`: `target-delegate`（`cargo lint` のみ `target-delegate-lint`）

---

## 0. 結論（先に）

**受入基準 1〜11 はすべて緑**である（オーナー再訂正の反映後に全ゲートを再実行した実測）。
`cargo test --workspace` は **774 passed / 0 failed** で、
改名前（origin/main = B11 着地時点）の 774 と**同数**。退行ゼロで、テストの増減も無い。

固定フィクスチャに旧名や `"intent_id"` のバイトが埋まっている箇所は**発見されなかった**
（§3 に実測手順）。したがって「止めて報告」の条件には該当しなかった。

判断が要った点は 5 つある。いずれも §5 に書いた:

- `EVENT_MANIFEST` の値 `"workflow-execution-event/1"` を**据え置いた**（改名一族の表に無く、
  doc 自身が「行に書かれて残る値」として逐語固定を明記している）
- ブリーフ更新で入った `IntentSnapshot` の**クレート内私有への降格**は、`Intent::state` /
  `from_state` の降格と memento アクセサ 16 本の削除を伴った
- `JournalEntry::intent_id()`（RMU の読取行）は集約のアクセサではないので**据え置いた**
- 再訂正で `IntentSnapshotBuilder` をクレート内私有にした結果、利用箇所がテストだけになり
  dead_code で赤になったため **`#[cfg(test)]` でも絞った**（リポジトリの既存 house style に前例あり）
- **`StateError` は改名しなかった。`SnapshotError` は「別の型」ではなく、この型自身の旧名**
  だった（B5 で `SnapshotError` → `StateError` へ改名済み、`entities.md` に「旧名の再エクスポート・
  型エイリアスは残さない」と記録）。衝突ではないが**過去の裁定を巻き戻す判断**になるので、
  独断で決めず据え置いて裁定を仰ぐ

---

## 1. 改名対応表（実測）

作業中にブリーフが 2 回更新された。1 回目は `WorkflowExecutionState` の行と `…StateBuilder` の
行（`IntentState` / `IntentStateBuilder` を取りやめ、`IntentSnapshot` / `IntentBuilder` +
`build()` が集約を返す形へ）、2 回目は Builder 名の確定（`IntentSnapshotBuilder`、`build()` は
写しを返す元の形へ戻す）である。**最終版の内容に従っている**。1 回目の指示で入れた
`build() -> Result<Intent, StateError>` は巻き戻した。

| 旧 | 新 | 実測 |
|---|---|---|
| `WorkflowExecution`（集約） | `Intent` | 置換済み |
| `WorkflowExecutionEvent` | `IntentEvent` | 置換済み |
| `WorkflowExecutionState` | `IntentSnapshot` + **クレート内私有へ降格** | 置換 + `pub(crate)` 化、facade から除外 |
| `WorkflowExecutionStateBuilder` | `IntentSnapshotBuilder` + **クレート内私有へ降格** | 置換 + `pub(crate)` 化 + `#[cfg(test)]`、facade から除外。`build()` は**従来どおり写しを返す** |
| `WorkflowExecutionRepository` | `IntentRepository` | 置換済み |
| `WorkflowExecutionRepositoryImpl` | `IntentRepositoryImpl` | 置換済み |
| `InMemoryWorkflowExecutionRepository` | `InMemoryIntentRepository` | 置換済み |
| `RehydratedWorkflowExecution` | `RehydratedIntent` | 置換済み |
| `AGGREGATE_TYPE_NAME = "WorkflowExecution"` | `"Intent"` | `intent_id.rs:19`。`type_name()` の逐語アサートも追随 |
| 集約フィールド `intent_id` / `intent_id()` | `id` / `id()` | 置換済み（memento の同名フィールドも追随） |

**置換の規模（実測）**: 識別子の機械置換は **1290 箇所 / 40 ファイル**。Bolt 全体の diff は
**41 ファイル・760 insertions・955 deletions**（削除超過は memento アクセサ 16 本の削除ぶん）。

### ファイル改名（すべて `git mv`）

| 旧 | 新 |
|---|---|
| `domain/src/orchestration/workflow_execution.rs` | `intent.rs` |
| `domain/src/orchestration/workflow_execution_event.rs` | `intent_event.rs` |
| `domain/src/orchestration/workflow_execution_state.rs` | `intent_snapshot.rs` |
| `domain/proptest-regressions/orchestration/workflow_execution.txt` | `intent.txt` |
| `use-case/src/orchestration/workflow_execution_repository.rs` | `intent_repository.rs` |
| `use-case/src/orchestration/rehydrated_workflow_execution.rs` | `rehydrated_intent.rs` |
| `interface-adapter/src/orchestration/workflow_execution_repository_impl.rs` | `intent_repository_impl.rs` |
| `interface-adapter/tests/workflow_execution_repository_contract.rs` | `intent_repository_contract.rs` |
| `interface-adapter/tests/workflow_execution_repository_impl_test.rs` | `intent_repository_impl_test.rs` |

`proptest-regressions` のパスはモジュール経路から導かれるので、集約の mod 名の改名に合わせて
同時に動かした（動かさないと既存の回帰シードが読まれなくなる）。

### 改名しなかったもの（対象外の確認）

| 対象 | 実測 |
|---|---|
| `WorkflowDefinition` 一族 | 213 箇所すべて無傷（`WorkflowExecution` は `WorkflowDefinition` に部分一致しないので巻き添えは構造的に起きない） |
| ドメインイベントの変種名・payload | 無改変（`Started` / `GateOpened` / … と各フィールド） |
| `IntentId` / `IntentDirName` | 無改変 |
| 監査語彙・投影出力 | 無改変（§2） |
| `RepositoryError::NotFound { intent_id }` | 無改変（材料名であって集約のフィールドではない） |
| `JournalEntry::intent_id()` | 無改変（§5 (c)） |
| `EVENT_MANIFEST = "workflow-execution-event/1"` | 無改変（§5 (a)） |
| `tests/golden/**` / `formal/**` / `docs/**` / `coding-rules/**` / `aidlc/**`（報告書を除く）/ `.claude/**` | 差分 0 |

---

## 2. 外形不変の証明（受入基準 8）

- `git diff --name-only origin/main..HEAD -- tests/` が**空** — ゴールデンは 1 バイトも
  変わっていない。
- **投影ゴールデン 19 本が無改変で全緑**（`projection_golden_test.rs` は `running 19 tests`）。
  ゴールデンパリティ 9 本・監査ブロックゴールデン 1 本・クラッシュ再構成 5 本も全緑。
- 監査語彙（`WORKFLOW_STARTED` 等）は `EventType` の語彙であり、集約の型名とは独立している。
  改名は `WorkflowExecution` の部分一致でしか動かないので、`WORKFLOW_*`（全大文字・アンダー
  スコア）には触れていない。

### 永続化バイトが変わる箇所（意図どおり・互換問題なし）

改名の結果、次の 2 つは**書かれるバイトが変わる**。どちらもブリーフが明示的に認めている:

1. `AggregateId::type_name()` が返す値 `"WorkflowExecution"` → `"Intent"`（本家 v3 が集約種別を
   識別するのに使う）
2. スナップショット payload の serde フィールド名 `intent_id` → `id`

ジャーナル・スナップショットは**クローンごとの使い捨てランタイム**（`.gitignore` 済み）で
あり、リポジトリに固定バイトとして存在しない。ゴールデンにも含まれていない（§3）。

---

## 3. 固定フィクスチャの確認（ブリーフの「止めて報告」条件）

**旧名や `"intent_id"` のバイトが埋まっている箇所は 1 件も見つからなかった。** 実測した手順:

| 検査 | コマンド | 結果 |
|---|---|---|
| ゴールデンに旧名 | `grep -rln "WorkflowExecution" tests/` | **0 件** |
| ゴールデンに `intent_id` | `grep -rln "intent_id" tests/` | **0 件** |
| Rust の文字列リテラル中の旧名 | `grep -rn '"WorkflowExecution"' modules/ --include='*.rs'` | 2 件 — どちらも `AGGREGATE_TYPE_NAME` の定義と、その値を固定する `type_name()` のアサート。**改名一族の表が明示的に改名対象としているもの**であり、固定フィクスチャではない |
| serde フィールド名の逐語アサート | `grep -rn '"intent_id"' modules/ --include='*.rs'` | **0 件** |
| SQL / JSON リテラルへの埋め込み | `intent_id` を含む行のうち `INSERT` / `SELECT` / `json` / `payload` / `{"` を含むもの | **0 件** |
| `formal/**`（Quint） | `grep -rln "WorkflowExecution\|intent_id" formal/` | 1 ファイル（§6 (1) — モデルと Rust の対応を書いた**コメントのみ**。モデル本体は Rust 型名を参照していない） |

生 SQL を書いている 2 つのテスト（`crash_reconstruction_test.rs` の書きかけ行注入、
`intent_repository_impl_test.rs` の `rewind_snapshot_to_genesis`）も確認したが、埋め込まれて
いるのは `'workflow-execution-event/1'`（= 据え置いた `EVENT_MANIFEST` の値）だけで、
スナップショット payload は `genesis_payload()` が**実行時に生きたストアから採る**形であり
固定バイトではない。

---

## 4. 受入基準の実行結果（すべて実測）

| # | 基準 | コマンド | 結果 |
|---|---|---|---|
| 1 | `cargo fmt --all --check` | `CARGO_TARGET_DIR=$PWD/target-delegate cargo fmt --all --check` | **緑**（exit 0） |
| 2 | `cargo clippy --workspace --all-targets -- -D warnings` | 同上 | **緑**（exit 0） |
| 3 | `cargo lint` | `CARGO_TARGET_DIR=$PWD/target-delegate-lint cargo lint` | **緑**（exit 0） |
| 4 | `cargo test --workspace`（退行 0） | 同上 | **緑**。**774 passed / 0 failed**。origin/main も 774 なので**増減なし・退行なし** |
| 5 | `scripts/quint-gate.sh` | 同上 | **緑**（exit 0） |
| 6 | `scripts/coverage.sh`（相対） | `… --base origin/main` | **緑**。head 98.52387%。絶対 `[PASS] >= 90.0%`、相対 `[PASS] head (98.5238740774213%) >= base (98.52600074822296%) - tolerance (0.01)`。head は base を **0.00213pp 下回る**が許容誤差 0.01 の内側で PASS（再訂正前の計測は head 98.53074% で base 超えだった — `build()` を写し返しへ戻し、ビルダーを `#[cfg(test)]` で絞ったぶん計測対象行が変わっている） |
| 7 | プロダクトコードに `unwrap` / `expect` 0 件 | clippy（`unwrap_used` / `expect_used` deny）+ 改名 6 ファイルの `#[cfg(test)]` 前を対象にした grep | **緑**（各ファイル 0 件） |
| 8 | 外形不変 | `git diff --name-only origin/main..HEAD -- tests/` + 投影ゴールデンの実行 | **緑**。差分 0、19 本無改変で全緑（§2） |
| 9 | `grep -rn "WorkflowExecution" modules/ --include='*.rs'` | 同左 | **緑**。**0 件**。訂正後の旧名（`IntentBuilder` / `IntentState` / `IntentStateBuilder`）も **0 件** |
| 10 | `git log --follow` でファイル履歴が追える | `git log --follow -- <各ファイル>` | **緑**。`intent.rs` 14 件・`intent_snapshot.rs` 7 件・`intent_repository_impl.rs` 6 件の履歴が改名をまたいで追える |
| 11 | 報告書 | 本ファイル | **完了** |

---

## 5. 判断が要った点

### (a) `EVENT_MANIFEST` の値は据え置いた

`EVENT_MANIFEST = "workflow-execution-event/1"` は、改名すれば `intent-event/1` 相当になる
綴りである。**据え置いた**理由は 2 つ:

1. **改名一族の表に無い。** 表は文字列値の改名を 1 つだけ（`AGGREGATE_TYPE_NAME`）明示して
   おり、この非対称は意図的と読める。
2. **doc 自身が逐語固定を宣言している。** 定数のテストに
   「綴りは行に書かれて残る値である — 変えると既存行が読めなくなるので逐語で固定する」と
   書かれており、値の変更は既存ジャーナル行を `Corrupt` にする破壊的変更である。

受入基準 9 とは衝突しない（`workflow-execution-event` は小文字・ハイフンなので
`WorkflowExecution` に部分一致しない）。doc 中の**型名**（`WorkflowExecutionEvent` /
`…RepositoryImpl`）は追随させた。

**判断を仰ぎたい**: 値も `intent-event/1` へ揃えるべきなら、別 Bolt での実施を推奨する
（永続化形式の変更であり、改名とは変更理由が違う）。

### (b) `IntentSnapshot` の降格が波及した範囲

ブリーフ更新で入った「クレート内私有へ降格」は、前提（ドメイン外の使用ゼロ）を自分で実測して
確認したうえで実施した — `WorkflowExecutionState` の型名はドメインクレート外に **0 件**、
使用は `intent.rs` 27 / `intent_snapshot.rs` 26 / `mod.rs` 2 のみ。

降格すると `pub` の面がコンパイラに拾われるので、次が連動した:

1. **`Intent::state` / `Intent::from_state` も `pub(crate)` へ。** 公開メソッドが私有型を
   返す／取ると `private_interfaces` に落ちる。出口は集約の `Serialize` / `Deserialize` だけに
   なった。
2. **memento のアクセサ 16 本を削除した。** 降格後は誰も呼ばず（`unreachable_pub` 16 件 +
   「never used」警告）、外へ出さない型に読取面を二重化する理由が無い
   （`no-backward-compatibility.md`）。同一クレートのテストは `pub(crate)` フィールドを直接読む。
3. **ドメイン外 5 箇所の `aggregate().state()` どうしの比較を、集約そのものの比較へ**
   置き換えた（`Intent: Eq`）。`interface-adapter/tests/intent_repository_impl_test.rs` ×2、
   `interface-adapter/tests/support/contract.rs`、`app/aidlc/tests/crash_reconstruction_test.rs`。
   memento 越しに覗く必要がそもそも無かった箇所であり、降格の副作用というより是正である。
4. **`intent_snapshot.rs` のテスト 6 本を集約の面から書き直した。** memento のアクセサが
   無くなったので、既定値・上書き・拒否のいずれも `Intent` の公開クエリで観測する。
   再訂正で `build()` が写しを返す形に戻ったため、テスト内のヘルパ
   `built(builder) -> Result<Intent, StateError>`（＝ `Intent::from_state(builder.build())`）を
   1 本置き、観測面は集約のままにした。`plan` / `conditional` が stages と食い違う場合と
   空ステージ列のケースは、以前は「ビルダーは検証しない」ことの確認だったが、いまは
   **集約が拒否する**ことの確認になった（`StateError::InvariantViolation` の逐語も固定）。
   テスト本数は 6 本のまま。

### (c) `JournalEntry::intent_id()` は据え置いた

RMU の `JournalEntry` にも `intent_id` フィールドと同名アクセサがあるが、これは**読取行が
運ぶ識別子**であって集約のアクセサではない。改名一族の表は「集約内のフィールド」と限定して
いるので対象外とした。結果、`.intent_id()` の呼出は RMU 側の 5 箇所だけが残っている
（`entry` / `row` / `rows[0]` をレシーバに持つもの）。集約をレシーバに持つ **12 箇所**は
`.id()` へ移した（ドメイン内 3・ドメイン外 9）。

### (d) `IntentSnapshotBuilder` を `#[cfg(test)]` でも絞った

再訂正の「可視性は写しと同じくクレート内私有」に従って `pub(crate)` へ降格したところ、
`dead_code`（`-D warnings`）で 2 件の赤が出た — **クレート内にこのビルダーの利用箇所が
テストしか無い**ためである。本番経路の birth は `Intent::start` が集約を直に起こすので、
任意の状態から写しを組む必要があるのはテストだけ、という実態が可視性を絞ったことで
表に出た形になる。

`#[allow(dead_code)]` で黙らせるのではなく、構造体と `impl` に `#[cfg(test)]` を付けて
**テストビルドだけに存在させた**。同リポジトリに前例がある:

- `modules/core/command/use-case/src/orchestration/mod.rs:21` — `#[cfg(test)] mod test_support;`
- `modules/core/infrastructure/src/canon_json/value.rs:307` — `#[cfg(test)] pub(crate) mod arbitrary`

ドメインクレートの結合テスト（`domain/tests/` の ITF 準拠 2 本）はビルダーを使っていないので
（実測 0 件）、`cfg(test)` で絞っても届かなくなる利用者はいない。

### (e) `StateError` は改名しなかった — `SnapshotError` は「別の型」ではなく旧名だった

訂正 4 は「既存の `SnapshotError` という型が**別に存在する**場合は衝突するので止めて報告」
という条件だった。実測すると:

- `.rs` 全域に `SnapshotError` は **0 件** — 型としては存在しない（＝文字どおりの衝突は無い）
- しかし `.md` に **31 件**あり、その正体は**この型自身の旧名**だった。
  `u3-event-store-repository/functional-design/entities.md:126` に
  「Builder は `WorkflowExecutionStateBuilder`、エラーは `StateError`（**旧 SnapshotError**）。
  旧名の再エクスポート・型エイリアスは残さない」と記録されている。写しの型名も同じく
  `WorkflowExecutionSnapshot` → `WorkflowExecutionState`（B5 改名）だった。

つまり B12 の訂正は、B5 で Snapshot → State へ動かした型名を Snapshot 側へ戻す変更であり、
`StateError` → `SnapshotError` はその対称な巻き戻しに当たる。**衝突ではないが、過去の裁定を
巻き戻す判断**である。また訂正 2・3 で `state()` / `from_state()` の名前は据え置くと明示された
ので、`StateError` は `from_state` と綴りが対応したままでもある。

以上から**据え置き、裁定を仰ぐ**ことにした（「上流成果物の間に矛盾を見つけたら読み替えて
進まず人間へ裁定を求める」に従った）。改名するなら `entities.md` の当該行も同時に改める
必要があるが、`aidlc/**` の設計文書はブリーフの所有ファイル外なので触っていない。

---

## 6. 申し送り

1. **`formal/orchestration/journal_protocol.qnt` の対応表コメントが古くなった。**
   冒頭に「`snapVersion` ↔ `WorkflowExecution::version()`」「`load(w)` ↔
   `WorkflowExecutionRepository::find_by_id`」等、モデルと Rust 名の対応を書いた**コメントが
   5 行**ある。モデル本体は Rust 型名を参照していないので Quint ゲートは緑のままだが、
   読み手には失効した名前が見える。`formal/**` は本 Bolt の対象外なので触っていない。
2. **`IntentSnapshot` のフィールドは `pub(crate)` のまま**である。`field-visibility.md` は
   `pub(crate)` フィールドも禁じているが、同規則の §射程 は「ワイヤ表現・外部形式の DTO」を
   対象外としており、クレート内私有の serde memento はこれに当たると読んだ。機械化ロードマップ
   の 1・2（構造体リテラルの一元化と `no-public-fields` の境界拡張）が来たときに、この型を
   どう扱うかを改めて決める必要がある。
3. **`EVENT_MANIFEST` の値** — §5 (a)。
4. **`StateError` を `SnapshotError` へ戻すか** — §5 (e)。戻す場合は
   `u3-event-store-repository/functional-design/entities.md:126` の記述（旧名を残さないと
   書いた B5 の裁定）も同時に改める必要がある。
5. **`docs/**` と `coding-rules/**` の旧名**はメインセッションの担当（ブリーフどおり触って
   いない）。`use-case-rules.md` / `gateway-taxonomy.md` / `good-examples.md` 等が
   `WorkflowExecutionRepository` を適用例として挙げている。
6. **`target-delegate/` と `target-delegate-lint/`** が未追跡のまま残っている
   （`.gitignore` は `/target` しか除外していない）。検証用のビルド生成物なので削除してよい。
7. **メインセッション側の未コミット変更が作業ツリーに残っている** — `brief-1.md`（再訂正の
   反映）と `coding-rules/` 6 ファイル（`cqrs-boundaries` / `factory-naming` /
   `gateway-taxonomy` / `good-examples` / `ubiquitous-language` / `use-case-rules`）。
   自分の所有ファイルではないので**コミットしていない**。取り込みの要否はそちらで判断を。

---

## 7. コミット

意味単位で 4 本に分けた（いずれも `b12: ` 接頭辞、`git add` は明示パス、**push なし**）。

| コミット | 内容 |
|---|---|
| `e27cfd8` | 集約 `WorkflowExecution` を `Intent` へ改名する（一族・ファイル名・`type_name`）。`IntentSnapshot` の降格を含む |
| `40c90b9` | 集約のフィールド `intent_id` とアクセサを `id` / `id()` へ改める |
| `75ad323` | 委任ブリーフと開発者報告を記録する |
| `033f898` | 写しのビルダーを `IntentSnapshotBuilder` へ改め、クレート内私有に絞る（再訂正の反映。`build()` の `Result<Intent, _>` 化を巻き戻し） |

分けたのは、1 本目が「型と面の名前」、2 本目が「集約内の属性名」、4 本目が「再訂正への追随」で
変更理由が違うためである。`e27cfd8` / `40c90b9` の時点でも `cargo test --workspace` は 774 全緑で、
どのコミットも単体でビルドが通る。
