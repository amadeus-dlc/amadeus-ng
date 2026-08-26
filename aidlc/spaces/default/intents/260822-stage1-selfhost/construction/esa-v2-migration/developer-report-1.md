# developer-report-1 — 委任 1: ドメインの Conformist 化（event-store-adapter-rs v2.0.0）

Conversation language: 日本語。ブランチ `bolt/b6-esa-v2-conformist`。**未コミット**（`git add` /
`commit` / `push` は行っていない）。

## 1. 一文サマリ

`WorkflowExecution` / `WorkflowExecutionEvent` / `IntentId` が本家 v2.0.0 の
`Aggregate` / `Event` / `AggregateId` を**直接実装**する状態にし（腐敗防止層なし）、
本家 memory バックエンドへの実 persist / snapshot + replay 復元まで通した。自前
`EventStoreImpl` は共存させたまま、検査 6 種すべて緑（テスト **689** 全緑、
ベースライン 674 + 新規 15）。

## 2. 本家 v2.0.0 の trait 定義（実測）

`gh api repos/j5ik2o/event-store-adapter-rs/contents/lib/src/types.rs?ref=v2.0.0` で
取得した**実物**。プロンプトの要約ではなくこれに従った。

```rust
pub trait AggregateId:
  Display + Debug + Clone + Serialize + for<'de> Deserialize<'de> + Send + Sync + 'static {
  fn type_name(&self) -> String;
  fn value(&self) -> String;
}

pub trait Event: Debug + Clone + Serialize + for<'de> Deserialize<'de> + Send + Sync + 'static {
  type ID: Display;                 // ← 境界は Display **のみ**
  type AggregateID: AggregateId;
  fn id(&self) -> &Self::ID;
  fn aggregate_id(&self) -> &Self::AggregateID;
  fn seq_nr(&self) -> usize;
  fn occurred_at(&self) -> &DateTime<Utc>;
  fn is_created(&self) -> bool;
}

pub trait Aggregate: Debug + Clone + Serialize + for<'de> Deserialize<'de> + Send + Sync + 'static {
  type ID: AggregateId;
  fn id(&self) -> &Self::ID;
  fn seq_nr(&self) -> usize;
  fn version(&self) -> usize;
  fn set_version(&mut self, version: usize);   // ← setter（`&mut self`）
  fn last_updated_at(&self) -> &DateTime<Utc>;
}
```

`EventStore` は `#[async_trait]`（`Send + Sync + 'static` 境界つき）。エラーは
`EventStoreWriteError` / `EventStoreReadError` の **2 種**。

**実測でわかった意味論**（`generic_event_store.rs` / `event_store_for_memory.rs`）:

| 事項 | 本家 | 我々（現状） |
| --- | --- | --- |
| `get_events_by_id_since_seq_nr(aid, n)` | `event.seq_nr() >= n`（**その番号を含む**） | ローカル `EventStore` は「`n` **より後**」 |
| genesis の判定 | `Event::is_created()` が真 → `create_*` 経路。`persist_event` は creation を**拒否**する | `seq_nr == 1` / `expected_version == 0` で判定 |
| 楽観 version | ストアが `expected_version + 1` を採番し、格納する集約へ `set_version` する | `version = 永続化済みの最後の seq_nr` |
| genesis の version | 本家サンプル（`event_store_test_support.rs`）は **1** から始まる | **0** から始まる |

いずれも**委任 2 の裁定事項**として §7 に設計質問で挙げた。本委任では触っていない。

## 3. 変更の before / after

### 3.1 依存（`Cargo.toml`）

```toml
event-store-adapter-rs = "=2.0.0"   # 完全固定（ADR-010 決定 4）。feature は既定（空）
chrono = { version = "0.4", default-features = false, features = ["std","clock","serde"] }
```

- `core-domain`: `event-store-adapter-rs` / `chrono` / `serde` を**通常依存**に追加
  （Conformist なのでドメインが本家 trait を実装する — ADR-010 決定 2）。dev に `tokio`。
- `core-interface-adapter`: `chrono` / `event-store-adapter-rs` を通常依存に追加。
- `core-use-case`: `chrono` / `event-store-adapter-rs` は **dev-dependency のみ**。
  ポート定義自体は本家に依存させていない（依存が入るのは委任 2）。

### 3.2 `WorkflowExecutionEvent`

| before | after |
| --- | --- |
| `intent_id: IntentId` / `seq_nr: u64` を封筒が直接持つ | `id: WorkflowExecutionEventId`（集約 ID と `seq_nr` の組）1 本に集約 |
| `occurred_at: String`（ISO 8601） | `occurred_at: DateTime<Utc>` |
| 固有メソッド `intent_id()` / `seq_nr()` / `occurred_at()` | `Event` の `aggregate_id()` / `seq_nr()` / `occurred_at()`（**固有版は削除** — 二重口を作らない） |
| serde なし | 封筒・ペイロード 12 変種すべてに `Serialize` / `Deserialize` |
| — | `id()` / `is_created()` を新設 |

`schema_version()` / `payload()` は本家契約の外なので固有メソッドのまま残した。

### 3.3 `WorkflowExecutionEventId`（新規 — `workflow_execution_event_id.rs`）

`Event::ID` の境界は `Display` **だけ**なので、ドメイン語の値オブジェクトにした。

- 中身は `(IntentId, seq_nr)` の組。`Display` は `<intent_id>#<seq_nr>`
  （`#` は `IntentId` の正準表記＝16 進小文字とハイフンに現れないので組へ一意に戻せる）。
- **採番は決定的**（§4.1 に判断の根拠）。

### 3.4 `WorkflowExecution`

| before | after |
| --- | --- |
| `seq_nr: u64` / `version: u64` | `usize` / `usize` |
| — | `last_updated_at: DateTime<Utc>` を新設（`apply_event` が適用イベントの `occurred_at` で更新、genesis は `Started` の `occurred_at`） |
| 固有 `intent_id()` / `seq_nr()` / `version()` | `Aggregate` の `id()` / `seq_nr()` / `version()`（固有版は削除） |
| `with_version(self, u64) -> Self` | **削除**し、本家の `set_version(&mut self, usize)` に一本化 |
| `occurred_at: &str` を受ける 12 コマンド | `occurred_at: DateTime<Utc>`（Copy なので値渡し） |
| serde なし | `Serialize` / `Deserialize`（**private フィールドのまま derive**。ADR-002 の FSM 規律は不変 — 公開 API は decide → 1 イベント → apply のまま） |

### 3.5 `IntentId`

`AggregateId` を実装。`type_name()` は `"WorkflowExecution"`（定数）、`value()` は生文字列。
`value()` は `tell-dont-ask.md` が禁じる綴りだが、**外部 trait の実装は Published Language への
準拠**であり例外として doc コメントに理由を明記した（`ubiquitous-language.md` の例外作法）。
既存の `as_str()` は残る — `value()` は `String` を返す本家の口、`as_str()` は借用を返す我々の口で、
用途も戻り型も違う（§7 の設計質問 D で扱いを問う）。

### 3.6 serde を入れたドメイン型と、Always Valid の保全

- **文字列 newtype**（`IntentId` / `StageSlug` / `WorkflowDefinitionId` /
  `DefinitionRevision`）は `#[serde(try_from = "String")]` + `TryFrom<String>` にした。
  復号が `parse` と**同じ検査**を通るので、「不正値はこの型に存在しない」という
  doc の主張が serde 経路でも破れない。`Serialize` は newtype なので生文字列へ落ちる。
- **列挙・単純構造体**（`PhaseId` / `PlanAction` / `Status` / `CheckboxState` /
  `AutonomyMode` / `JumpDirection` / `StageIndex` / `StageEntry` / `PhaseBoundary`）は素の derive。
- 付随して `StageSlugError` に `Display` + `std::error::Error` を実装した
  （`try_from` のエラー型は `Display` 必須。他 2 つの newtype は既に持っていた）。材料のみを
  描く既存の様式に合わせ、赤例テストも足した。

### 3.7 `usize` 化の波及（全箇所）

| 面 | 変更 |
| --- | --- |
| `ApplyError::SequenceGap { expected, actual }` | `u64` → `usize` |
| `EventStoreError::Conflict { expected, actual }` / `Corrupt { seq_nr }` | `u64` → `usize` |
| `RepositoryError::Conflict` / `Corrupt { seq_nr }` | 同上 |
| ローカル `EventStore` ポート | `persist_event(_, version: usize)` / `get_events_by_id_since_seq_nr(_, seq_nr: usize)` |
| `WorkflowExecutionState` / そのビルダー | `seq_nr` / `version` を `usize` へ |
| `EventPayloadWire::encode/decode` | `seq_nr: usize` |
| `StateWire` | `exact_integer` は JSON の値域検査なので `u64` のまま。境界で `to_u64(usize)` に変換 |
| ITF 準拠テスト（`journal_protocol_conformance.rs`） | `snapVersion` / `snapSeq` / `loadedVersion` を `usize` へ。`journalLen` / `checkpoint` / `readModelSeq` は**ジャーナル面なので `u64` のまま** |

**ローカル `EventStore` ポートの `u64` → `usize` は「共存のための最小修正」ではなく是正**である。
ADR-010 / `upstream-contracts.md` が名指しした違反（借り物の契約をドメインに合わせて書き換えた）
そのものであり、直さないと本委任の目的が達成されない。削除・差し替えはしていない。

### 3.8 `to_u64` アクセサの扱い（実測にもとづく判断）

**`GlobalSeqNr::to_u64` は `u64` のまま残した。** 理由:

- `GlobalSeqNr` は**ジャーナル行の全集約横断通番**（C6 `journal.global_seq_nr` =
  `INTEGER PRIMARY KEY AUTOINCREMENT`）であり、**本家の契約には存在しない我々固有の概念**
  （ADR-010 が「本家に無いものは我々が持ち続ける」と明記）。集約内の `seq_nr` とは別物で、
  `usize` 化の対象ではない。
- `StageIndex::to_usize` も従来どおり（元から `usize`）。

`event_store_impl.rs` の変換ヘルパは次のように整理した（すべて境界での変換であり、契約の
書き換えではない — `upstream-contracts.md` §境界での変換、C-CONV の綴り）:

- `to_i64<T: TryInto<i64>>(..)` — ドメインの符号なし整数（`usize` / `u64` 両方）→ SQLite `INTEGER`
- `to_u64(i64, ..)` — SQLite `INTEGER` → **global 通番**
- `to_usize(i64, ..)` — SQLite `INTEGER` → **集約内の `seq_nr` / `version`**（新設）

### 3.9 時刻の波及

- `Clock` trait: `now_ms(&self) -> u64` → `now(&self) -> DateTime<Utc>`。
- `SystemClock`: `SystemTime::now()` → `Utc::now()`。
- `FakeClock`: `AtomicU64` → `Cell<DateTime<Utc>>`。`Mutex` にしなかったのは施錠失敗という
  panic 経路（`expect_used` deny）を作らないため。`&self` の裏の `Cell` は
  `interior-mutability.md` の既定に対する例外なので、doc コメントに理由を明記した
  （従来の `AtomicU64` と同じ既存の例外枠、テスト専用の実装に閉じている）。
- **自前の ISO 8601 整形を撤去した**: `format_iso8601_utc(epoch_ms)` と
  `civil_from_days`（Howard Hinnant の暦計算 18 行）を削除し、
  `DateTime::to_rfc3339_opts(SecondsFormat::Secs, true)` に置き換えた。ADR-010 が予告した
  NFR4.1 の再検討点。**出力の逐語形は不変**（`YYYY-MM-DDTHH:MM:SSZ`、秒精度、`Z` サフィックス）で、
  閏日・年境界・秒未満切り捨ての固定テストはそのまま残し、逆向きの
  `parse_iso8601_utc` の往復テストを足した。
- **upstream 互換の文字列面は逐語維持**: `journal.occurred_at` 列・`snapshot.updated_at` 列は
  `DateTime` からの変換で作る。行の読み出しは逆変換で `DateTime<Utc>` に戻し、
  戻せない綴りは `Corrupt(UndecodablePayload)` にする（黙って既定値にしない）。

### 3.10 `WorkflowExecutionState` が 16 属性 → **17 属性**（記述と実態の食い違い、§6 参照）

`last_updated_at` は集約の状態であり、`from_state` が復元できないと
スナップショット往復で集約が一致しなくなる（`crash_reconstruction_test` などが落ちる）。
そこで memento に 17 番目の属性として足し、`StateWire` の閉集合キーも 17 本にした
（`"last_updated_at"` — 値は upstream 互換の秒精度 ISO 8601 文字列）。C6 の
`snapshot.payload` の中身が 1 キー増える。**列構成（3 表のスキーマ）は不変**。

## 4. 判断（ブリーフが報告を求めた 3 点 + α）

### 4.1 イベント ID の採番方式 — **決定的採番（集約 ID + `seq_nr`）**

本家の `type ID` の境界は `Display` **のみ**（実測）。ULID / UUID を選ぶ理由は無く、
むしろ次の 2 点から決定的採番が正しいと判断した。

1. **集約は時計も乱数も持たない**（NFR3.1 / ADR-002）。ULID 生成は時刻源を必要とし、
   イベント生成経路に非決定性を持ち込む（ITF 準拠テスト・PBT・ゴールデンの決定性が崩れる）。
2. **一意性は既に保証されている**。ジャーナルの `UNIQUE (aggregate_id, seq_nr)` が
   この組の一意性そのものであり、別立ての採番を足しても一意性は増えない。

副産物として、封筒から `intent_id` / `seq_nr` の重複フィールドが消えた（ID が唯一の正本）。

### 4.2 `last_updated_at` の維持規則 — **適用したイベントの `occurred_at`**

`apply_event` が `seq_nr` を進めるのと同じ 1 行で `last_updated_at = *event.occurred_at()` を
置く。したがって:

- 通常実行とリプレイで**必ず同じ値**になる（BR2.3 — 同一経路）。
- 集約は時計を読まない（NFR3.1）。値は常に呼出側が渡した時刻に由来する。
- genesis は `Started` の `occurred_at`。
- ガード不成立の `Err` では動かない（`apply_event` は一時コピーに適用して成功時だけ差し替える）。

`WorkflowExecutionStateBuilder` の既定は Unix epoch（birth 時の発生時刻を知っているのは
`Started` を作った側だけなので、ビルダーは既定値を持ち、呼出側が `.last_updated_at(..)` で置く）。

### 4.3 `to_u64` の扱い — §3.8 のとおり `GlobalSeqNr` は `u64` 据え置き

### 4.4 `with_version` の削除

`with_version` と `set_version` を並立させると `no-backward-compatibility.md` の
「互換口の並立」に当たるので、`with_version` を消して呼出側 5 箇所を `set_version` に直した。

## 5. 「触ってはいけないもの」への最小修正（全列挙）

削除・差し替えはしていない。共存のために必要だった変更だけを挙げる。

| ファイル | 修正 | 必要だった理由 |
| --- | --- | --- |
| `use-case/orchestration/event_store.rs` | ポートの `u64` → `usize`、doc の「u64 へ具体化した」記述を撤回に書き換え、`FakeStore` の型追従 | ADR-010 が名指しした契約書き換えの是正。ドメインの `seq_nr` が `usize` になったので型が合わない |
| `use-case/orchestration/event_store_error.rs` / `repository_error.rs` | `Conflict{expected,actual}` / `Corrupt{seq_nr}` を `usize` へ | 同上 |
| `use-case/orchestration/journal_reader.rs` | テストの封筒生成を `DateTime<Utc>` へ | 署名変更の追従のみ（ポートの形は不変） |
| `interface-adapter/.../event_store_impl.rs` | ①`event.intent_id()` → `aggregate_id()` ②`with_version` → `set_version` ③`u64`/`usize` 変換ヘルパの整理 ④自前 ISO 8601 整形を chrono へ ⑤行復号で `occurred_at` を `DateTime` に戻す ⑥`JournalRow`/`SnapshotRow` の `seq_nr`/`version` を `usize` へ | 削除メソッドの追従、時計の型変更、`usize` 化 |
| `interface-adapter/.../schema.rs` | **無修正**（列定義は変わらない） | — |
| `interface-adapter/.../memory/in_memory_event_store.rs` | `event_store_impl.rs` と同種の追従（SQLite 実装と同形を保つため） | 契約テストが両実装を通るため片方だけ直せない |
| `interface-adapter/.../workflow_execution_repository_impl.rs` / `memory/workflow_execution_repository.rs` | `intent_id()` → `aggregate_id()` / `id()`、`with_version` → `set_version` | 削除メソッドの追従 |
| `interface-adapter/.../wire/{mod,event_wire,state_wire}.rs` | `seq_nr: usize`、`StateWire` に 17 番目のキー | `usize` 化と `last_updated_at` の往復 |
| `interface-adapter/src/clock.rs` | chrono ベースへ全面書き換え | ブリーフ項目 6 |

`docs/**` / `.claude/**` / `scripts/**` / `formal/**` / `.coderabbit.yaml` は**一切触っていない**。
Quint モデルのアクション名・ITF フィクスチャ内の文字列も**一切触っていない**（一括置換は
テストモジュール本体に限定し、`AT` → `at()` の置換では `occurred_at` / `parked_at` などの
巻き添えを個別に復元して確認した。`formal/` と `**/fixtures/` は grep で無変更を確認）。

## 6. 記述が実態と食い違う箇所（コンダクタへの申し送り）

**コードは直したが、設計文書は触っていない**（同期はコンダクタ担当）。

| # | 文書 | 現在の記述 | 実態 |
| --- | --- | --- | --- |
| 1 | C3（契約設計） | ローカル `EventStore` の数値は `u64` | **`usize`**（本家と同形へ復帰）。ADR-010 が撤回を明記済みだが C3 本文が未追従 |
| 2 | C5（イベント封筒） | 封筒は `intent_id` / `seq_nr` / `schema_version` / `occurred_at` | ドメイン型の封筒は `id`（＝前 2 者の組）/ `schema_version` / `occurred_at`。**ジャーナル行の列構成は不変**（C5 のワイヤ契約は変わっていない） |
| 3 | C6（状態の写し） | 「全状態 **16 属性**」 | **17 属性**（`last_updated_at` 追加）。`snapshot.payload` の JSON キーが 16 → 17。**表・列の定義は不変** |
| 4 | entities.md | `WorkflowExecutionEvent` の材料一覧にイベント ID が無い | `WorkflowExecutionEventId` が新設された（Domain Primitive） |
| 5 | NFR4.1（依存最小化） | 自前 ISO 8601 整形で日付クレートを避ける | **chrono を採用**し自前整形を撤去（ADR-010 が再検討を明記）。依存は §8 の実測 |
| 6 | BR5.2 / ドメイン層 doc | 「ドメインは serde を知らない」 | 集約・ドメインイベント・集約識別子は serde を持つ（本家 trait の境界要求）。**観測互換のワイヤ形式はアダプタ層のまま**なので BR5.2 の趣旨は保たれている。ソース側の doc コメントは本委任で修正済み |
| 7 | ローカル `EventStore` の doc | `get_events_by_id_since_seq_nr` は「指定 `seq_nr` **より後**」 | 本家は「**その番号を含む**」。ポートを消す委任 2 で境界が 1 ずれる（テストで確認済み） |
| 8 | ADR-006 由来の記述全般 | 「本家に依存しない」 | ADR-010 で撤回済み。残存箇所の掃除が要る |

## 7. 設計質問（**委任 2 の前に裁定が要るもの**）

### A. `Deserialize` が集約不変条件を検査しない（**重要**）

ブリーフの指示どおり `WorkflowExecution` に serde を **derive** した。結果として
**復号は `from_state` の検査点を迂回する**。実測で確認した（一時的な probe テストで
検証し、コミット対象からは外した）:

- 3 ステージの集約を直列化し、JSON の `"cursor":0` を `"cursor":99` に書き換えると
  **`from_str` は `Ok` を返す**（`cursor_in_scope` などの不変条件は検査されない）。
- panic はしない（範囲外の索引はクエリが `None` を返す設計 — NFR4.3）。しかし
  「不変条件を満たす集約しか存在しない」という保証は serde 経路では成立しない。

いまは実害が無い（自前ストアは `StateWire` → `from_state` を通す）。**委任 2 で本家の
SQLite バックエンドが唯一の復元経路になると、security-design §2 の検査点 3
（集約不変条件）が消える。** 選択肢:

| 案 | 中身 | 評価 |
| --- | --- | --- |
| (a) 受け入れる | 「ストアに入っているのは自分が書いた行だから正しい」と割り切る | 最小。ただし DB 破損・手編集・スキーマ移行の失敗が静かに通る |
| (b) memento 経由にする | `#[serde(into = "WorkflowExecutionState", try_from = "WorkflowExecutionState")]` にして `state()` / `from_state()` を serde 経路にも通す | 検査点が復活し、既存コードを再利用できる。`WorkflowExecutionState` に serde derive が要る。**これが筋に見える** |
| (c) 復元後に検査 | Repository 側で復元直後に検査メソッドを呼ぶ | 呼び忘れが効く。型で守れない |

（b）は本委任のブリーフ（「serde は private フィールドのまま derive できる」）を超えるので、
**勝手に決めず止めた**。

### B. genesis の楽観 version — 本家は 1、我々は 0

本家サンプルは genesis の集約を `version = 1` で作り、以後ストアが +1 する。我々は
`version = 0` から始め、`version = 永続化済みの最後の seq_nr` という別の規則で運用している
（BR5.3）。memory バックエンドでは**どちらでも動く**ことを確認済み（本委任の証明テストは
0 始まりのまま通る）が、委任 2 で本家 SQLite バックエンドに載せると、`version` を採番するのは
本家になるので **BR5.3 の「version = 最後の seq_nr」は成立しなくなる**。

裁定が要る: BR5.3 を「version はストアが採番する不透明な楽観トークン」に改めるか、
genesis を 1 始まりに合わせるか。

### C. `events_after(GlobalSeqNr)` / チェックポイントの置き場所

ADR-010 は「SQLite を使う範疇で amadeus-ng が独自に実装する」と決めているが、本家の
`EventStoreForSqlite` は `Connection` を露出しない（ADR-010 が調査済み）。本委任では
`JournalReader` に手を付けていない。委任 2 で「同一 DB ファイルへの別接続」を開く実装に
なる想定だが、その接続の所有者（Repository と別か、合成ルートが両方持つか）は未決。

### D. `IntentId::value()` と `as_str()` の並立

本家の `value()`（`String` を返す）を実装したことで、我々の `as_str()`（`&str` を返す）と
2 つの読み出し口が並ぶ。戻り型も用途も違うので `no-backward-compatibility.md` の
「互換口の並立」には当たらないと判断したが、`tell-dont-ask.md` の「アクセサを `value()` と
名乗らない」規則との関係は正本に例外として明記したほうがよい（`upstream-contracts.md` に
「外部 trait の実装は例外」と一行足す形）。**正本ファイルは触っていない。**

### E. `thiserror` が推移依存に入った

`error-handling.md` は「thiserror / anyhow 不使用」と定めている。本家 v2.0.0 が
`thiserror` を使っているため**推移依存として入る**（我々のコードでは使っていない）。
Conformist を採る以上そうなるので、規則の射程を「我々が書くエラー型」に限る旨を
正本へ書き足すのが筋に見える。**正本ファイルは触っていない。**

## 8. 依存の推移実測（`cargo tree`）

`Cargo.lock` に**新規 20 パッケージ**。うち本ホスト（macOS / Linux）で**実際に
コンパイルされるのは 8**。残り 12 は wasm / windows / android / haiku へ target-gate された
`chrono` → `iana-time-zone` の分岐で、この構成ではビルドされない。

**実際に入るもの**（`cargo tree -e normal --workspace` 実測）:

```
core-domain
├── chrono v0.4.45
│   ├── iana-time-zone v0.1.65 → core-foundation-sys v0.8.7   (macOS のみ)
│   ├── num-traits v0.2.19        (既存)
│   └── serde v1.0.229            (既存)
└── event-store-adapter-rs v2.0.0
    ├── async-trait v0.1.92 (proc-macro)
    ├── chrono v0.4.45            (同上)
    ├── serde v1.0.229            (既存)
    ├── serde_json v1.0.151       (既存)
    ├── thiserror v2.0.20 (+ thiserror-impl)   (既存 — rusqlite 経由で既に入っていた)
    └── tracing v0.1.44 (+ tracing-attributes, tracing-core)
```

**純増（このホストでビルドされるもの）**: `event-store-adapter-rs`, `chrono`,
`iana-time-zone`, `core-foundation-sys`, `async-trait`, `tracing`, `tracing-attributes`,
`tracing-core`。`serde` / `serde_json` / `thiserror` / `num-traits` / `once_cell` /
`pin-project-lite` / `syn` は既存（新規ではない）。

**target-gate されてビルドされないもの**: `android_system_properties`,
`futures-core` / `futures-task` / `futures-util` / `slab`（wasm の `js-sys` 経由）,
`iana-time-zone-haiku`, `log`, `windows-core` / `-implement` / `-interface` / `-result` /
`-strings`。

`sqlite` feature は**有効にしていない**（本委任では memory バックエンドしか使わない）ので、
本家経由の `rusqlite` は入っていない（我々の `rusqlite` は従来どおり自前ストアのもの）。

## 9. テスト（TDD）

シグネチャ変更はすべて**テストを先に新形へ書き換えて Red を実測**してから実装した。
記録した代表的な Red:

- `IntentId`: `type_name()` / `value()` / serde 往復 → `E0599 no method named ...` /
  `E0425 cannot find type DateTime`
- 封筒: `aggregate_id()` / `id()` / `is_created()` / `occurred_at() -> &DateTime` →
  `E0599 no method named 'aggregate_id' / 'id' / 'is_created'`（4 種 10 件）
- 集約: `set_version` / `Aggregate` 実装 → `E0599 no method named 'with_version'` ほか

**新規テスト 15 本**（674 → 689）:

| 場所 | 本数 | 内容 |
| --- | --- | --- |
| `domain/tests/upstream_event_store_conformance.rs`（新規） | 4 | 本家 memory バックエンドへの実 persist / 復元 |
| `intent_id.rs` | 2 | `AggregateId` 契約、serde 往復と**不正値の拒否** |
| `workflow_execution_event_id.rs`（新規） | 3 | 組・`Display` 綴り・serde 往復 |
| `workflow_execution_event.rs` | 3 | `id()` の決定的採番、`is_created()` が genesis のみ、serde 往復 |
| `workflow_execution.rs` | 1 | 集約の serde 往復（`last_updated_at` / `version` を含む） |
| `stage_slug.rs` | 1 | `StageSlugError` の材料描画 |
| `event_store_impl.rs` | 1 | ISO 8601 の整形と逆変換の往復 |

### 証明テストの検出力（mutation 検証）

`Event::is_created` を `false` に固定する変異を入れて実測 → **4 本中 3 本が FAILED**。
コンパイルが通るだけでなく、本家の分岐（create / update / creation 拒否）を実際に
駆動していることを確認した。変異は元に戻してある。

証明テストが固定している内容:

1. genesis を `persist_event` に渡すと本家が拒否する（`is_created` の配線）
2. genesis → `persist_event_and_snapshot` → 読み直しで**集約が完全一致**（`last_updated_at` 込み）
3. スナップショット同時更新 → 楽観 version をストアが採番する（0 → 1）
4. **ジャーナルのみ追記**（`persist_event`）→ スナップショットが進まない状態から
   **replay で追いつく**（`seq_nr` 2 → 3、checkbox が `AwaitingApproval` に）
5. そこから続けてスナップショット同時更新 → cursor / checkbox / approved が期待どおり
6. ジャーナルの順序と `WorkflowExecutionEventId` の安定性、`since` 境界が**含む**こと

## 10. 検査結果（実測値）

| 検査 | 結果 |
| --- | --- |
| `cargo fmt --all --check` | **PASS**（無出力） |
| `cargo clippy --workspace --all-targets -- -D warnings` | **PASS**（警告 0） |
| `cargo lint` | **PASS**（exit 0） |
| `PROPTEST_RNG_SEED=20260823 cargo test --workspace` | **PASS — 689 passed / 0 failed**（ベースライン 674 全緑を維持 + 新規 15） |
| `bash scripts/quint-gate.sh` | **PASS — all steps green**（typecheck 3 / invariants 3 / witness 9 / `quint test r_.*` 1） |
| `cargo audit` | **PASS**（exit 0、120 crate、脆弱性 0） |
| `cargo audit --file tools/lint/Cargo.lock` | **PASS**（exit 0、5 crate、脆弱性 0） |

clippy で 2 件の指摘を受け、次のように処理した（握り潰しではない）:

1. `serde_json::to_string` が `clippy.toml` の `disallowed-methods`（BR1.7 — 契約 JSON は
   canon-json 経由）→ serde 境界そのものの往復確認であり BR1.7 の射程外なので、
   `#[allow(clippy::disallowed_methods, reason = ...)]` を**該当行だけ**に付け、
   理由をコメントで書いた（canon-json 内部の唯一の例外と同じ作法）。
2. `FakeClock` の `Mutex::lock().expect(..)` が `expect_used` deny → `Cell` に替えて
   panic 経路そのものを消した（allow を足していない）。

## 11. 未了 / 次委任へ

- **委任 2 の本体**（`EventStoreImpl` / `schema.rs` / ローカル `EventStore` trait /
  `JournalReader` の削除・差し替え、`sqlite` feature の有効化）— 本委任の対象外。
- **§7 の設計質問 A〜E** — 特に A（`Deserialize` の不変条件検査）と B（genesis の version）は
  委任 2 の設計に直接効くので、着手前の裁定が要る。
- **設計文書の同期**（§6 の 8 件）— コンダクタ担当。
- **コーディング規則の正本への追記 2 件**（§7 D / E）— オーナー確認が要るので触っていない。
- **Quint モデル `journal_protocol` の検証対象**（ADR-010 が「本家の契約 + 我々の投影へ移る」と
  予告）— 本委任ではモデルに触れず、ITF 準拠テストは型の追従のみ。委任 2 で再確認が要る。
- `git add` / `commit` / `push` は**行っていない**。
