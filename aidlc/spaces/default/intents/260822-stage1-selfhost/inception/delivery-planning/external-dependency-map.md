# external-dependency-map — 外部依存の一覧

> Delivery Planning（Inception 2.9）成果物。出典: `bolt-plan.md`、`../units-generation/unit-of-work.md`、
> `../units-generation/unit-of-work-dependency.md`、`../units-generation/unit-of-work-story-map.md`、
> `../contract-design/contract-summary.md`、`../domain-design/components.md`、`../requirements-analysis/requirements.md`
> （前提 A1〜A3・制約 C1）、`../practices-discovery/team-practices.md`、確認質問 `delivery-planning-questions.md`
> （Q5 = A: 外部依存は実質なし）。
>
> **Bolt** = 1 つの Unit を構築フェーズに 1 回通す作業の単位（1 Bolt = 1 PR）。

## 1. 結論

本プロジェクトは AI 開発者エージェントとオーナーだけで完結し、外部チーム・外部 API・データ提供の待ちは無い
（Q5 = A）。念のため、Bolt を止め得る外部要素を §2 に列挙する。

## 2. 外部要素の一覧

| 依存 | 所有者 | リードタイム | 止める Bolt | 滑ったときの手 |
|---|---|---|---|---|
| upstream ピン `3c3146cf`（v2.6.40）の dist 資産（ステージ・エージェント・プロトコル・コンパイル済みグラフ） | awslabs/aidlc-workflows（固定済み — 制約 C1） | なし（リポジトリに取り込み済み） | — | ピン更新は別 intent。stage-1 中は更新しない |
| 0b ゴールデン採取に使う bun + upstream ツール | オーナー環境（前提 A3） | なし（導入済み） | B1 | bun が動かない環境では採取をオーナー環境で実施し成果物だけコミット |
| GitHub の branch protection 設定権限 | オーナー（リポジトリ管理者） | 即時 | B6（FR9.1） | 設定が遅れても他 Bolt は進む。B6 のゲートまでに設定 |
| Claude Code のフック機構の契約（stdin JSON・終了コード） | Anthropic（外部仕様、変更不可） | — | B9 | ゴールデン（B1）と実機確認（B9）で吸収。変更があれば逸脱台帳へ |
| オーナーのレビュー・承認（毎 Bolt ゲート） | オーナー | Bolt ごと | 全 Bolt | 直列運用の前提。承認待ちの間は次 Bolt に着手しない |
| `cargo audit` の RustSec advisory DB（ネットワーク） | CI 環境 | 即時 | B6 以降の CI | DB 取得失敗時は CI ジョブの再実行 |

## 3. 外部依存ではないもの

契約上の未解決項目（`../contract-design/contract-summary.md` §4 の 7 件）は外部依存ではなく、各 Unit の
functional-design で閉じる内部の決定事項として `bolt-plan.md` の該当 Bolt に含める。
