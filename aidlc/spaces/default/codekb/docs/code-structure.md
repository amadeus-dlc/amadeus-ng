# code-structure — パッケージ/モジュール構成とコードパターン

> リバースエンジニアリング成果物（2026-08-22 実施、`c4d8d95` 時点）。一次情報は開発者スキャン結果とコーディング規則の正本 `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/`。

## ワークスペース構成

Cargo workspace（resolver = "3"、edition 2024 一律、全クレート version 0.1.0、`MIT OR Apache-2.0`）。リポジトリはモノレポ（ADR 0005 — docs とコードが main 同居）。

```
Cargo.toml                 # workspace 定義 + workspace lints（約 50 ルール deny）
modules/
  core/
    domain/                # core-domain — ドメイン層（I/O ゼロ・serde 非依存）
    use-case/              # core-use-case — ポート trait のみ
    interface-adapter/     # core-interface-adapter — Gateway 実装 + 機構
  shared/
    audit-events/          # 監査イベント語彙（Published Language）
    directive-schema/      # ディレクティブ種別の閉集合
    message-catalog/       # upstream 逐語文言カタログ
    canon-json/            # 正準 JSON（スタブ 3 行）
  infra-io/                # 低水準 I/O プリミティブ（ポリシーなし）
  app/aidlc/               # composition root バイナリ（スタブ）
  harness/claude/          # ハーネス配線（スタブ 3 行）
tools/lint/                # amadeus-lint — workspace 非メンバーの detached クレート
formal/                    # Quint モデル 3 本（orchestration 2 + workspace 1、計 1,102 行）
scripts/                   # coverage.sh / quint-gate.sh
tests/
  golden/upstream-3c3146cf/    # ゴールデン入力（stage-graph.json 81,850 bytes、バイト変更禁止）
  conformance/fixtures/        # ITF トレース 15 本
docs/specs/ docs/adr/ docs/upstream/   # 日本語正本仕様 + ADR + 凍結 upstream 仕様
.claude/ aidlc/            # stage-0 開発プロセスホスト（プロダクトコードではない）
```

`tools/lint` は**意図的に workspace 非メンバー**（自前の空 `[workspace]` + 独自 `Cargo.lock`）— coverage / test 対象から外すため。ただしこの選択により CI の fmt/clippy/test が届かない副作用がある（設計監査 C27）。

## ドメイン層のモジュール編成

`core-domain` は境界づけられたコンテキスト 3 つを `pub mod` として公開し、その内側は private mod + ファサードで統制する:

- `orchestration/` — 集約 `WorkflowExecution`（engine_loop.qnt の純粋ステップ関数）+ `AutonomyMode` / `JumpDirection` / `PlanAction` / `SkeletonStance` / `Verdict` 等の Domain Primitive
- `workflow_definition/` — 読取モデル集約 `WorkflowDefinition` + `StageGraph` / `ScopeGrid` / `StageNode`（28 フィールド + Builder）ほか Domain Primitive 10 種
- `workspace/` — `LockProtocol`（audit_lock.qnt の純粋ステップ関数）、`reap_eligible` 述語、状態ファイル純関数群、`CloneId` / `LockIdentity` / `ShardName` / `SpaceName` 等の Always Valid newtype

## コードパターン（規約として一貫）

いずれもコーディング規則の正本（`coding-rules/`）に裁定記録があり、実コードに一貫して反映されていることをスキャンで確認済み。

1. **private mod + ファサード `pub use`**（module-visibility.md）: 型ごとのファイル分割は内部事情とし、mod.rs が「キュレーションされた公開 API 宣言」になる。deny 済み `unreachable_pub` が「pub を付けたが再輸出していない」アイテムをビルドエラー化する運用ループ。
2. **Domain Primitive / Always Valid**: 不変条件を持つ型は private ctor + `parse`。定義モジュール外から構築が必要な値レコードのみ `pub fn new`。
3. **フィールドはデフォルト private**（field-visibility.md）: 読取はアクセサ経由（`as_str` / フィールド名そのまま。`get_` 接頭辞なし）。`cargo lint` の `no-public-fields` ルールで機械強制。
4. **Tell, Don't Ask**（tell-dont-ask.md）: ビジネスロジック領域での getter からの判断再実装を禁止。分類語彙は所有型が述語として公開（例: `CheckboxState` の分類述語、`reap_eligible` のドメイン一元化）。`checkbox-vocabulary` / `reap-decision-locality` ルールで機械強制。
5. **ドメイン同値は `Eq`/`PartialEq` の手実装**（domain-equality.md): 名前付き比較メソッドを作らない。同一性に含めないフィールドは doc に根拠明記。
6. **逐語ピン留め doc**: upstream `file:line` @ `3c3146cf` への逐語引用が doc コメントに常設され、仕様・ADR・coding-rules へ相互参照する。`missing_docs = deny`。
7. **ワイヤ構造体はアダプタ層のみ**: serde 依存は `core-interface-adapter` に閉じ、ドメイン型へは parse-don't-validate で写す。
8. **lint 抑制は理由必須**: `// amadeus-lint: allow(<rule>) — 理由` 形式。現存 3 箇所、全件理由付き（ただし理由なし抑制が機械的には成立してしまう穴が C28 として登録済み）。
9. **TODO はトラッキングタグ付きのみ**: 6 件全件がタグ付きで、野良 TODO / FIXME / HACK はゼロ。

## テストコードの配置

- インライン `#[cfg(test)]` — 48 ファイル（PBT は proptest。集約本体 `workflow_execution.rs` に PBT 同居）
- `modules/core/domain/tests/` — ITF 準拠テスト 2 本（Quint トレースの再生 + 状態射影突き合わせ）
- `modules/core/interface-adapter/tests/` — 統合テスト 4 本（ゴールデンパリティ、FS ロック、Repository 実装 775 行、シンボリックリンク防御）
- `tools/lint/src/check.rs` — lint ルールの赤例テスト 31 本（workspace テスト外で独立実行）

## 未実装領域の構造

スタブ 3 クレート（`canon-json`、`modules/app/aidlc` の `const fn main()`、`harness-claude`）はいずれもフェーズ A の計画済み未着手であり、クレートの「席」だけが依存グラフ上に確保されている。`state_file_io.rs` は B-2（`WorkflowExecutionRepository`）の内部部品として先行実装され、消費者不在のため `dead_code` 許可中 — 未完了の設計判断がコードに `allow` として可視化されるパターンである。
