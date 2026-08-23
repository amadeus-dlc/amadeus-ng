# Tell, Don't Ask — getter は存在してよいが濫用禁止

**裁定日**: 2026-08-22（オーナー）
**出典**: オーナー自作スキル [j5ik2o-tell-dont-ask](https://github.com/j5ik2o/ai-tools/tree/main/plugins/software-design/skills/j5ik2o-tell-dont-ask) / [j5ik2o-breach-encapsulation-naming](https://github.com/j5ik2o/ai-tools/tree/main/plugins/software-design/skills/j5ik2o-breach-encapsulation-naming)
**適用例**: PR #12（reap 適格判定のドメイン抽出（退役済み — ADR-007 / Bolt B5。以後は履歴としての例）・CheckboxState 分類述語）
**機械強制**: `cargo lint`（`checkbox-vocabulary`。ルール追加時は検出力を証明する赤例テスト必須）

## 適用領域（2026-08-22 オーナー明確化）

本ルールが禁じるのは**ビジネスロジック領域**（ドメインの判断・ユースケースの分岐材料）での getter 濫用である。**I/O 境界での getter 使用は正当**であり対象外 — リポジトリ・Gateway の永続化、JSON 等への変換、Presenter の描画は、状態を外部表現へ写す場所であり getter なしには実装できない。つまり「getter を使うな」ではなく「**ビジネスロジックで getter から判断を組むな**」である。

## ルール

- **違反** = 他オブジェクトから getter で状態を抜き出し、**その所有者が持つべき（または既に持つ）判断**を呼出側で実装すること。典型形: `if b.x() == .. && b.y() > .. { /* A が B の代わりに決める */ }`。
- 判断は状態の所有者へ移す。値の分類語彙（例: `CheckboxState` の in-flight / finished / active）はその型が述語として公開し、呼出側は変種集合を再列挙しない。
- ドメインに判断の単一実装があるとき（例: `CheckboxState` の分類述語 in-flight / finished / active）、呼出側は生データから同じ判断を再実装せず委譲する。境界規約（`>` か `>=` か等）や変種集合を 2 箇所に重複させない。
  - 履歴上の実例だった `lock_protocol::reap_eligible`（Gateway が `stale_ms` 比較を再実装しないよう境界規約を単一実装に閉じ込めた例）は退役済み — ADR-007 / Bolt B5。以後は履歴としての例であり、規範は上の `checkbox-vocabulary` の例で成立する。

## 違反に数えない（明示的例外）

- テストコードのアサーションとテストドライバ（ITF 準拠の状態射影突き合わせを含む）
- Presenter / 描画 / エラーメッセージ組み立てのためのデータ射影
- 境界での写像（serde ワイヤ構造体 → ドメイン型の parse-don't-validate）
- 意図的な読取モデル / Published Language のクエリ（`WorkflowDefinition` の 5 述語、`ScopeGrid` の 3 値照会 = B1 裁定）。ただし読取モデルが述語として公開済みの判断を生アクセサから再実装するのは違反
- 値オブジェクトの述語クエリ（`mode.is_autonomous()` 等）を訊いて**呼出側自身に帰属する判断**を下すもの
- 集約ルートが自分の境界内の子・自フィールドを読むもの
- I/O Gateway がファイルシステム状態を見て I/O を組み立てるもの（ドメインに同じ判断の関数がある場合を除く）

## 不可避な公開の命名

永続化・シリアライズ・テスト・framework glue のためにどうしても内部状態を晒すアクセサは `breach_encapsulation_of_x` と命名し、侵害を可視化する（採否は導入時にオーナー確認。導入すれば「breach アクセサの呼出はテスト/アダプタ層のみ」という lint ルールが型解決なしで書ける）。

## 集約所有の前提集合

ゲート前提（I7）や skipped 受理前提（I13）のような**集約が所有する遷移前提の変種集合**は分類語彙の再実装ではない。lint には `// amadeus-lint: allow(<rule>) — 理由` で明示し、理由に不変条件番号を書く。
