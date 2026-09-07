# ユースケース層の規則 — DIP・スタティックバインディング・ユースケース間呼出禁止

**裁定日**: 2026-08-22（オーナー、統一ルール）
**適用例**: B-1 以降の全ユースケース実装（`CommitVerdictUseCase` / `NextUseCase` / …。~~`ReportUseCase`~~ → 改名 2026-08-29 オーナー裁定 — report が「レポート（帳票）」と誤読される動詞衝突のため、更新意図を先頭に置く `Commit*` へ。CLI 動詞との対応は U7 の ROUTES 表が持ち、型名は upstream の綴りに縛られない）
**機械強制**: Cargo のクレート分離（実装依存 = E0432）+ `cargo lint` ルール候補（use-case 層内の `*UseCase` import/呼出検出）

## 1. DIP — ユースケースは契約（trait）しか知らない

ユースケースが依存してよいのは**ポートの trait**（`XxxRepository` 等）とドメイン層だけ。`XxxRepositoryImpl` などの実装への依存は禁止。結線（実物/InMemory の選択）は **composition root だけ**が行う。

本リポジトリでは層 = クレートなので、`core-command-use-case` の `Cargo.toml` に `core-command-interface-adapter` が無いことがこの規則の機械強制になっている（import した瞬間 E0432）。**dev-dependency にも書けない** — テストのためだけの依存でも同じ辺が張られるので、DIP のクレート分離強制が壊れ、依存も循環する（実測 `modules/core/command/use-case/Cargo.toml`: `[dependencies]` は `core-command-domain` / `chrono`、`[dev-dependencies]` は `tokio` のみ。クレート名は CQRS のクレート分離に追従 — B12 以降）。

## 2. バインディングはスタティックが既定

```rust
pub struct CommitVerdictUseCase<E: IntentExecutionRepository, I: IntentRepository> {
    intent_execution_repository: E,
    intent_repository: I,
}

impl<E: IntentExecutionRepository, I: IntentRepository> CommitVerdictUseCase<E, I> {
    pub const fn new(
        intent_execution_repository: E,
        intent_repository: I,
    ) -> CommitVerdictUseCase<E, I> {
        CommitVerdictUseCase {
            intent_execution_repository,
            intent_repository,
        }
    }
}
```

**例の差し替え 2026-08-31（オーナー裁定、b26 段階2 完了）**: 旧例は `NextUseCase<R: WorkflowDefinitionRepository>` だったが、`next` / `continue` のような**読むだけ**の動詞はクエリ側（`modules/core/query/use-case` / `modules/core/query/interface-adapter`）へ移設済みであり、コマンド側に `NextUseCase` は存在しない（[cqrs-boundaries.md](cqrs-boundaries.md) 規則 5〜7 + 追補、§4 の再々裁定）。**コマンド側に残るユースケースは書き込むものだけ**なので、例も実在する書込ユースケース `CommitVerdictUseCase` に差し替えた。`WorkflowDefinitionRepository` が消えたわけではない — `find_by_id` と `store` の両動詞を持つ**通常のリポジトリ**としてコマンド側に残る（`store` の実装は定義を変更する最初のユースケースと同じ Bolt で書く — 先行実装しない）。

- **既定はジェネリクス（単相化）**。理由: ①`dyn` の object safety 制約で**契約の設計が歪む**のを防ぐ（`-> impl Iterator`・関連型・ジェネリックメソッドが使える）②ワンショット CLI で実装の数が少なく、単相化コストは無視できる ③テストがポート実装の素の値で組める ④配線ミスがコンパイル時に落ちる（E1 文化）。

  **ポート実装の 3 層（オーナー裁定 2026-08-31 / ADR-010、2026-09-07 実測）**: 自作 HashMap のテストダブルは禁止で、置き場は次の 3 つだけである。

  1. **本番のコマンド側 Gateway** — `XxxRepositoryImpl<S>`（`modules/core/command/interface-adapter/src/orchestration/*_repository_impl.rs`）。型引数 `S` がバックエンドで、`open()` が SQLite、`in_memory()` が本家 event-store-adapter-rs の memory バックエンドを選ぶ。**公開のインメモリ実装はこれが正**であり、実装コードは SQLite と 1 行も違わないので同じ契約テストを両方に課せる（実測 `intent_execution_repository_impl.rs:182` / `intent_repository_impl.rs:156` / `workflow_definition_repository_impl.rs:177`）。
  2. **use-case クレート内の `#[cfg(test)]` フェイク** — `InMemoryIntentExecutionRepository` / `InMemoryIntentRepository` / `InMemoryWorkflowDefinitionRepository` / `InMemoryCompiledDefinitionRepository`（`modules/core/command/use-case/src/orchestration/test_support.rs`、`pub(crate)`、`orchestration/mod.rs:38-39` で `#[cfg(test)]`）。**上記 §1 の DIP 制約下でユースケースを単体テストするための唯一の手段**である — アダプタ層を dev-dependency にも書けないので、本家ストアはこのクレートに届かない。禁止の対象外はこの用途だけで、他所で自作ダブルを書いてはならない。
  3. **クエリ側の公開テストダブル** — `InMemoryXxxDao` 13 本（`modules/core/query/interface-adapter/src/memory/`）。DAO はリードモデルを読むだけなのでイベントストアを要さない。
- **`dyn` を使ってよいのは**: 機構シーム（Gateway 実装内部の `Arc<dyn Clock>` 等 — 複数インスタンスで fake を共有する用途）と、将来ディスパッチャが多数のユースケースを一様保持する必要が実際に生じた**その境界だけ**。ユースケース自身の設計には持ち込まない。

## 2b. execute の引数は集約 ID と値オブジェクトのみ — 集約インスタンスを渡さない

**オーナー裁定 2026-08-30**: ユースケースの `execute` 引数に**集約を渡してはならない**。
渡してよいのは**集約 ID と値オブジェクト**（コマンドの材料）だけである。集約は
ユースケースが**保持するリポジトリ**で `execute` 内部から取得する（リポジトリを外で使わない）。

```rust
// ○ 集約 ID + 値オブジェクト
execute(&mut self, id: &IntentExecutionId, transition: ReportedTransition, at: DateTime<Utc>)

// ✕ 集約インスタンスの受け渡し（リポジトリ利用がユースケースの外へ漏れる）
execute(&mut self, id: &IntentExecutionId, intent: &Intent, ...)
```

- 読み取り専用ユースケースの「書けない」保証は、**find 系動詞しか持たない読取専用ポートの注入**
  で型保証する。~~（§2 の `NextUseCase<WorkflowDefinitionRepository>` が既にこの形）~~ —
  **この手法は失効（2026-08-31・オーナー、b26 段階2）**: 読むだけのユースケース自体が
  コマンド側から消え（クエリ側へ移設済み — §4 の再々裁定）、対象を失った。
- 本規則は**ユースケース層の署名**の話である。集約の**メソッド**が他集約を `&` 参照で受ける
  （`next_decision(&self, &WorkflowDefinition, ..)` 等）のは
  [aggregate-references.md](aggregate-references.md) が定めるドメイン内のパラメータ渡しで
  あり、対象外。

## 3. ユースケースからユースケースを呼ばない

**共有されるべきはドメイン層**。ユースケース間の再利用を許すと、ドメイン概念として彫り出されるべき共有ロジックが応用層に滞留し、**ドメイン層の設計が歪む**（貧血化）。この禁止は境界規律であると同時に、ドメインモデル発見の**強制関数**である（実証例（履歴 — `reap_eligible` は ADR-007 / Bolt B5 で退役、規律の例としてのみ残す）: `reap_eligible` — Gateway の判断複製を禁止した圧力が「reap 適格性」というドメイン述語を彫り出した）。

- 共有したいロジック → **下へ**（ドメインサービス・集約コマンド・Domain Primitive として抽出）
- 複数ユースケースの連携 → **上へ**（Controller / composition root がオーケストレーション）
- upstream 自体がこの規律で出来ている: `next` は unpark を呼ばず「実行せよ」という print directive を**返す**、resume 4 択はルーティング（コマンド名の提示）のみ、`--single` は Report が呼ぶのではなく **Controller が手前で分岐**する。

## 4. 読取専用の型保証（I8 型）は参照渡しで

> **射程の再裁定（2026-08-30・オーナー）**: 本節の「Controller が集約を `&` で渡す」機構は **§2b により失効** — execute の引数に集約は渡さない（読み取り専用でも）。読み取り専用の型保証は **find 系動詞しか持たない読取専用ポートの注入**へ置き換える（書込不能はポートの動詞集合が型で保証する）。本節の目的（読取専用を型で保証する）は生きており、手段だけが変わった。`CommitVerdictUseCase` への誤適用は B12 改訂 10 で是正済み。~~U6（Next）実装時は§2b の形で設計する~~
>
> **同日夕の再々裁定（2026-08-30・オーナー — 本節ごと失効）**: 読むだけのユースケース（next/continue）は**そもそもコマンド側に存在しない** — クエリ側（リードモデルを読む実装）へ移る（[cqrs-boundaries.md](cqrs-boundaries.md) 規則 5〜7 + 追補）。したがって「読取専用ポートの注入」という型保証の手法も対象を失って失効。コマンド側のリポジトリは `find_by_id` + `store` の両動詞を持ち、`find_by_id` だけを呼んで何も書かないユースケースが違反である。移設は Issue #65 の Bolt で実施。**（2026-08-31 追記）移設は b26 段階2 で実施済み** — コマンド側の `NextUseCase` / `ContinueUseCase` / `NextTurnInput` を削除し、実装をクエリ側（`modules/core/query/use-case` / `modules/core/query/interface-adapter`）へ移した。
>
> **失効の射程は「コマンド側」に限る（2026-08-31・オーナー裁定、b27）**: 上の再々裁定が失効させたのは**コマンド側**で読むだけのユースケースに読取専用ポートを注入する手法であって、**クエリ側**のポート注入ではない。クエリ側のユースケースは**リードモデルを読む DTO/DAO ポート**（読取動詞 `find` のみ・更新動詞なし）を注入して読むのが**正式な形**であり、ポートは use-case 層の `port/` に**コマンド側と同型に**配置する（[cqrs-boundaries.md](cqrs-boundaries.md) 規則 6 の同日追記）。§1 の DIP（trait のみ依存・結線は合成ルート）と §2 のスタティックバインディング既定は、**クエリ側ユースケースにもそのまま適用する** — `NextUseCase<D: WorkflowDefinitionDao, S: ExecutionStateDao, M: MemoryRulesDao>` が実例。b26 が採ったポートレス（読取結果を `execute` の引数で値渡し）の形は、この失効をクエリ側へ誤適用したものであり b27 で是正した。

書込を型で禁じたいユースケース（例: `Next`）には Repository を**そもそも注入しない**。Controller が `repository.find_by_id()` した集約を `&` 参照で渡す — リードモデルを経由せずに、Rust の参照とポート非注入だけで書込不能が成立する（動詞は gateway-taxonomy §2b の許容語彙に合わせた。`find()` は廃止 — C4 改訂 2026-08-23）。（**2026-08-24 改訂**: 旧文は「CQRS を導入せずに（gateway-taxonomy.md — CQRS 不採用）」だったが、ADR-001/003/004 で CQRS + ES を採用済みのため前提が失効した。本節が言いたいのは「読取専用を**型で**保証する」ことであり CQRS の採否とは独立である。依存境界は [cqrs-boundaries.md](cqrs-boundaries.md) を参照）
