# ファクトリの命名 — コンストラクタ相当は `new`、それ以外は用途で名前を選ぶ

**裁定日**: 2026-08-24（オーナー）
**出典**: オーナー提示の命名表（Java 由来）を Rust へ翻訳したもの
**適用例**: U3（Bolt B5）— `EventStoreImpl::open` / `StorePath::for_space` / `WorkflowExecution::start` ほか
**機械強制**: `cargo lint` ルール化候補（下記「機械化の候補」）。現状はレビュー基準

## 原則

**コンストラクタ相当のファクトリは `fn new(...) -> Self` に統一する**（オーナー明言）。
失敗しうる生成は `fn new(...) -> Result<Self, E>` とし、名前は `new` のままにする
（`try_new` のような別名を並立させない — [no-backward-compatibility.md](no-backward-compatibility.md)）。

コンストラクタ相当**でない**ファクトリは、下表の用途に従って名前を選ぶ。
名前が用途を語るので、doc を読まなくても何が起きるか予想がつく
（[interior-mutability.md](interior-mutability.md) の「シグニチャから内部の振る舞いが
予想できるのが良い設計」と同じ理由）。

## 対応表（Rust 翻訳版）

| 名前 | 用途 | Rust での綴り | 例 |
| --- | --- | --- | --- |
| `new` | **コンストラクタ相当**（受け取った値をそのまま組み立てる） | `fn new(..) -> Self` / `-> Result<Self, E>` | `StageEntry::new(..)` |
| `of` | 複数の値を集約してインスタンスを生成（値オブジェクト） | `fn of(..) -> Self` | `ShardName::of(host, clone_id)` |
| `from` | 他の型からの変換 | **`impl From<T>` / `impl TryFrom<T>` を第一選択**。inherent にするなら `from_<源の名前>` | `PhaseId::from_index(u32)`、`CheckboxState::from_marker(char)` |
| `parse` | 文字列を解析して生成 | `fn parse(s: &str) -> Result<Self, E>`（可能なら `impl FromStr` も） | `IntentId::parse(&str)` |
| `create` | 新しいエンティティ／ドメインオブジェクトを作る | ドメイン語があればそちらを優先、無ければ `create` | 集約の genesis は `WorkflowExecution::start(..)` |
| `generate` | ランダム・計算・アルゴリズムに基づいて値を作る | `fn generate(..) -> Self` | UUIDv7 の採番 |
| `open` | 外部リソースを開いてハンドルを得る（**表には無いが Rust の標準慣用**） | `fn open(..) -> Result<Self, E>` | `EventStoreImpl::open(path, clock)`（`File::open` と同型） |

## Rust に合わせて**採らない**もの（理由つき）

出典表の 3 つは、そのまま転写すると Rust の言語慣用と衝突するため採用しない。

| 出典 | 採らない理由 | 代わりに |
| --- | --- | --- |
| `valueOf` | Rust には boxing キャッシュ（`Integer.valueOf` が前提とする仕組み）が無い。再利用しながらの変換という概念が言語側に存在しない | 変換なら `from` 系。本当にインターンが要る型が現れたらオーナー裁定で `value_of` を足す |
| `getInstance` | Rust API ガイドライン **C-GETTER** が `get_` 接頭辞を禁じている。`get_instance` は書いた瞬間に慣用違反になる | シングルトンは `fn instance() -> &'static Self`、既定値は `impl Default` |
| `newInstance` | Rust では「常に新しいインスタンスを生成する」のが `new` の既定の意味であり、`new_instance` は `new` の同義語にしかならない（口が 2 つ並ぶ） | `new` |

## 判定フロー

```
1. 受け取った値をそのまま組み立てるだけか？
   ├─ Yes → new（失敗しうるなら -> Result<Self, E>、名前は new のまま）
   └─ No  → 次へ

2. 何から作るか？
   ├─ 文字列を解析     → parse（+ FromStr）
   ├─ 他の型から変換   → From / TryFrom。inherent なら from_<源>
   ├─ 複数の値を集約   → of
   ├─ 外部リソースを開く → open（-> Result<Self, E>）
   ├─ 乱数・計算で作る → generate
   └─ ドメインの出来事として作る → ドメイン語（例: start）、無ければ create
```

## 禁止パターン

- コンストラクタ相当なのに `new` 以外の名前を付ける（`make` / `build_new` / `construct` など）
- 失敗しうる生成を `new` と `try_new` に分けて**両方公開**する
- inherent メソッドに素の `fn from(x: T) -> Self` を書く（`From::from` と綴りが衝突し、
  呼出側でどちらが呼ばれるか読めなくなる）
- `get_` 接頭辞のファクトリ（C-GETTER 違反）
- `new_instance` / `create_new` のような `new` の同義語を並立させる
- 同じ用途に複数の入口を残す（[no-backward-compatibility.md](no-backward-compatibility.md)）

## 対象外

- **ビルダーの終端** `fn build(self) -> T` は本表の対象外（ビルダーパターンの語）。
- **変換メソッド**（`as_*` / `to_*` / `into_*`）は生成ではなく変換なので、Rust API
  ガイドライン C-CONV に従う。
- `&mut self` を受けて自身を書き換えるコマンドはファクトリではない
  （[command-query-separation.md](command-query-separation.md)）。

## 機械化の候補

優先順は 型 → 既存 lint → `cargo lint` カスタムルール（README の方針）。赤例テスト必須。

1. `impl` ブロック内の関連関数で、戻り値が `Self` / 自型 / `Result<Self, _>` / `Option<Self>` の
   ものを列挙し、名前が `new` / `of` / `from_*` / `parse` / `create` / `generate` / `open` の
   いずれでもなければ報告する（例外は `#[allow]` + 理由コメント）。
2. inherent な `fn from(` と、`get_` で始まるファクトリを検出して拒否する。
3. `new` と `try_new` が同じ型に共存していたら拒否する。
