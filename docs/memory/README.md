# docs/memory — 設計ルールの正本（人間・全エージェント共有）

オーナー裁定で確定した**リポジトリ横断の設計ルール**を置く。特定エージェントのプライベートメモリには置かない（アクセスできない主体が出るため）。1 ルール 1 ファイル。ルールを追加・改訂したら本 README の一覧も更新する。

各ルールには裁定日・適用例（PR）・機械強制の有無（`cargo lint` ルール / clippy / 型）を記す。仕様（upstream 互換の観測可能契約）は `docs/specs/` が正本であり、ここに置くのは**書き方のルール**である。

| ルール | 一言 | 機械強制 |
| --- | --- | --- |
| [tell-dont-ask.md](tell-dont-ask.md) | getter は存在してよいが濫用禁止 — 判断は状態の所有者へ | `cargo lint`（checkbox-vocabulary / reap-decision-locality） |
| [domain-equality.md](domain-equality.md) | ドメイン同値関係は `Eq`/`PartialEq` で表現 — 名前付き比較メソッド禁止 | レビュー基準（未リント化） |
| [field-visibility.md](field-visibility.md) | フィールドはデフォルト private — 公開はアクセサ経由 | `cargo lint` ルール化予定 |
