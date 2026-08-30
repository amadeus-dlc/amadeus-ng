# Gateway の責務分類と命名 — Repository は集約名から名付ける

**裁定日**: 2026-08-22（オーナー、共通ルール）
**適用例**: Gateway 責務再設計 PR（`StateFileStore` ポート削除 / `StageGraphReader` → `WorkflowDefinitionRepository` / Clock・ProcessProbe のアダプタ層退去）
**機械強制**: レビュー基準（未リント化）。将来 `cargo lint` ルール候補は下記「機械強制の候補」

## ルール

### 1. Gateway 責務は 2 つだけ

インターフェイスアダプタ層の Gateway が担うのは次の 2 種類に限る。

> **追加 2026-08-24**: ADR-001（ES 採用）以降、第三の責務 **永続化基盤ポート**が存在する
> （下記 §1c）。「2 種類に限る」は**ドメインの Gateway について**の話であり、ES 基盤は
> その下請けである。

| 責務 | 中身 | 対応するポート |
| --- | --- | --- |
| **Repository** | 集約の永続化・再構成 | `XxxRepository`（`Xxx` = 集約名） |
| **外部システムクライアント** | Git / GitHub / ハーネス CLI など、別プロセス・別システムとの RPC | 外部システム名を冠した専用ポート（例: `GitHubPullRequestClient`） |

**機構（時計・ID 生成・プロセス生存判定・乱数・環境変数読取）は Gateway ではない。** clean-architecture の層責務では、時計と ID 生成器と DI 配線は Infrastructure が所有する機構であって、アプリケーション境界のポートではない（典拠: `j5ik2o-clean-architecture/references/layer-responsibilities.md` — *"Infrastructure ... Owns mechanisms: logging, metrics, configuration, dependency injection, concrete database drivers, HTTP clients, **clocks, ID generators**, and runtime wiring."*）。

判定は「**どのユースケースがこのポートを消費するか**」で行う。答えられないなら、それはアプリ境界のポートではなく、実装の内部注入シームである。

- 実装は**アダプタ層の機構モジュール**に置く（本リポジトリでは `core_interface_adapter::{clock}` — コンテキスト（`orchestration` / `workspace`）の外、クレート root）。
- 配線（実物と fake の差し替え）は **composition root** が行う。
- use-case 層には trait を置かない。置くと「ユースケースが消費しないポート」がポート表に居座り、Gateway 責務の分類が濁る。

### 1b. 非 Repository ポートの一般形

Repository（集約 I/O）に当てはまらない外界協調は、**アウトプット契約をそのまま trait に表現**する。そのとき、**契約の意味論（予算・再入・二重解放不能など）を散文ではなく型に載せる** — 上限や締切は専用の引数型で、使い切りの資源は非 `Clone` のガード型で、といった具合に、契約を破る呼び方がそもそも書けない形にする。集約ではないものを Repository に無理に寄せない。

旧模範例の `WorkspaceLock`（2026-08-22 承認）は ADR-007 で退役した（並行制御は SQLite Tx + 楽観 version）。型に意味論を載せるという設計指針だけを引き継ぐ（ADR-007 / 2026-08-23）。

### 2b. Repository のメソッド語彙（j5ik2o-ddd-repository-design が正典）

- 使ってよい動詞: **`find_by_id` / `find`（単一集約の named retrieval）/ `save` / `remove`** ＋ **ドメイン概念を表す named retrievals**。`load` / `get` / `fetch` 等は使わない。
- `find_by_...` の無秩序な増殖は禁止。複雑な検索・画面向け読取は**読取モデル側**で行う（ADR-003 / ADR-004 — `aidlc-state.md` と監査シャードが読取モデルで、`ReadModelUpdater` が投影する）。Repository に生やす前に、まず「ドメイン概念を表す named retrieval」で表現できるか、そもそも読取モデルの仕事ではないかを考える。
- インターフェイスで **not-found の挙動・ロック・トランザクション所有・永続化エラー**を明示的に定義する（例: `WorkflowDefinitionRepository::find_by_id` の not-found は契約上 fatal な `Err`（`NotFound { expected, actual }`、identity ファイルの読取失敗は `HarnessIdentity { path, cause }`）、grid 欠損は転置導出 — 12 §4。引数なしの `find()` は廃止済み — C4 改訂 2026-08-23 / ADR-008）。
- **アンチパターン**（スキル逐語より）: Repository が内部エンティティを返す / 集約が Repository を呼ぶ / **`updateField` 系メソッドで集約の振る舞いを迂回する**（外科的ライタ（フィールド単位で状態ファイルを書き換える純関数）は `XxxRepositoryImpl` の内部詳細に限り、Repository のメソッドにしない）/ ジェネリックな基底 Repository。

**ES Repository の拡張語彙**: イベントソーシングの Repository（`IntentExecutionRepository` — ~~`WorkflowExecutionRepository`~~ 集約の分割・改名 2026-08-29 に追随）は `store(event, aggregate)` / `find_by_id` を動詞とする。上の許容動詞一覧は**ステートソーシング Repository の規則**であり、ES Repository の動詞は本家ライブラリ（event-store-adapter-rs）の語彙に従う — `store` はその拡張語彙として明示的に許可する（ADR-006）。

### 2. Repository 名 = 集約名 + Repository

集約は各コンテキスト仕様の宣言表が持っている（[`01-domain-model.md`](../../../../../../docs/specs/01-domain-model.md) §3 の集約候補、[`11-workspace.md`](../../../../../../docs/specs/11-workspace.md) §2.1、[`12-workflow-definition.md`](../../../../../../docs/specs/12-workflow-definition.md) §2.1）。Repository はそこに載っている集約ルート名をそのまま冠する。

- `IntentExecution` → `IntentExecutionRepository`（~~`WorkflowExecution` → `WorkflowExecutionRepository`~~ 集約の分割・改名 2026-08-29）
- `Intent` → `IntentRepository`（U7 の intent-create 実装時に新設予定。**Repository は自分の集約・エンティティだけを I/O する** — `IntentRepository` は `Intent` のみ、`IntentExecutionRepository` は `IntentExecution` のみ。他方を復元して返すのも違反。**署名は自集約の ID だけを取る** — 他の集約・エンティティを引数にも戻り値にも出さない。再生に他エンティティの材料が要る場合、それは自ストリームの誕生イベントに記録されているはずであり、Impl がそこから内部復元する（`find_by_id(&IntentExecutionId)` が `Started` から再生用 `Intent` を組む実例 — オーナー確定 2026-08-29）（オーナー確認 2026-08-29）。再生・判断に他方のデータが要るときは `&` 参照のパラメータ渡し — [aggregate-references.md](aggregate-references.md)）
- `WorkflowDefinition` → `WorkflowDefinitionRepository`

`AuditLedger` はイベントログ（`IntentExecution` のイベント列）であって集約ではないため、Repository を持たない — 監査シャードは ReadModelUpdater の投影である（ADR-001 / 003）。

**ストレージ媒体名の Repository は禁止。**

| 禁止名 | 理由 |
| --- | --- |
| `StateFileRepository` | `StateFile`（`aidlc-state.md`）は**永続化媒体**であって集約ではない。集約は `IntentExecution`（旧 `WorkflowExecution`）であり、「状態がファイルに入っている」は Repository **実装**の内部詳細 |
| `StageGraphRepository` | `stage-graph.json` という**ファイル名由来**の名前。集約は 3 入力を束ねた `WorkflowDefinition` で、`StageGraph` はその内包物 |

媒体名を冠すると、格納形式の変更（ファイル → SQLite → リモート）がポート名の変更に波及し、ユースケース層が永続化の都合を知ってしまう。

### 1c. 第三の責務 — 永続化基盤ポート（2026-08-24 追加）

ADR-001 でイベントソーシングを採用した結果、Repository でも外部システムクライアントでもない
ポートが実在する。**Repository の下請けとして、ジャーナル・スナップショット・投影チェック
ポイントを扱う基盤**である。

| 責務 | 中身 | 対応するポート | 実在 |
| --- | --- | --- | --- |
| **永続化基盤ポート** | ES のジャーナル追記・スナップショット・投影チェックポイント | `EventStore` / `JournalReader` | `use-case/src/orchestration/{event_store,journal_reader}.rs` |

**語彙は本家ライブラリ（event-store-adapter-rs）に従う。** §2b の Repository 動詞規則
（`load` / `get` / `fetch` を使わない）は **Repository 限定**であり、このポートには及ばない
— 実在の `get_latest_snapshot_by_id` / `get_events_by_id_since_seq_nr` は本家の語彙であって
違反ではない（[ubiquitous-language.md](ubiquitous-language.md) の Published Language）。

集約の永続化を担うのは `IntentExecutionRepository` であり、`EventStore` はその下請けである。
**ユースケースが `EventStore` を直接注入されることはない**（するなら Repository の意味が消える）。

### 3. ポート造語（Store / Reader / Writer / Source / Provider）は禁止

`XxxStore` / `XxxReader` / `XxxWriter` は DDD の語彙ではない。「読むだけの Gateway だから Reader」という命名は、**Repository の一部の操作にポートを 1 つずつ立てる**ことになり、集約単位のトランザクション境界を name の上で解体する。~~読取専用の集約（本システムから書き換えない `WorkflowDefinition` のような Published Language 成果物）は、`save` を持たない Repository として表現すればよい~~ — **失効（2026-08-30 オーナー裁定）**: リポジトリは `find_by_id` と `store` の両動詞を持つのが正であり、`find_by_id` だけを呼んで何も書かない使い方こそが違反（それはクエリ側の仕事 — [cqrs-boundaries.md](cqrs-boundaries.md) 追補）。`WorkflowDefinitionRepository` も両動詞を持つ（`store` 実装は定義を変更する最初のユースケースと同じ Bolt で書く）。

**明示的な例外（2026-08-24）**: **§1c の永続化基盤ポート `EventStore` / `JournalReader` は本禁止の対象外**である。
理由は 2 つ。(a) これらは Repository ではないので「Repository の操作を分割した」という本禁止の
根拠が当てはまらない。(b) 名前が本家 event-store-adapter-rs の Published Language であり、
ドメイン語へ言い換えると対応が読めなくなる（[ubiquitous-language.md](ubiquitous-language.md)）。
**例外はこの 2 本のみ**で、新たに `XxxStore` / `XxxReader` を増やすことは認めない。
機械化する場合も、この 2 本を除外リストに持つ実装にすること。

### 4. 読取専用ユースケースは型で保証する

> **改訂 2026-08-24（オーナー裁定）**: 本節は当初「CQRS は採用しない（まず素の DDD）」だった。
> その後 ADR-001 でイベントソーシングを、**ADR-003「SQLite ストア + upstream 互換ファイルは
> リードモデル + RMU」/ ADR-004「状態ファイルはリードモデル」で読取モデルの分離を採用**したため、
> 前提が失効した。書込モデル（集約 `WorkflowExecution` + `EventStore` のジャーナル/スナップショット）と
> 読取モデル（`aidlc-state.md` と監査シャード。`ReadModelUpdater` がチェックポイント以降の
> イベントを投影して更新）は**実際に分かれている**。節の本体（読取専用を型で保証する 2 手段）は
> CQRS の採否とは独立に有効なので、前提の記述だけを差し替えて残す。

集約の書込口は Repository に一本化する（動詞は §2b の許容語彙 — 設計監査 C2 / 2026-08-23）。
読取モデルは Repository ではなく投影（RMU）が更新するので、**Repository に読取モデル向けの
検索メソッドを生やさない**（§2b の「`find_by_...` の無秩序な増殖は禁止」と同じ帰結）。

「このユースケースには書かせたくない」という**型による保証**は、次の 2 手段で実現する。

- **Writer を注入しない**: 読取専用ユースケースのコンストラクタに Repository を渡さない。
- **`find_by_id` 済み集約を `&` 参照で渡す**: Controller が Repository で集約を `find_by_id` し、ユースケースには `&Aggregate` を渡す。所有権と可変性が Rust の型で読取専用を保証する。

例: [`10-orchestration.md`](../../../../../../docs/specs/10-orchestration.md) I8（`next` は読み取り専用）は、`Next` ユースケースに `IntentExecutionRepository` を注入せず、Controller が `find_by_id` 済みの `IntentExecution` を `&` で渡すことで型強制する（設計監査 C2 / 2026-08-23）。

### 5. 配置と命名 — trait は use-case 層、実装は `XxxRepositoryImpl`

| 種別 | 層 | 命名 | 例 |
| --- | --- | --- | --- |
| ポート（trait） | use-case | `XxxRepository` | `WorkflowDefinitionRepository` |
| 実 Gateway 実装 | interface-adapter | `XxxRepositoryImpl` | `WorkflowDefinitionRepositoryImpl` |
| テストダブル | interface-adapter（`memory/` 配下） | `InMemoryXxxRepository` | `InMemoryWorkflowDefinitionRepository` |

- **1 trait 1 Impl**。`Fs` / `Sys` / `Postgres` のような技術接頭辞は使わない — 格納形式は実装の内部詳細であり、型名に出せば「どの技術を使うか」がレビュー対象の公開 API になってしまう。
- `Impl` 接尾辞は**本物の Gateway 実装の印**。テストダブルには付けず、`InMemory` 接頭辞で区別する。
- 集約が使う trait はユースケース層に置くが、**集約自身は Repository を呼ばない**（典拠: `j5ik2o-ddd-repository-placement` — *"Aggregate code: no repository dependencies."* / *"Application/use-case layer: depends on repository interfaces and orchestrates loading/saving."*）。find / save の指揮はユースケースが執る（動詞は §2b の許容語彙に合わせた — 設計監査 C2 / 2026-08-23）。

### 6. Repository は in-memory から始める

典拠: `j5ik2o-ddd-domain-model-first/references/details.md` § Repository Timing — *"Do not introduce database repositories before use cases need persistence. In-memory repositories reveal the port contract without locking the domain to a database."*

`InMemoryXxxRepository` を先に書いてポート契約を露出させ、実 I/O 実装は後から足す。実装前のポートに永続化の都合（ファイル名・スキーマ）を混ぜないための順序である。

## 適用の帰結（2026-08-22 の再設計）

| 旧 | 新 | 理由 |
| --- | --- | --- |
| `core_use_case::workspace::StateFileStore`（ポート） | 削除 → B-2 の `WorkflowExecutionRepository` | ポート造語 + 媒体名。実装 `FsStateFileStore` は `workspace::state_file_io`（private mod・`pub(crate)`）へ降格し、Repository 実装の内部部品になった |
| `core_use_case::orchestration::StageGraphReader` | `WorkflowDefinitionRepository` | Reader 造語 + ファイル名由来。集約は 3 入力を束ねた `WorkflowDefinition`（12 §2.1 で集約ルートへ昇格） |
| `core_use_case::workspace::Clock` / `ProcessProbe` | `core_interface_adapter::{Clock, ProcessProbe}` | どのユースケースも消費しない。`FsWorkspaceLock` の注入シームにすぎず、機構は Infrastructure 責務 |

## 機械強制の候補

いずれも未実装。優先順は 型（E1）→ 既存 lint → `cargo lint` カスタムルール（赤例テスト必須）。

1. **ポート造語の検出**: use-case 層の `pub trait` 名が `Store` / `Reader` / `Writer` / `Source` / `Provider` で終わったら拒否。
2. **Repository 名と集約名の照合**: `XxxRepository` の `Xxx` が `core-domain` に存在する集約ルート型名であることを検査（集約表を機械可読にする前提が要る）。
3. **技術接頭辞の検出**: interface-adapter 層の `XxxRepository` 実装型名が `Fs` / `Sys` / `Db` 等で始まったら拒否（`XxxRepositoryImpl` / `InMemoryXxxRepository` のみ許可）。
4. **I8 の型強制**: `Next` ユースケースの構造体フィールドに Repository 型が現れないことを検査。

## 根拠

Gateway を「外界に触る何か」の総称にすると、そこに置いたポートの数だけユースケース層の依存面が増え、集約とトランザクション境界が名前の上で見えなくなる。責務を Repository と外部システムクライアントの 2 つに絞り、機構をアダプタ層の内部に押し込めることで、**ポート表がそのままアプリケーション境界の一覧**になる。名前を集約から取るのは、その一覧を読んだ人が「どの集約が永続化されるか」をコンテキスト仕様の集約表と 1 対 1 に突き合わせられるようにするためである。
