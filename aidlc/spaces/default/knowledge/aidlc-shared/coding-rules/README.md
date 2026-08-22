# aidlc/spaces/default/knowledge/aidlc-shared/coding-rules — 設計ルールの正本（人間・全エージェント共有）

オーナー裁定で確定した**リポジトリ横断の設計ルール**を置く。特定エージェントのプライベートメモリには置かない（アクセスできない主体が出るため）。1 ルール 1 ファイル。ルールを追加・改訂したら本 README の一覧も更新する。

各ルールには裁定日・適用例（PR）・機械強制の有無（`cargo lint` ルール / clippy / 型）を記す。**設計の前提はまずスキル正典から** — 設計・命名・配置を自分で考える前に、インストール済みの j5ik2o-* 設計スキル（software-design プラグイン約 29 本。`j5ik2o-ddd-repository-design` / `-repository-placement` / `-custom-linter-creator` 等）を列挙し、該当スキルの SKILL.md と references/ を**先に**読む。前提はスキルに書いてある（実例: Reader 造語と `load` メソッドはどちらもスキル未読のまま設計して差し戻された）。**オーナーの指摘（裁定）は可能な限り機械的な強制へ落とし込む** — 優先順は 型（E1）→ 既存 lint（clippy / rustc）→ `cargo lint` カスタムルール。カスタムルールは検出力を証明する赤例テストが必須（Quint ゲートと同じ DoD）。仕様（upstream 互換の観測可能契約）は `docs/specs/` が正本であり、ここに置くのは**書き方のルール**である。

| ルール | 一言 | 機械強制 |
| --- | --- | --- |
| [tell-dont-ask.md](tell-dont-ask.md) | getter は存在してよいが濫用禁止 — 判断は状態の所有者へ | `cargo lint`（checkbox-vocabulary / reap-decision-locality） |
| [domain-equality.md](domain-equality.md) | ドメイン同値関係は `Eq`/`PartialEq` で表現 — 名前付き比較メソッド禁止 | レビュー基準（未リント化） |
| [field-visibility.md](field-visibility.md) | フィールドはデフォルト private — 公開はアクセサ経由 | `cargo lint`（no-public-fields） |
| [module-visibility.md](module-visibility.md) | mod はデフォルト private — 公開はファサードの `pub use` 経由 | `unreachable_pub`（私有 mod 化で実効化）+ `cargo lint` ルール化予定 |
| [gateway-taxonomy.md](gateway-taxonomy.md) | Gateway 責務は Repository と外部システムクライアントの 2 つ — Repository 名は集約名から取る（Store/Reader/Writer 造語と媒体名は禁止）。機構（時計・ID・プロセス生存）は Gateway ではない | レビュー基準（未リント化。候補は同文書） |
| [use-case-rules.md](use-case-rules.md) | DIP（trait のみ依存）・スタティックバインディング既定・ユースケース間呼出禁止 | Cargo クレート分離 + `cargo lint` ルール化予定 |
