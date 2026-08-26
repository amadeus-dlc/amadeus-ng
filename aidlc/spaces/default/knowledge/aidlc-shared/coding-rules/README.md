# aidlc/spaces/default/knowledge/aidlc-shared/coding-rules — 設計ルールの正本（人間・全エージェント共有）

オーナー裁定で確定した**リポジトリ横断の設計ルール**を置く。特定エージェントのプライベートメモリには置かない（アクセスできない主体が出るため）。1 ルール 1 ファイル。ルールを追加・改訂したら本 README の一覧も更新する。

各ルールには裁定日・適用例（PR）・機械強制の有無（`cargo lint` ルール / clippy / 型）を記す。**設計の前提はまずスキル正典から** — 設計・命名・配置を自分で考える前に、インストール済みの j5ik2o-* 設計スキル（software-design プラグイン約 29 本。`j5ik2o-ddd-repository-design` / `-repository-placement` / `-custom-linter-creator` 等）を列挙し、該当スキルの SKILL.md と references/ を**先に**読む。前提はスキルに書いてある（実例: Reader 造語と `load` メソッドはどちらもスキル未読のまま設計して差し戻された）。**オーナーの指摘（裁定）は可能な限り機械的な強制へ落とし込む** — 優先順は 型（E1）→ 既存 lint（clippy / rustc）→ `cargo lint` カスタムルール。カスタムルールは検出力を証明する赤例テストが必須（Quint ゲートと同じ DoD）。仕様（upstream 互換の観測可能契約）は `docs/specs/` が正本であり、ここに置くのは**書き方のルール**である。


**良い例は [good-examples.md](good-examples.md) に索引がある** — 規則の文面に対して
「この形」と指せる実在ファイルの一覧。スニペットを書き写さずファイルを指すので、コードが
変われば例も追随する。リンク切れは所見として扱う（カタログを直す前に「なぜ動いたか」を確認）。

| ルール | 一言 | 機械強制 |
| --- | --- | --- |
| [tell-dont-ask.md](tell-dont-ask.md) | getter は存在してよいが濫用禁止 — 判断は状態の所有者へ | `cargo lint`（checkbox-vocabulary） |
| [domain-equality.md](domain-equality.md) | ドメイン同値関係は `Eq`/`PartialEq` で表現 — 名前付き比較メソッド禁止 | レビュー基準（未リント化） |
| [field-visibility.md](field-visibility.md) | フィールドはデフォルト private — 公開はアクセサ経由。**`pub` も `pub(crate)` も禁止で例外を認めない**（2026-08-24 改訂。検出境界の拡張は既存違反の是正と同じ Bolt で着地させる） | `cargo lint`（no-public-fields。`pub(crate)` への拡張は次 Bolt） |
| [module-visibility.md](module-visibility.md) | mod はデフォルト private — 公開はファサードの `pub use` 経由。利便性のための再エクスポートはどこでも禁止（所有元が読めなくなる） | `unreachable_pub`（私有 mod 化で実効化）+ `cargo lint` ルール化予定 |
| [gateway-taxonomy.md](gateway-taxonomy.md) | Gateway 責務は Repository と外部システムクライアントの 2 つ — Repository 名は集約名から取る（Store/Reader/Writer 造語と媒体名は禁止）。機構（時計・ID・プロセス生存）は Gateway ではない。ES Repository は `store` / `find_by_id`（ADR-006） | レビュー基準（未リント化。候補は同文書） |
| [use-case-rules.md](use-case-rules.md) | DIP（trait のみ依存）・スタティックバインディング既定・ユースケース間呼出禁止 | Cargo クレート分離 + `cargo lint` ルール化予定 |
| [error-handling.md](error-handling.md) | 失敗はモジュールごとの手実装エラー enum — `Display` は材料のみ、利用者向け文言はアダプタ層（message-catalog）、thiserror / anyhow 不使用 | `missing_errors_doc` / `missing_panics_doc` / `unwrap_used` / `expect_used` deny（workspace lints）+ `cargo lint` ルール候補（thiserror / anyhow 禁止） |
| [interior-mutability.md](interior-mutability.md) | 内部可変性は既定で禁止 — 可変操作はまず `&mut self`。`&self` の裏に `RefCell`/`Cell`/ロックを置く「`&self` への偽装」は禁止。`&self` + 内部可変性には**強い理由**が要る（立証責任は採る側。現在認められている例外はロックを取り合うメソッドのみ、条件付き）。並行してロックを取りたい場合は `SharedLock`/`SharedRwLock` を持つ `*Shared` ラッパーへ閉じる（手書きの `Rc<RefCell<_>>`/`Arc<Mutex<_>>` は禁止） | レビュー基準（`cargo lint` ルール化予定 — 候補は同文書） |
| [command-query-separation.md](command-query-separation.md) | Query は `&self` + 戻り値、Command は `&mut self` + 戻り値なし or `Result<(), E>`。分離不能ならオーナー許可のうえ理由をコメントに書く | レビュー基準（`cargo lint` ルール化予定） |
| [no-backward-compatibility.md](no-backward-compatibility.md) | 後方互換のコードを残さない — `#[deprecated]`・旧名エイリアス・`pub use .. as`・互換口の並立を禁止。改名や署名変更は呼出側ごと一斉に直す（未配布のため互換の対価が無い。upstream 互換は別問題） | レビュー基準（`#[deprecated]` 検出を `cargo lint` ルール化候補） |
| [factory-naming.md](factory-naming.md) | コンストラクタ相当は `fn new(..) -> Self` に統一。それ以外は用途で選ぶ（`of` 集約 / `from`(`From`・`from_<源>`) 変換 / `parse` 文字列 / `open` リソース / `generate` 算出 / `create` エンティティ、ドメイン語があれば優先）。`valueOf`・`getInstance`・`newInstance` は Rust 慣用と衝突するので不採用 | レビュー基準（`cargo lint` ルール化候補） |
| [ubiquitous-language.md](ubiquitous-language.md) | ドメインモデル（`core/domain` の集約・エンティティ・値オブジェクト・ドメインイベント）の型名・フィールド名・メソッド名はユビキタス言語にする。例外は認めるが**doc コメントに理由の記述が必須** | レビュー基準 |
| [cqrs-boundaries.md](cqrs-boundaries.md) | コマンド側とクエリ側は相互に依存しない。**RMU だけが両側に依存できる**（橋）。**コマンド側の最新状態は常に集約から**（リードモデルは常に遅延しているので物理的に読めない）。境界は**クレート分離**で物理強制する（mod 分割では効かない） | クレート分離（`Cargo.toml` の不在）— 違反はビルドで落ちる |
