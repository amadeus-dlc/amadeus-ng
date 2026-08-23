# ユースケース層の規則 — DIP・スタティックバインディング・ユースケース間呼出禁止

**裁定日**: 2026-08-22（オーナー、統一ルール）
**適用例**: B-1 以降の全ユースケース実装（`ReportUseCase` / `NextUseCase` / …）
**機械強制**: Cargo のクレート分離（実装依存 = E0432）+ `cargo lint` ルール候補（use-case 層内の `*UseCase` import/呼出検出）

## 1. DIP — ユースケースは契約（trait）しか知らない

ユースケースが依存してよいのは**ポートの trait**（`XxxRepository` 等）とドメイン層だけ。`XxxRepositoryImpl` などの実装への依存は禁止。結線（実物/InMemory の選択）は **composition root だけ**が行う。

本リポジトリでは層 = クレートなので、`core-use-case` の `Cargo.toml` に `core-interface-adapter` が無いことがこの規則の機械強制になっている（import した瞬間 E0432）。

## 2. バインディングはスタティックが既定

```rust
pub struct NextUseCase<R: WorkflowDefinitionRepository> {
    repository: R,
}

impl<R: WorkflowDefinitionRepository> NextUseCase<R> {
    pub fn new(repository: R) -> Self { Self { repository } }
}
```

例の `WorkflowDefinitionRepository` は `save` を持たない読取専用ポートなので、§4 の I8（`Next` に書込側の `WorkflowExecutionRepository` を注入しない）と両立する — 注入禁止の対象は書込側の Repository であり、読取専用の定義 Repository ではない（10 §3）。

- **既定はジェネリクス（単相化）**。理由: ①`dyn` の object safety 制約で**契約の設計が歪む**のを防ぐ（`-> impl Iterator`・関連型・ジェネリックメソッドが使える）②ワンショット CLI で実装は実質 2 つ（Impl + InMemory）— 単相化コストは無視できる ③テストが `XxxUseCase<InMemoryXxxRepository>` の素の値で組める ④配線ミスがコンパイル時に落ちる（E1 文化）。
- **`dyn` を使ってよいのは**: 機構シーム（Gateway 実装内部の `Arc<dyn Clock>` 等 — 複数インスタンスで fake を共有する用途）と、将来ディスパッチャが多数のユースケースを一様保持する必要が実際に生じた**その境界だけ**。ユースケース自身の設計には持ち込まない。

## 3. ユースケースからユースケースを呼ばない

**共有されるべきはドメイン層**。ユースケース間の再利用を許すと、ドメイン概念として彫り出されるべき共有ロジックが応用層に滞留し、**ドメイン層の設計が歪む**（貧血化）。この禁止は境界規律であると同時に、ドメインモデル発見の**強制関数**である（実証例: `reap_eligible` — Gateway の判断複製を禁止した圧力が「reap 適格性」というドメイン述語を彫り出した）。

- 共有したいロジック → **下へ**（ドメインサービス・集約コマンド・Domain Primitive として抽出）
- 複数ユースケースの連携 → **上へ**（Controller / composition root がオーケストレーション）
- upstream 自体がこの規律で出来ている: `next` は unpark を呼ばず「実行せよ」という print directive を**返す**、resume 4 択はルーティング（コマンド名の提示）のみ、`--single` は Report が呼ぶのではなく **Controller が手前で分岐**する。

## 4. 読取専用の型保証（I8 型）は参照渡しで

書込を型で禁じたいユースケース（例: `Next`）には Repository を**そもそも注入しない**。Controller が `repository.find_by_id()` した集約を `&` 参照で渡す — CQRS を導入せずに（[gateway-taxonomy.md](gateway-taxonomy.md) — CQRS 不採用）、Rust の参照とポート非注入だけで書込不能が成立する（動詞は gateway-taxonomy §2b の許容語彙に合わせた。`find()` は廃止 — C4 改訂 2026-08-23）。
