# ユースケース層の規則 — DIP・スタティックバインディング・ユースケース間呼出禁止

**裁定日**: 2026-08-22（オーナー、統一ルール）
**適用例**: B-1 以降の全ユースケース実装（`CommitVerdictUseCase` / `NextUseCase` / …。~~`ReportUseCase`~~ → 改名 2026-08-29 オーナー裁定 — report が「レポート（帳票）」と誤読される動詞衝突のため、更新意図を先頭に置く `Commit*` へ。CLI 動詞との対応は U7 の ROUTES 表が持ち、型名は upstream の綴りに縛られない）
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

例の `WorkflowDefinitionRepository` は `save` を持たない読取専用ポートなので、§4 の I8（`Next` に書込側の `IntentExecutionRepository` を注入しない）と両立する — 注入禁止の対象は書込側の Repository であり、読取専用の定義 Repository ではない（10 §3）。

- **既定はジェネリクス（単相化）**。理由: ①`dyn` の object safety 制約で**契約の設計が歪む**のを防ぐ（`-> impl Iterator`・関連型・ジェネリックメソッドが使える）②ワンショット CLI で実装は実質 2 つ（Impl + InMemory）— 単相化コストは無視できる ③テストが `XxxUseCase<InMemoryXxxRepository>` の素の値で組める ④配線ミスがコンパイル時に落ちる（E1 文化）。
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
  で型保証する（§2 の `NextUseCase<WorkflowDefinitionRepository>` が既にこの形）。
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
> **同日夕の再々裁定（2026-08-30・オーナー — 本節ごと失効）**: 読むだけのユースケース（next/continue）は**そもそもコマンド側に存在しない** — クエリ側（リードモデルを読む実装）へ移る（[cqrs-boundaries.md](cqrs-boundaries.md) 規則 5〜7 + 追補）。したがって「読取専用ポートの注入」という型保証の手法も対象を失って失効。コマンド側のリポジトリは `find_by_id` + `store` の両動詞を持ち、`find_by_id` だけを呼んで何も書かないユースケースが違反である。移設は Issue #65 の Bolt で実施。

書込を型で禁じたいユースケース（例: `Next`）には Repository を**そもそも注入しない**。Controller が `repository.find_by_id()` した集約を `&` 参照で渡す — リードモデルを経由せずに、Rust の参照とポート非注入だけで書込不能が成立する（動詞は gateway-taxonomy §2b の許容語彙に合わせた。`find()` は廃止 — C4 改訂 2026-08-23）。（**2026-08-24 改訂**: 旧文は「CQRS を導入せずに（gateway-taxonomy.md — CQRS 不採用）」だったが、ADR-001/003/004 で CQRS + ES を採用済みのため前提が失効した。本節が言いたいのは「読取専用を**型で**保証する」ことであり CQRS の採否とは独立である。依存境界は [cqrs-boundaries.md](cqrs-boundaries.md) を参照）
