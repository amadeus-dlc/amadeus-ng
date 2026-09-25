# aidlc/spaces/default/knowledge/aidlc-shared/coding-rules — 設計ルールの正本（人間・全エージェント共有）

オーナー裁定で確定した**リポジトリ横断の設計ルール**を置く。特定エージェントのプライベートメモリには置かない（アクセスできない主体が出るため）。1 ルール 1 ファイル。ルールを追加・改訂したら本 README の一覧も更新する。

各ルールには裁定日・適用例（PR）・機械強制の有無（`cargo lint` ルール / clippy / 型）を記す。**設計の前提はまずスキル正典から** — 設計・命名・配置を自分で考える前に、インストール済みの j5ik2o-* 設計スキル（software-design プラグイン約 29 本。`j5ik2o-ddd-repository-design` / `-repository-placement` / `-custom-linter-creator` 等）を列挙し、該当スキルの SKILL.md と references/ を**先に**読む。前提はスキルに書いてある（実例: Reader 造語と `load` メソッドはどちらもスキル未読のまま設計して差し戻された）。**オーナーの指摘（裁定）は可能な限り機械的な強制へ落とし込む** — 優先順は 型（E1）→ 既存 lint（clippy / rustc）→ `cargo lint` カスタムルール。カスタムルールは検出力を証明する赤例テストが必須（Quint ゲートと同じ DoD）。仕様（upstream 互換の観測可能契約）の正本はゴールデン `tests/golden/upstream-a277af21/`（ピン `a277af21` = v2.7.1 の配布実バイト）と配布元 submodule `vendor/aidlc-workflows/`であり（手書きの `docs/specs/` は 2026-09-07 に削除した）、ここに置くのは**書き方のルール**である。


## 2026-09-08に再確認した必須規則

- 集約IDの型名は **集約名 + `Id`**。[ubiquitous-language.md](ubiquitous-language.md) の対応規則で確認する。
- 完成型の初期化は **完全コンストラクタ** に集約し、**setterは禁止**する。初期化漏れと、ドメインの文脈を無視した任意の状態変更を防ぐためである。
- **`with_*` はビルダーのファクトリメソッドであり、setterとは区別する。** `build()` は完成型の完全コンストラクタを呼ぶ。詳しい判定と点検項目は [factory-naming.md](factory-naming.md) を参照する。
- **ユースケースで業務判断のためにドメインのgetterを呼ばない。Repositoryの `find_by_id` への型付きIDの直接受け渡しだけは許可する（2026-09-14）。** ドメインモデル貧血症を防ぐため、判断を状態の所有者へ委譲する。実装・是正の区切りで `cargo lint` を実行する。既存の禁止範囲と点検方法は [tell-dont-ask.md](tell-dont-ask.md) を参照する。
- **ドメインのFCCの要素はドメイン固有型とし、プリミティブ型を使わない。** `PendingIterations` の要素は `PendingIteration` とする。既存移行とリンター追加は利用者が許可した後続Issueで管理する。詳細は [first-class-collections.md](first-class-collections.md) を参照する。

記録だけで是正済みとは扱わず、コードの構築・更新経路を点検し、結果を現在のintent記録へ残す。

## 規則が衝突したら（優先順）

規則が 23 本になり、全文を頭に入れて衝突を裁定する前提は成立しない。**読み替えて進まず、
その場で正本を直す**（上流成果物の矛盾は読み替えず裁定を求める、というオーナー規律 2026-08-22 の適用。
旧 memory 層 `project.md` の Corrections に記録されていたが、memory 層は 2026-09-07 に初期化したため本節が正本である）。
どちらが正かは次の順で決める。

1. **観測互換**（ゴールデン `tests/golden/upstream-a277af21/`（ピン `a277af21` = v2.7.1 の配布実バイト）と配布元 submodule `vendor/aidlc-workflows/`が定める upstream 契約）— これだけは設計規則より上。
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

**パッケージングの根拠は [packaging-principles-research-20260925.md](../../packaging-principles-research-20260925.md)** — 良い分割の条件（変わる理由で切る・依存の向き・深いモジュールなど）を文献で確かめた調査記録。規則ではなく根拠として参照する。

**良い例は [good-examples.md](good-examples.md) に索引がある** — 規則の文面に対して
「この形」と指せる実在ファイルの一覧。スニペットを書き写さずファイルを指すので、コードが
変われば例も追随する。リンク切れは所見として扱う（カタログを直す前に「なぜ動いたか」を確認）。

| ルール | 一言 | 機械強制 |
| --- | --- | --- |
| [abstract-data-type.md](abstract-data-type.md) | **土台** — AVDM / DP は抽象データ型。操作（契約）で定義され表現では定義されない。内部構造を暴露せず、呼び手を契約にだけ依存させる。カプセル化の単位は `struct` であって `mod`。**1 ファイル 1 公開型**（2026-09-01 改訂 — 全層へ拡張） | 部分的（`cargo lint` の no-public-fields / one-public-type / public-type-file-name） |
| [good-examples.md](good-examples.md) | 規則の文面に対して「この形」と指せる**実在ファイルの索引**。スニペットを書き写さないのでコードが変われば例も追随する | — |
| [tell-dont-ask.md](tell-dont-ask.md) | **ユースケースの業務判断にgetterを使わない**。Repositoryの `find_by_id` への型付きIDの直接受け渡しは許可（2026-09-14）。アダプタ層でのgetterは合法。判断は状態の所有者へ。`value()`/`inner()`/`raw()`で内部型を意識させない | `cargo lint`（checkbox-vocabulary / use-case-domain-getter） |
| [domain-equality.md](domain-equality.md) | ドメイン同値関係は `Eq`/`PartialEq` で表現 — 名前付き比較メソッド禁止 | レビュー基準 |
| [first-class-collections.md](first-class-collections.md) | コレクション操作（`filter` / `map` / `fold_left` / `at`）を優先し、型の意味に沿った `combine` / `divide` を先に選ぶ。イテレータ公開は最後の手段で、例外は理由付きの境界処理のみ（裁定日 2026-09-06 — 従来この告知は「規則が衝突したら」節の中に置かれていたが、一覧表の本行へ畳んだ） | 設計・レビュー基準（全型への一律lintは未実装） |
| [field-visibility.md](field-visibility.md) | フィールドはデフォルト private — 公開はアクセサ経由。**`pub` も `pub(crate)` も禁止で例外を認めない**（2026-08-24 改訂。検出境界の拡張は既存違反の是正と同じ Bolt で着地させる） | `cargo lint`（no-public-fields。境界拡張は機械化ロードマップ 2） |
| [module-visibility.md](module-visibility.md) | mod はデフォルト private — 公開はファサードの `pub use` 経由。利便性のための再エクスポートはどこでも禁止（所有元が読めなくなる） | `unreachable_pub`（私有 mod 化で実効化） |
| [gateway-taxonomy.md](gateway-taxonomy.md) | ドメインの Gateway 責務は Repository と外部システムクライアントの 2 つ（加えて ES 永続化基盤ポート `EventStore` / `JournalReader` を第三の責務として §1c が定める — Repository の下請けで、ユースケースへ直接注入しない） — Repository 名は集約名から取る（Store/Reader/Writer 造語と媒体名は禁止）。機構（時計・ID・プロセス生存）は Gateway ではない。ES Repository は `store` / `find_by_id`（ADR-006）。**コマンド側で外界（fs / 乱数 / プロセス / ネットワーク）に触るのは Repository 実装だけ**（§1d、2026-09-04） | `cargo lint`（`port-naming` — use-case 層の `pub trait` はコマンド側 `XxxRepository` / クエリ側 `XxxDao` のみ。`command-side-io` — `modules/core/command/**` の `*_repository_impl.rs` 以外に fs / 乱数 / プロセス / ネットワークの I/O が現れたら所見。2026-09-04、#47 / b44）。Repository 名と集約名の照合・技術接頭辞はレビュー基準 |
| [use-case-rules.md](use-case-rules.md) | DIP（trait のみ依存）・スタティックバインディング既定・ユースケース間呼出禁止 | Cargo クレート分離 |
| [error-handling.md](error-handling.md) | 失敗はモジュールごとの手実装エラー enum — `Display` は材料のみ、利用者向け文言は**出す側の `wording` モジュール**（合成ルート `aidlc` と RMU の投影ライタ）、thiserror / anyhow 不使用（2026-08-29 の文言カタログ解体後の形へ同期 — 本文 error-handling.md の該当箇条と一致。実測 `modules/app/aidlc/src/wording.rs` / `modules/core/read-model-updater/src/workspace/wording.rs`） | `missing_errors_doc` / `missing_panics_doc` / `unwrap_used` / `expect_used` deny（workspace lints） |
| [interior-mutability.md](interior-mutability.md) | 内部可変性は既定で禁止 — 可変操作はまず `&mut self`。`&self` の裏に `RefCell`/`Cell`/ロックを置く「`&self` への偽装」は禁止。`&self` + 内部可変性には**強い理由**が要る（立証責任は採る側。現在認められている例外はロックを取り合うメソッドのみ、条件付き）。並行してロックを取りたい場合は `SharedLock`/`SharedRwLock` を持つ `*Shared` ラッパーへ閉じる（手書きの `Rc<RefCell<_>>`/`Arc<Mutex<_>>` は禁止） | レビュー基準 |
| [command-query-separation.md](command-query-separation.md) | Query は `&self` + 戻り値、Command は `&mut self` + 戻り値なし or `Result<(), E>`。分離不能ならオーナー許可のうえ理由をコメントに書く | レビュー基準 |
| [no-backward-compatibility.md](no-backward-compatibility.md) | 後方互換のコードを残さない — `#[deprecated]`・旧名エイリアス・`pub use .. as`・互換口の並立を禁止。改名や署名変更は呼出側ごと一斉に直す（未配布のため互換の対価が無い。upstream 互換は別問題） | レビュー基準（機械化ロードマップ 4） |
| [domain-object-kinds.md](domain-object-kinds.md) | ドメインオブジェクトは**エンティティ**（集約ルート = グローバル / ローカル）・**値オブジェクト**・**ファーストクラスコレクション**・**ドメインイベント**の 4 種が基本。**ドメインサービスの新設は人間の裁定が必須**。それ以外の種類は実測ありの問題と対策内容を添えて人間の裁定にかけてから（2026-09-02 オーナー規律） | レビュー基準 |
| [domain-services.md](domain-services.md) | ドメインサービスは**最後の手段** — 構築規則・導出・判断はまず所有する型の関連メソッドへ。自由関数は「どの型も所有できない」説明を doc に書けるときだけ | レビュー基準 |
| [domain-packaging.md](domain-packaging.md) | **ドメイン層はドメインの概念で分ける — 技術駆動パッケージングの禁止**（2026-09-25 オーナー裁定）。最上位は境界づけられたコンテキスト、その下は特定の型が所有するサブツリー（イベント族の変種・従属部品）だけ。`entities/`・`value_objects/`・`events/`・`services/` のような種類別の区切りは作らない | `cargo lint`（`domain-packaging`） |
| [infrastructure-layer.md](infrastructure-layer.md) | infrastructure 層は**言語拡張**（原子的 I/O・時計・ID・ロギング等の汎用機構）だけ — **RPC クライアント・DB アクセスは置かない**（相手方契約を知る gateway は interface-adapter へ）。配置は core-infrastructure / harness-infrastructure | Cargo クレート分離 + レビュー基準 |
| [factory-naming.md](factory-naming.md) | **基本コンストラクタ 1 本に構築経路を集約**し、補助コンストラクタは必ずそれへ委譲する（Scala の primary/auxiliary を Rust へ。検査可能な性質 = 構造体リテラルが型ごとに 1 箇所）。setter は使わない。コンストラクタ相当は `fn new(..) -> Self` に統一。それ以外は用途で選ぶ（`of` 集約 / `from`(`From`・`from_<源>`) 変換 / `parse` 文字列 / `open` リソース / `generate` 算出 / `create` エンティティ、ドメイン語があれば優先）。`valueOf`・`getInstance`・`newInstance` は Rust 慣用と衝突するので不採用 | `cargo lint`（`setter-method`）。完全初期化・構築経路・その他の命名はレビュー基準 |
| [ubiquitous-language.md](ubiquitous-language.md) | ドメインモデル（`core/domain` の集約・エンティティ・値オブジェクト・ドメインイベント）の型名・フィールド名・メソッド名はユビキタス言語にする。例外は認めるが**doc コメントに理由の記述が必須** | レビュー基準 |
| [upstream-contracts.md](upstream-contracts.md) | **借り物の契約を自分のドメインに合わせて曲げない**。ライブラリには別のドメインがある。取りうる関係は Conformist か腐敗防止層の 2 つで、契約を書き換えるのはどちらでもない。食い違いは**境界で変換**する | レビュー基準 |
| [domain-persistence-neutrality.md](domain-persistence-neutrality.md) | **ドメインは永続化知識から中立** — serde 属性・ストア trait 実装・ジャーナル語彙・復号中間表現を domain に書かない。永続化モデル（DTO）はアダプタが所有し、復号は検査付き再構成コンストラクタへ渡す。読む側（RMU）は自前 DTO（側ごと専用化） | クレート依存（domain の Cargo.toml に serde / ESA が無いこと）+ レビュー基準 |
| [aggregate-commands.md](aggregate-commands.md) | **集約のコマンド（`&mut self` の状態遷移）は必ず単一のドメインイベントを戻り値で返す**（decide / apply 分離・1 コマンド 1 イベント・拒否はガード付き Err）。CQS の「Command は戻り値なし」は集約には適用しない — イベントは書込の産物であり読取チャネルではない | レビュー基準（`cargo lint` ルール候補） |
| [aggregate-references.md](aggregate-references.md) | **集約は他の集約・エンティティを ID で参照する** — オブジェクトの埋め込み禁止（1:n で複製を抱え、整合性境界が壊れる）。判断に要るデータは `&` 参照のメソッド引数で渡し、`id` 照合でガードする。イベントが材料の複製を運ぶのは歴史であり違反ではない | レビュー基準（`cargo lint` ルール候補） |
| [cqrs-boundaries.md](cqrs-boundaries.md) | コマンド側とクエリ側は相互に依存しない。**RMU だけが両側に依存できる**（橋）。**コマンド側の最新状態は常に集約から**（リードモデルは常に遅延しているので物理的に読めない）。境界は**クレート分離**で物理強制する（mod 分割では効かない）。**DAO は 1 表 1 引当**（規則 6、2026-09-03） | クレート分離（`Cargo.toml` の不在）— 違反はビルドで落ちる。加えて `cargo lint`（`dao-single-table` / `read-model-updater-contract`） |

---

## 機械化ロードマップ（2026-08-24 制定）

**実測**: `cargo lint`（`tools/lint/src/check.rs`）に実装済みのルールは **2 本**
（`checkbox-vocabulary` / `no-public-fields`）。一方、各規則に散らばる「ルール化予定 / 候補」は
**8 本**ある。**予定が実装の 4 倍あるのは規則の信頼を下げる** — 「そのうち機械が見てくれる」と
読まれ、レビューでの適用が緩む。

**更新 2026-09-01**: `one-public-type`（1 ファイル 1 公開型 —
[abstract-data-type.md](abstract-data-type.md) §改訂 2026-09-01）が加わり実装済みは **3 本**。
下表の 4 本より先に着地したのは、同日のオーナー裁定で新設された規則であり待ち行列に並んで
いなかったため。着手条件 1〜3 は充足済み（反例が構造的に無い / 赤例テスト同梱 / 検出と
実測 71 ファイルの是正を b32 の同一 Bolt で着地）。

**更新 2026-09-03**: `dao-single-table`（クエリ側 DAO の SQL は 1 文 1 表 —
[cqrs-boundaries.md](cqrs-boundaries.md) 規則 6「表の形と読み方」）が加わり実装済みは **4 本**。
`one-public-type` と同じく、同日のオーナー裁定で新設された規則なので下表の待ち行列に並んで
いない。着手条件 1〜3 の充足: (1) 裁定の逐語「JOIN しない」に留保が無く反例が構造的に無い
（将来の例外は理由付き allow で通す）、(2) 赤例 4 形（素の文字列リテラルの JOIN /
`concat!` 内の JOIN / `macro_rules!` 本体内 `concat!` の JOIN / `EXISTS` 副問合せ）を
b43 の作業ツリーの現物から採ってテストに同梱、(3) 検出と JOIN 解体を b43 の同一 Bolt で着地。

**更新 2026-09-04**: `port-naming`（use-case 層の `pub trait` はコマンド側 `XxxRepository` /
クエリ側 `XxxDao` のみ — [gateway-taxonomy.md](gateway-taxonomy.md) §1・§3・§5）と
`command-side-io`（`modules/core/command/**` の `*_repository_impl.rs` 以外に fs / 乱数 /
プロセス / ネットワークの I/O が現れたら所見 — 同 §1d）が加わり実装済みは **6 本**。
どちらも GitHub #47（オーナー示唆 2026-08-30「コマンド側でリポジトリ以外の I/O 責務を作ろうと
したら警告する仕組みが要る」）を是正 Bolt 3 後半（b44）に折り込んだもので、下表の待ち行列には
並んでいない。着手条件 1〜3 の充足: (1) #47 の文言に留保が無く、正当な例外（外部システム
クライアント `XxxClient` / 理由のある I/O）は理由付き allow で通す、(2) 赤例（R6 4 形 / R7 6 形）と
「同じソースが射程内では鳴る」対を同梱、(3) 導入時点の既存違反は 0 件（是正対象なし — 検出だけが
先行して CI が赤になる状態は生じない）。[gateway-taxonomy.md](gateway-taxonomy.md)「機械強制の
候補」1（ポート造語の検出）は `port-naming` が上位互換（禁止語の黒リストではなく許可接尾辞の
白リスト）として吸収した。

**更新 2026-09-25**: `public-type-file-name`（公開型が 1 つのファイルは、ファイル名がその型名の
snake_case — [abstract-data-type.md](abstract-data-type.md)）が加わり実装済みは **7 本**。
`one-public-type` が数だけを見ていたため、規則の後半（ファイル名）が機械強制されていなかった
穴を塞いだもの。着手条件 1〜3 の充足: (1) 規則の文面に留保が無く、正当な形（ファサード・
入口・イベント族の変種ファイル）は構造で除外できる、(2) 実在した 3 形（型名と無関係な名前・
自由関数モジュールへのエラー型の同居・イベント族の名前の入れ替わり）を赤例として同梱、
(3) 既存違反 10 件の是正を同じ変更で着地。

**更新 2026-09-25（2）**: `domain-packaging`（ドメイン層の技術駆動パッケージングの禁止 —
[domain-packaging.md](domain-packaging.md)）が加わり実装済みは **8 本**。規則の新設と同時に機械化した。
着手条件 1〜3 の充足: (1) 禁止する名前の完全一致だけを見るので、特定の型名が `_event` などで終わる区間
（イベント族のサブツリー）は構造的に鳴らない、(2) オーナー裁定の例（`entities/`・`value_objects/`）を
赤例に、コンテキスト・型の所有サブツリー・射程外の層を緑例に同梱、(3) 導入時点の既存違反は 0 件
（13 のサブディレクトリはすべて所有者の型を持つ）。

**更新 2026-09-25（3）**: `read-model-updater-contract`（RMU の更新入口は共通契約
`ReadModelUpdater` 1 つ — [cqrs-boundaries.md](cqrs-boundaries.md) 規則 3 の同日追記）が加わり
実装済みは **9 本**。RMU の更新器が型ごとに形の違う `catch_up` を持っていたのを共通契約へ
揃えたリファクタリング（オーナー承認）と同じ変更で入れたもので、下表の待ち行列には並んで
いない。着手条件 1〜3 の充足: (1) 規則の文面に留保が無く、名前と実装の対応は構造で判定できる
（正当な例外は理由付き allow で通す）、(2) 実在した 2 形（名乗るのに契約を持たない型・契約の外に
並んだ `catch_up*` の更新入口）と、その裏返し（契約を実装するのに名乗らない型）を赤例として同梱し、
変更前の `main`（05a4ef74）に当てると 16 件（前者 6 件・後者 10 件）を検出することを確かめた、
(3) 既存違反 16 件の是正を同じ変更で着地。

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
| 1 | **構造体リテラルは型ごとに 1 箇所** | [factory-naming.md](factory-naming.md)（基本コンストラクタ） | 反例ほぼ無し。~~是正対象は `WorkflowExecutionState` の 1 型~~ — 是正済み（B13 2026-08-30。メメント型は廃止され型ごと消滅した。現行の再構成は `IntentExecution::replay(snapshot, events)` と、アダプタの DTO から起こす `IntentExecution::new` — 実測 `modules/core/command/domain/src/orchestration/intent_execution.rs:352`）。着手条件 3（既存違反の是正と同じ Bolt で着地）の対象は次に見つかった反例で決める |
| 2 | **`pub(crate)` / `pub(super)` フィールド**（`no-public-fields` の境界拡張） | [field-visibility.md](field-visibility.md) | 例外を認めない裁定済み。~~是正対象は 1 と同じ型~~ — 是正済み（同上、B13 2026-08-30 で型ごと消滅） |
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
