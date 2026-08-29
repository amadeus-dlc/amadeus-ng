# infrastructure 層 — 言語拡張の置き場（RPC・DB アクセスは置かない）

**裁定日**: 2026-08-29（オーナー）
**スキル正典**: j5ik2o-clean-architecture（`references/layer-responsibilities.md` — Infrastructure は
機構 logging / metrics / configuration / DI / clocks / ID generators / runtime wiring を所有。
「infrastructure を外部 IO 全般の置き場にするのは典型的誤り — persistence adapters は
interface adapters」）
**機械強制**: クレート分離（`Cargo.toml` の依存方向）+ レビュー基準

## 原則

infrastructure 層は**言語拡張**である — 標準ライブラリを延長する汎用機構だけを置く。
ドメインの語彙も、相手方システムの契約も知らない部品である。

- **置くもの**: ファイル I/O プリミティブ（原子的 tmp+rename・追記専用・fs メタデータ）、
  時計、ID 生成、ロギング、設定読取、計測、といった汎用機構
- **置かないもの（オーナー明言）**: **RPC クライアント・DB アクセス**・外部サービス結合。
  これらは相手方システムの契約（プロトコル・スキーマ・語彙）を知る **gateway** であり、
  interface-adapter 層に置く（[gateway-taxonomy.md](gateway-taxonomy.md)）

## 判定基準

「その部品は**相手方システムの契約を知るか**」— 知るなら gateway（interface-adapter）。
知らずに標準ライブラリを汎用に延長するだけなら infrastructure。
ファイルシステムを stdlib 経由で扱う原子的書込は後者、SQLite の表スキーマを知る読取実装は前者。

## 配置と依存方向

- 文脈ごとに置く: **`core-infrastructure`**（現体現: 旧 `infra-io` — `atomic` / `append_only` /
  `fs_meta`）、**`harness-infrastructure`**（harness 文脈の言語拡張。実体が生まれるまで憲章のみ）
- infrastructure は **domain / use-case / interface-adapter を知らない**（依存しない）。
  逆はどの層から依存してもよい

## 禁止パターン

- infrastructure クレートに rusqlite・HTTP/RPC クライアント・外部 API の型が現れる
- infrastructure がドメイン型（イベント・集約・値オブジェクト）を import する
- 「汎用だから」と gateway（Repository 実装・外部システムクライアント）を infrastructure へ置く
