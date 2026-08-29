# aidlc/spaces/default/knowledge/aidlc-shared/coding-rules — 設計ルールの正本（人間・全エージェント共有）

オーナー裁定で確定した**リポジトリ横断の設計ルール**を置く。特定エージェントのプライベートメモリには置かない（アクセスできない主体が出るため）。1 ルール 1 ファイル。ルールを追加・改訂したら本 README の一覧も更新する。

各ルールには裁定日・適用例（PR）・機械強制の有無（`cargo lint` ルール / clippy / 型）を記す。**設計の前提はまずスキル正典から** — 設計・命名・配置を自分で考える前に、インストール済みの j5ik2o-* 設計スキル（software-design プラグイン約 29 本。`j5ik2o-ddd-repository-design` / `-repository-placement` / `-custom-linter-creator` 等）を列挙し、該当スキルの SKILL.md と references/ を**先に**読む。前提はスキルに書いてある（実例: Reader 造語と `load` メソッドはどちらもスキル未読のまま設計して差し戻された）。**オーナーの指摘（裁定）は可能な限り機械的な強制へ落とし込む** — 優先順は 型（E1）→ 既存 lint（clippy / rustc）→ `cargo lint` カスタムルール。カスタムルールは検出力を証明する赤例テストが必須（Quint ゲートと同じ DoD）。仕様（upstream 互換の観測可能契約）は `docs/specs/` が正本であり、ここに置くのは**書き方のルール**である。


## 規則が衝突したら（優先順）

規則が 13 本になり、全文を頭に入れて衝突を裁定する前提は成立しない。**読み替えて進まず、
その場で正本を直す**（`project.md` Corrections「上流成果物の矛盾は読み替えず裁定を求める」）。
どちらが正かは次の順で決める。

1. **観測互換**（`docs/specs/` の upstream 契約）— これだけは設計規則より上。
   Published Language の逐語は常に勝つ。
2. **「例外を認めない」と明記した規則** — 現在は [field-visibility.md](field-visibility.md) のみ
   （ただし「例外なし」は**射程の中で**の話。射程外＝対象外は各規則の §射程 / §対象外 を見る）。
3. **土台の「目的」** — [abstract-data-type.md](abstract-data-type.md) の目的（**呼び手が表現を
   知らずに済むこと**）。派生規則の文面がこの目的を裏切るなら、派生規則の文面が誤りである。
   **文面ではなく目的が勝つ** — 土台の文面を優先すると派生規則の運用が止まる場面があるため。
4. **各規則の本文** — 裁定日ヘッダで比較し、新しいほうが勝つ。
5. **表・語彙の既定**（[factory-naming.md](factory-naming.md) の対応表など）— 最も弱い。
   より正確なドメイン語があればそちらが勝つ、と規則自身が既に書いている。

**衝突の実例と是正は [CONSISTENCY-AUDIT-2026-08-24.md](CONSISTENCY-AUDIT-2026-08-24.md)。**
2026-08-24 の監査で Critical 3・Major 8・Minor 6 が出た。Critical 3 は是正済み。

**土台は [abstract-data-type.md](abstract-data-type.md)** — AVDM / DP は抽象データ型であり、
操作（契約）で定義され表現では定義されない。内部構造を暴露せず、呼び手を契約にだけ依存させる。
field-visibility / tell-dont-ask / factory-naming / CQS / domain-equality / ubiquitous-language は
いずれもここから導かれる帰結である。

**良い例は [good-examples.md](good-examples.md) に索引がある** — 規則の文面に対して
「この形」と指せる実在ファイルの一覧。スニペットを書き写さずファイルを指すので、コードが
変われば例も追随する。リンク切れは所見として扱う（カタログを直す前に「なぜ動いたか」を確認）。

| ルール | 一言 | 機械強制 |
| --- | --- | --- |
| [abstract-data-type.md](abstract-data-type.md) | **土台** — AVDM / DP は抽象データ型。操作（契約）で定義され表現では定義されない。内部構造を暴露せず、呼び手を契約にだけ依存させる。カプセル化の単位は `struct` であって `mod` | 部分的（`cargo lint` の no-public-fields ほか） |
| [good-examples.md](good-examples.md) | 規則の文面に対して「この形」と指せる**実在ファイルの索引**。スニペットを書き写さないのでコードが変われば例も追随する | — |
| [tell-dont-ask.md](tell-dont-ask.md) | getter は存在してよいが濫用禁止 — 判断は状態の所有者へ。**アクセサを `value()`/`inner()`/`raw()` と名乗って内部型を意識させない**（2026-08-24 追記） | `cargo lint`（checkbox-vocabulary） |
| [domain-equality.md](domain-equality.md) | ドメイン同値関係は `Eq`/`PartialEq` で表現 — 名前付き比較メソッド禁止 | レビュー基準 |
| [field-visibility.md](field-visibility.md) | フィールドはデフォルト private — 公開はアクセサ経由。**`pub` も `pub(crate)` も禁止で例外を認めない**（2026-08-24 改訂。検出境界の拡張は既存違反の是正と同じ Bolt で着地させる） | `cargo lint`（no-public-fields。境界拡張は機械化ロードマップ 2） |
| [module-visibility.md](module-visibility.md) | mod はデフォルト private — 公開はファサードの `pub use` 経由。利便性のための再エクスポートはどこでも禁止（所有元が読めなくなる） | `unreachable_pub`（私有 mod 化で実効化） |
| [gateway-taxonomy.md](gateway-taxonomy.md) | Gateway 責務は Repository と外部システムクライアントの 2 つ — Repository 名は集約名から取る（Store/Reader/Writer 造語と媒体名は禁止）。機構（時計・ID・プロセス生存）は Gateway ではない。ES Repository は `store` / `find_by_id`（ADR-006） | レビュー基準 |
| [use-case-rules.md](use-case-rules.md) | DIP（trait のみ依存）・スタティックバインディング既定・ユースケース間呼出禁止 | Cargo クレート分離 |
| [error-handling.md](error-handling.md) | 失敗はモジュールごとの手実装エラー enum — `Display` は材料のみ、利用者向け文言はアダプタ層（message-catalog）、thiserror / anyhow 不使用 | `missing_errors_doc` / `missing_panics_doc` / `unwrap_used` / `expect_used` deny（workspace lints） |
| [interior-mutability.md](interior-mutability.md) | 内部可変性は既定で禁止 — 可変操作はまず `&mut self`。`&self` の裏に `RefCell`/`Cell`/ロックを置く「`&self` への偽装」は禁止。`&self` + 内部可変性には**強い理由**が要る（立証責任は採る側。現在認められている例外はロックを取り合うメソッドのみ、条件付き）。並行してロックを取りたい場合は `SharedLock`/`SharedRwLock` を持つ `*Shared` ラッパーへ閉じる（手書きの `Rc<RefCell<_>>`/`Arc<Mutex<_>>` は禁止） | レビュー基準 |
| [command-query-separation.md](command-query-separation.md) | Query は `&self` + 戻り値、Command は `&mut self` + 戻り値なし or `Result<(), E>`。分離不能ならオーナー許可のうえ理由をコメントに書く | レビュー基準 |
| [no-backward-compatibility.md](no-backward-compatibility.md) | 後方互換のコードを残さない — `#[deprecated]`・旧名エイリアス・`pub use .. as`・互換口の並立を禁止。改名や署名変更は呼出側ごと一斉に直す（未配布のため互換の対価が無い。upstream 互換は別問題） | レビュー基準（機械化ロードマップ 4） |
| [domain-services.md](domain-services.md) | ドメインサービスは**最後の手段** — 構築規則・導出・判断はまず所有する型の関連メソッドへ。自由関数は「どの型も所有できない」説明を doc に書けるときだけ | レビュー基準 |
| [infrastructure-layer.md](infrastructure-layer.md) | infrastructure 層は**言語拡張**（原子的 I/O・時計・ID・ロギング等の汎用機構）だけ — **RPC クライアント・DB アクセスは置かない**（相手方契約を知る gateway は interface-adapter へ）。配置は core-infrastructure / harness-infrastructure | Cargo クレート分離 + レビュー基準 |
| [factory-naming.md](factory-naming.md) | **基本コンストラクタ 1 本に構築経路を集約**し、補助コンストラクタは必ずそれへ委譲する（Scala の primary/auxiliary を Rust へ。検査可能な性質 = 構造体リテラルが型ごとに 1 箇所）。setter は使わない。コンストラクタ相当は `fn new(..) -> Self` に統一。それ以外は用途で選ぶ（`of` 集約 / `from`(`From`・`from_<源>`) 変換 / `parse` 文字列 / `open` リソース / `generate` 算出 / `create` エンティティ、ドメイン語があれば優先）。`valueOf`・`getInstance`・`newInstance` は Rust 慣用と衝突するので不採用 | レビュー基準（機械化ロードマップ 1・3） |
| [ubiquitous-language.md](ubiquitous-language.md) | ドメインモデル（`core/domain` の集約・エンティティ・値オブジェクト・ドメインイベント）の型名・フィールド名・メソッド名はユビキタス言語にする。例外は認めるが**doc コメントに理由の記述が必須** | レビュー基準 |
| [upstream-contracts.md](upstream-contracts.md) | **借り物の契約を自分のドメインに合わせて曲げない**。ライブラリには別のドメインがある。取りうる関係は Conformist か腐敗防止層の 2 つで、契約を書き換えるのはどちらでもない。食い違いは**境界で変換**する | レビュー基準 |
| [domain-persistence-neutrality.md](domain-persistence-neutrality.md) | **ドメインは永続化知識から中立** — serde 属性・ストア trait 実装・ジャーナル語彙・復号中間表現を domain に書かない。永続化モデル（DTO）はアダプタが所有し、復号は検査付き再構成コンストラクタへ渡す。読む側（RMU）は自前 DTO（側ごと専用化） | クレート依存（domain の Cargo.toml に serde / ESA が無いこと）+ レビュー基準 |
| [aggregate-commands.md](aggregate-commands.md) | **集約のコマンド（`&mut self` の状態遷移）は必ず単一のドメインイベントを戻り値で返す**（decide / apply 分離・1 コマンド 1 イベント・拒否はガード付き Err）。CQS の「Command は戻り値なし」は集約には適用しない — イベントは書込の産物であり読取チャネルではない | レビュー基準（`cargo lint` ルール候補） |
| [aggregate-references.md](aggregate-references.md) | **集約は他の集約・エンティティを ID で参照する** — オブジェクトの埋め込み禁止（1:n で複製を抱え、整合性境界が壊れる）。判断に要るデータは `&` 参照のメソッド引数で渡し、`id` 照合でガードする。イベントが材料の複製を運ぶのは歴史であり違反ではない | レビュー基準（`cargo lint` ルール候補） |
| [cqrs-boundaries.md](cqrs-boundaries.md) | コマンド側とクエリ側は相互に依存しない。**RMU だけが両側に依存できる**（橋）。**コマンド側の最新状態は常に集約から**（リードモデルは常に遅延しているので物理的に読めない）。境界は**クレート分離**で物理強制する（mod 分割では効かない） | クレート分離（`Cargo.toml` の不在）— 違反はビルドで落ちる |

---

## 機械化ロードマップ（2026-08-24 制定）

**実測**: `cargo lint`（`tools/lint/src/check.rs`）に実装済みのルールは **2 本**
（`checkbox-vocabulary` / `no-public-fields`）。一方、各規則に散らばる「ルール化予定 / 候補」は
**8 本**ある。**予定が実装の 4 倍あるのは規則の信頼を下げる** — 「そのうち機械が見てくれる」と
読まれ、レビューでの適用が緩む。

そこで、**順序と着手条件をここ 1 箇所で管理する**。個々の規則に「予定」と書き足すのをやめる
（規則側は「レビュー基準」か「`cargo lint`（ルール名）」のどちらかだけを書く）。

### 着手の条件（3 つとも満たすもののみ実装する）

1. **反例が構造的に存在しない**か、**例外に理由を書かせれば足りる**
   （[factory-naming.md](factory-naming.md) §「機械化の候補」の判断）
2. **赤例テストが書ける**（検出力を証明できる。README 冒頭の DoD）
3. **既存違反の是正と同じ Bolt で着地できる**（検出だけ先行させると CI が赤のまま残る）

### 優先順（次の Bolt から順に）

| 順 | ルール | 根拠となる規則 | 着手条件の充足 |
| --- | --- | --- | --- |
| 1 | **構造体リテラルは型ごとに 1 箇所** | [factory-naming.md](factory-naming.md)（基本コンストラクタ） | 反例ほぼ無し。是正対象は `WorkflowExecutionState` の 1 型 |
| 2 | **`pub(crate)` / `pub(super)` フィールド**（`no-public-fields` の境界拡張） | [field-visibility.md](field-visibility.md) | 例外を認めない裁定済み。是正対象は 1 と同じ型 — **同じ Bolt で 1 と一緒に** |
| 3 | **inherent な `fn from(`** | [factory-naming.md](factory-naming.md) | 反例無し。現状の違反 0 件なので単独で着地できる |
| 4 | **`#[deprecated]` の検出** | [no-backward-compatibility.md](no-backward-compatibility.md) | 反例無し。現状の違反 0 件 |

### 実装しないと決めたもの

| ルール | 理由 |
| --- | --- |
| 「戻り値が `Self` の関連関数は名前が許可リストのいずれか」 | 誤検出が多すぎる。正確なドメイン語（`hash_canonical` / `serialize`）を軒並み潰す（[factory-naming.md](factory-naming.md) §「やってはいけない機械化」） |
| `of` / `from_*` / ドメイン語のどれを選ぶべきかの判定 | 意味の判断であり機械に渡せない。レビュー基準のまま |

### 未定（着手条件を満たすか未検証）

`interior-mutability` / `command-query-separation` / `module-visibility` / `use-case-rules` /
`gateway-taxonomy` / `error-handling`（thiserror・anyhow 禁止）の各候補。
**上の 4 本を実装してから、改めて条件 1〜3 に照らして判断する。** それまで「予定」とは書かない。

