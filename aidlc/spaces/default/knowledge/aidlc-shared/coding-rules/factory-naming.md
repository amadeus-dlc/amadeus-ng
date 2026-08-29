# ファクトリの命名 — コンストラクタ相当は `new`、それ以外は用途で名前を選ぶ

**裁定日**: 2026-08-24（オーナー）
**出典**: オーナー提示の命名表（Java 由来）を Rust へ翻訳したもの
**適用例**: U3（Bolt B5）— `EventStoreImpl::open` / `StorePath::for_space` / `WorkflowExecution::start` ほか。
命名監査の結果と、表の動詞へ矯正**しない**と決めた 25 件の反例カタログは
`<record>/construction/u3-event-store-repository/code-generation/naming-audit-report.md`。
**機械強制**: `cargo lint` ルール化候補（下記「機械化の候補」）。現状はレビュー基準

## 基本コンストラクタと補助コンストラクタ（オーナー明言 2026-08-24 — Scala の実践を Rust へ）

**setter は使わない。基本コンストラクタと補助コンストラクタを使う。**

setter を並べると「このケースでは setter A、このケースでは setter B」となり、
**何が初期化の基本なのかが不明になる**。不変条件がどこで確立されるのかも読めなくなる。

Scala が言語で強制している性質を Rust に持ち込む — 補助コンストラクタの最初の文は必ず
`this(...)`、すなわち**すべての構築経路が基本コンストラクタを通る**。

| Scala | Rust |
| --- | --- |
| 基本コンストラクタ（primary） | その型を作る**唯一の**関連関数。**構造体リテラルを書くのはここだけ** |
| 補助コンストラクタ（`def this(..)`） | 別の関連関数（`parse` / `from_*` / `of` / `open` …）。**必ず基本コンストラクタへ委譲する** |

### 検査可能な性質 — リテラルは 1 箇所

**構造体リテラル `Foo { .. }`（タプル構造体なら `Foo(..)`）が、型ごとに 1 箇所にしか
現れないこと。** これが「すべての構築経路が基本を通る」の機械的な言い換えである。
2 箇所以上に現れたら、その型は基本コンストラクタを持っていない。

### 基本コンストラクタが `new` とは限らない

どれが基本かは型による。**「1 つに定まっている」ことが要件**であって、名前ではない。

- 受け取った部品をそのまま組み立てる型 → `new(..)` が基本
- **AVDM の値オブジェクト** → `parse(&str) -> Result<Self, E>` が基本。
  検証を通らない構築口（生の `new(String)` など）を**並立させない** — 並立させた瞬間、
  不正な値を持つインスタンスが表現可能になり AVDM が壊れる
- 外部資源を開く型 → `open(..) -> Result<Self, E>` が基本

本リポジトリの例:
[`stage_slug.rs`](../../../../../../modules/core/domain/src/workflow_definition/stage_slug.rs) は
`parse` が基本コンストラクタで、リテラル `StageSlug(..)` は `parse` の中の 1 箇所だけに現れる。
検証を迂回する `new` は存在しない。**正しい形**。

反例（2026-08-24 実測、次 Bolt で是正）:
`WorkflowExecutionState` はリテラルが **2 箇所**（Builder の中と、集約の `state()` の中）に
現れ、基本コンストラクタが無い。`pub(crate)` フィールドがそれを可能にしていた
（[abstract-data-type.md](abstract-data-type.md) / [field-visibility.md](field-visibility.md)）。

### ビルダーの鎖メソッドは setter ではない — ファクトリメソッドである

**ビルダーは否定されない。setter を排除してもビルダーは作れる**（オーナー明言 2026-08-24）。
両者は形が違う。

```rust
fn set_name(&mut self, v: T)            // setter — その場で書き換え、戻り値なし
fn with_name(mut self, v: T) -> Self    // ファクトリメソッド — 消費して新しい値を作る
```

`mut self` を取って `Self` を返すものは**新しい値を生む**のでファクトリである。本ルールが
排除するのは前者であって後者ではない。

**命名は `with_<フィールド>`** にする。裸のフィールド名（`plan(..)` / `condition(..)`）は
**取得に読める** — `state.plan(x)` は「plan を返す」に見えてしまう。`with_plan(x)` なら
「plan を伴った新しい値」と読める。

### 既存の値から 1 つだけ変える — `to_builder()` で往復する

「setter が無いと 1 フィールドだけ変えられない」への答え（オーナー提示 2026-08-24）。

```rust
let new_person = person.to_builder().with_first_name("kato").build();
```

- **`to_builder()`** — 既存の値をビルダーへ戻す（C-CONV の `to_` = 所有を生む変換）
- **`with_*`** — ビルダー上のファクトリメソッド
- **`build()`** — 基本コンストラクタを呼んで値へ戻す

これで**値型は不変のまま**、変更の語彙はビルダーが持ち、**構築経路は 1 本のまま**保たれる。
`build()` が基本を通るので、何度往復しても不変条件の確立場所は 1 箇所である。

小さい型なら、ビルダーを挟まず**値型に直接 `with_*`** を置いてよい
（`ScopeMetadata::with_depth` / `WorkflowExecution::with_version` が本リポジトリの例）。
フィールドが増えて `with_*` が値型を埋め尽くしはじめたら、ビルダーへ移す合図である。

### ビルダーは基本コンストラクタを置き換えない

引数が多いなどでビルダーを置くなら、**`build()` が基本コンストラクタを呼ぶ**。ビルダーが
構造体リテラルを直接書いたら、それは 2 本目の構築経路であり本ルール違反である。
そもそも引数が多すぎるなら、**値オブジェクトへ束ね直す**のが先である。

本リポジトリの実測（2026-08-24）— 3 つとも `mut self -> Self` のファクトリメソッドで
setter は 1 つも無い。違いは命名だけ:

| 型 | 命名 | 判定 |
| --- | --- | --- |
| `ScopeMetadata::with_depth` ほか 5 本 | `with_` あり | **正しい** |
| `WorkflowExecutionStateBuilder::plan` ほか 12 本 | 裸のフィールド名 | `with_*` へ |
| `StageNodeBuilder::condition` ほか 21 本 | 裸のフィールド名 | `with_*` へ |

## 原則

**コンストラクタ相当のファクトリは `fn new(...) -> Self` に統一する**（オーナー明言）。
失敗しうる生成は `fn new(...) -> Result<Self, E>` とし、名前は `new` のままにする
（`try_new` のような別名を並立させない — [no-backward-compatibility.md](no-backward-compatibility.md)）。

コンストラクタ相当**でない**ファクトリは、下表の用途に従って名前を選ぶ。
名前が用途を語るので、doc を読まなくても何が起きるか予想がつく
（[interior-mutability.md](interior-mutability.md) の「シグニチャから内部の振る舞いが
予想できるのが良い設計」と同じ理由）。

**ただし本表は「他に言うことが無いとき」の既定である。** より正確な語があるなら、
そちらが勝つ。`hash_canonical(value) -> Digest` を `Digest::generate` に、
`serialize(value, profile) -> String` を `of` に矯正するのは**改悪**である — 前者は
「何を計算するか」を語っているのに、後者は「何かを作る」としか言っていない。
表の動詞は語彙が貧しいぶん適用範囲が広いだけで、優れているわけではない。

判断の順序は次のとおり:

1. **この関数が作るものを、ドメインが正確な語で呼んでいるか？** → その語を使う
   （`start` / `hash_canonical` / `serialize` / `to_value` / `open`）。
2. 呼んでいない → 表の用途に従う。
3. 表のどれにも当てはまらない → 何をするかを述べる名前を自分で選び、
   **なぜ表に載せなかったかを doc に一行書く**。

## 対応表（Rust 翻訳版）

| 名前 | 用途 | Rust での綴り | 例 |
| --- | --- | --- | --- |
| `new` | **コンストラクタ相当**（受け取った値をそのまま組み立てる） | `fn new(..) -> Self` / `-> Result<Self, E>` | `StageEntry::new(..)` |
| `of` | 複数の値を集約してインスタンスを生成（値オブジェクト） | `fn of(..) -> Self` | `ShardName::of(host, clone_id)` |

> **`of` の落とし穴**: 「与えた値を包む」ものにだけ使う。固定レイアウトの**導出**に `of` を
> 使うと、何を根拠に導出したかが名前から消える。実例 — `StorePath::of(root, space)` は一度
> 採用したが `StorePath::for_space(root, space)` へ戻した。後者は「space のためのパス」と
> 言えているが、前者は「root と space から何か作る」としか言っていない（命名監査 F8）。
> 表に当てはめること自体を目的にしない — 原則 1（正確な語が勝つ）が常に優先する。
| `from` | 他の型からの変換 | **`impl From<T>` / `impl TryFrom<T>` を第一選択**。inherent にするなら `from_<源の名前>` | `PhaseId::from_index(u32)`、`CheckboxState::from_marker(char)` |
| `parse` | 文字列を解析して生成 | `fn parse(s: &str) -> Result<Self, E>`（可能なら `impl FromStr` も） | `IntentId::parse(&str)` |
| `create` | 新しいエンティティ／ドメインオブジェクトを作る | ドメイン語があればそちらを優先、無ければ `create` | 集約の genesis は `IntentExecution::start(id, intent, at)`（旧 `WorkflowExecution::start` — 分割・改名 2026-08-29） |
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

## 集約の基本コンストラクタは「全情報を受ける genesis」（オーナー裁定 2026-08-30）

集約では**基本コンストラクタ = genesis**（対を返す。[aggregate-commands.md](aggregate-commands.md)）
であり、**情報を削った側を基本と呼ばない**。`Intent` の実例で確定した:

- `Intent::create(id, &WorkflowDefinition, start_request, scan) -> Result<(Intent, IntentEvent), _>`
  が基本 — 定義そのもの（全情報）から計画を解決し、対を返す。動詞 `create` は upstream の
  `intent-create`（ドメイン語優先）。
- 再構成（`replay` / `From<Created>`）はファクトリではないので、補助→基本の委譲規律の対象外。
  ただし作ってよい構築口は genesis / `replay` / `apply_event` だけである（同裁定）。

**誤適用の経緯**: `Intent::from_material(6 値)` は「受け取った部品を組み立てるコンストラクタ相当」
なのに `from_` を名乗った違反だった（`Material` という源の型は存在しない — `from_<源>` は実在する
型・成果物に限る）。さらに genesis と同一引数列の双子（DRY 違反）でもあった。是正の途中で
`restore` / `rehydrate`（第 3 の構築口）や memento 双子型（`IntentSnapshot`）を経由しかけたが、
いずれも同裁定で否定された — 詳細は aggregate-commands.md「再構成の形」。

## 対象外

- **ビルダーの終端** `fn build(self) -> T` は本表の対象外（ビルダーパターンの語）。
- **変換メソッド**（`as_*` / `to_*` / `into_*`）は生成ではなく変換なので、Rust API
  ガイドライン C-CONV に従う。
- `&mut self` を受けて自身を書き換えるコマンドはファクトリではない
  （[command-query-separation.md](command-query-separation.md)）。

## 機械化の候補

優先順は 型 → 既存 lint → `cargo lint` カスタムルール（README の方針）。赤例テスト必須。

**やってはいけない機械化**: 「戻り値が `Self` の関連関数は、名前が許可リストのいずれかで
なければ拒否する」という形。これは上の原則 1（正確なドメイン語が勝つ）と正面から衝突し、
`hash_canonical` / `serialize` / `to_value` / `start` のような**良い名前を軒並み誤検出する**。
誤検出の多いルールは `#[allow]` を量産させ、やがて誰も読まなくなる。命名の良し悪しは
本質的にレビューの仕事であり、機械に渡せるのは**例外が存在しない狭い部分だけ**である。

機械化してよいのは、次の 3 つのように**反例が構造的に存在しない**検査に限る。

1. **inherent な `fn from(`** — `From::from` と綴りが衝突し、呼出側でどちらが呼ばれるか
   読めなくなる。正当な例外は無い（変換したいなら `impl From` か `from_<源>`）。
2. **`get_` 接頭辞のファクトリ** — Rust API ガイドライン C-GETTER が禁じている。
   正当な例外は無い。
3. **同じ型に `new` と `try_new` が共存** — 同じ用途の入口が 2 つある状態そのものが違反
   （[no-backward-compatibility.md](no-backward-compatibility.md)）。正当な例外は無い。

### 広いルールを機械化する道 — 例外に**理由を書かせる**（オーナー提案 2026-08-24）

上の 3 つより広い検査（「戻り値が `Self` の関連関数で、名前が表の動詞でないものを報告する」）
も、**例外に理由の記述を強制すれば**使えるようになる。改名を強制するのではなく、
**説明を強制する**ルールになるからである。`hash_canonical` の作者は「ドメイン語のほうが
正確」と一行書けば通り、誤検出は `#[allow]` の量産ではなく**根拠の蓄積**に変わる。

`cargo lint` の抑制規約はこれを満たすよう実装済み（`tools/lint/src/check.rs`）:

```rust
// ❌ 抑制されない — 理由が無い
// amadeus-lint: allow(factory-naming)

// ❌ 抑制されない — 区切り記号だけ
// amadeus-lint: allow(factory-naming) —

// ✅ 抑制される — 区切り記号は問わない、何か書いてあればよい
// amadeus-lint: allow(factory-naming) ドメイン語のほうが正確 — 何を計算するかを名前が語る
```

理由の**質**は機械に測れない（それはレビューの仕事）。機械が保証できるのは
「例外を使うなら根拠が同じ場所に書いてある」ことだけであり、それで十分に価値がある。

ただし広い検査を実際に足す前に、**リポジトリ全体の正当な例外を先に洗い出すこと**。
例外の総数が分からないまま入れると、初回の実行で大量の所見が出て、理由を書く作業が
機械的な儀式に堕する。まず一覧を作り、それぞれに理由を書けるか確かめてから有効化する。
