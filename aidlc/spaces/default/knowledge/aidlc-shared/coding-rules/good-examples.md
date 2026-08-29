# 良い例カタログ — リポジトリ内の実在コードで規則を示す

**作成日**: 2026-08-24（オーナー提案「コード上のよい例って規則に含まれていましたっけ？」）
**性格**: 規則ではなく**索引**。各規則の抽象的な文面に対して、「この形」と指せる実物を列挙する。

## なぜファイルを指すのか

規則にスニペットを書き写すと、コードが変わったとき例だけが古くなる。**実在のファイルを指せば
例は勝手に追随する**し、周辺の文脈（エラー型・テスト・doc）まで一緒に読める。逆に、ここに
挙げた例が規則から外れたら、それは規則違反か、規則を見直す合図である。

**リンク切れは所見**として扱う。改名・移動でここが指せなくなったら、カタログを直すのではなく
「なぜ動いたか」を先に確認すること。

## 値オブジェクト — parse-don't-validate

**[`domain/src/workflow_definition/stage_slug.rs`](../../../../../../modules/core/domain/src/workflow_definition/stage_slug.rs)**

`StageSlug` は次をすべて満たす。関連規則: [factory-naming.md](factory-naming.md) /
[parse-don't-validate の考え方] / [field-visibility.md](field-visibility.md)。

- **private タプルフィールド** — 外から中身を組み立てられない
- **`parse(&str) -> Result<Self, E>` が唯一の入口** — 不正な値を持つ `StageSlug` は表現不能
- **エラー enum が理由を具体的に運ぶ** — `Empty` / `InvalidLeading(char)` / `InvalidChar(char)`。
  「不正」ではなく**何がどう不正か**を型で返す
- **`as_str(&self) -> &str`** — 「中身を出す」ではなく「`&str` として見る」と言っている。
  `value()` と名乗ると呼び手は `StageSlug` を「String が入った箱」として扱いはじめる
  （[tell-dont-ask.md](tell-dont-ask.md) の「内部型を意識させない」）
- **`Display` 実装** — 表示は `Display`、生の値は `as_str`、と役割が分かれている

同型の例: `IntentId::parse`（UUIDv7）、`StageNumber::parse`、`DefinitionRevision::parse`、
`ProjectionName::parse`、`SpaceName::parse`。

## 不変な値を 1 つだけ変える — `with_*` / `to_builder()`

**[`domain/src/workflow_definition/scope_metadata.rs`](../../../../../../modules/core/domain/src/workflow_definition/scope_metadata.rs)**（値型に直接 `with_*`）

```rust
pub fn with_depth(mut self, depth: String) -> ScopeMetadata
pub const fn with_skeleton(mut self, skeleton: SkeletonDefault) -> ScopeMetadata
```

`mut self` を取って `Self` を返すのでこれは **setter ではなくファクトリメソッド**である
（[factory-naming.md](factory-naming.md)）。`with_` が付いているので「depth を伴った新しい値」
と読める。裸のフィールド名（`depth(x)`）だと「depth を返す」に読めてしまう。

フィールドが多い型は `person.to_builder().with_first_name("kato").build()` の形で往復する。
`build()` が基本コンストラクタを呼ぶので、構築経路は 1 本のまま保たれる。

## 集約 — FSM として設計する

**[`domain/src/orchestration/workflow_execution.rs`](../../../../../../modules/core/domain/src/orchestration/workflow_execution.rs)**

- **コマンドは `&mut self`、1 コマンド 1 イベント**（ADR-002）。判断はクエリメソッド
- **ドメインの語でメソッドを名乗る** — `start` / `park` / `unpark` / `mark_stage` /
  `record_approval` / `invalidate_approval` / `switch_autonomy`
  （[ubiquitous-language.md](ubiquitous-language.md)）
- **ガードは `Err` で拒否**する（`guard_running`）。呼び手が状態を確かめてから呼ぶ形にしない

## Gateway — ポートは use-case 層、実装は `XxxRepositoryImpl`

**[`use-case/src/orchestration/workflow_execution_repository.rs`](../../../../../../modules/core/use-case/src/orchestration/workflow_execution_repository.rs)**（ポート）と
**[`interface-adapter/src/orchestration/workflow_execution_repository_impl.rs`](../../../../../../modules/core/interface-adapter/src/orchestration/workflow_execution_repository_impl.rs)**（実装）

- ポートは trait だけ。実装の語（SQLite / ファイル）がポート面に出ない
  （[gateway-taxonomy.md](gateway-taxonomy.md)）
- `find_by_id`（Query, `&self`）/ `store`（Command, `&mut self`）
  （[command-query-separation.md](command-query-separation.md)）
- 実装は `EventStoreImpl` を**直接所有**。内部可変性なし
  （[interior-mutability.md](interior-mutability.md)）

## 契約テスト — 同じ約束を両実装に課す

**[`interface-adapter/tests/workflow_execution_repository_contract.rs`](../../../../../../modules/core/interface-adapter/tests/workflow_execution_repository_contract.rs)**

- ジェネリック関数 1 本を 2 実装（SQLite / InMemory）に同一に走らせる
- **契約の外**（実装ごとに違ってよい挙動）を trait doc に**逸脱として明記**し、
  各実装の実挙動を実装固有テストで固定する。「契約の外だから書かない」にしない

## 反例カタログ — 表の動詞へ矯正してはいけない 25 件

**[`<record>/construction/u3-event-store-repository/code-generation/naming-audit-report.md`](../../../intents/260822-stage1-selfhost/construction/u3-event-store-repository/code-generation/naming-audit-report.md)** §3

`hash_canonical` / `serialize` / `IntentExecution::start`（旧 `WorkflowExecution::start`） / `encode`・`decode` /
`open_append_only` など、[factory-naming.md](factory-naming.md) の表の動詞になっていないが
**現在の名前のほうが良い**もの。将来 lint ルールを書く際の反例として使う。
