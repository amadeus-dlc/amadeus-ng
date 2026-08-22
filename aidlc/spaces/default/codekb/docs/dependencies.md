# dependencies — 外部依存とクレート間依存

> リバースエンジニアリング成果物（2026-08-22 実施、`c4d8d95` 時点）。一次情報は `Cargo.toml` 群と `Cargo.lock`（開発者スキャンで全文確認）。

## クレート間依存（内向き強制構造）

依存は**常に内向き**（ドメイン層へ向かう方向のみ）。逆依存は `Cargo.toml` に依存が存在しないことで **E0432（ビルドエラー）として物理的に成立しない** — これが D4（クリーンアーキテクチャ）と use-case-rules.md（DIP）の機械強制である。

| クレート | 内部依存 | 外部依存 |
| --- | --- | --- |
| core-domain | audit-events, directive-schema, message-catalog | なし |
| core-use-case | core-domain, audit-events, directive-schema | なし |
| core-interface-adapter | core-use-case, core-domain, audit-events, directive-schema, canon-json, message-catalog, infra-io | serde, serde_json, md5 |
| aidlc（app） | core-interface-adapter, infra-io | なし |
| harness-claude | core-interface-adapter | なし |
| infra-io | なし | libc, nix |
| audit-events / directive-schema / message-catalog / canon-json | **依存ゼロ** | なし |
| amadeus-lint（detached） | なし | syn, proc-macro2 |

構造上の要点:

- **shared 4 クレートは依存ゼロ**の純粋部品で、どの層からも利用可（D4 の「純粋部品」群）。
- **infra-io に依存できるのはアダプタ層と composition root のみ**（core-domain / core-use-case の `Cargo.toml` に infra-io が無い）。
- **core-use-case にアダプタ層依存が無い**こと自体が「ユースケースは trait しか知らない」（DIP）の強制。実装への import は書いた瞬間 E0432。
- **ドメイン層は serde 非依存**。ワイヤ形式の都合がドメイン型に浸透しない。

## 外部依存（実測バージョン）

| クレート | バージョン | 消費者 | 役割 |
| --- | --- | --- | --- |
| serde / serde_json | 1.0.229 / 1.0.151 | core-interface-adapter | PL 3 入力のワイヤ構造体 |
| md5 | 0.8.1 | core-interface-adapter | ロック dir 名の upstream 互換導出 |
| nix | 0.30.1 | infra-io | signal / fs の safe wrapper |
| libc | 0.2.189 | infra-io | 低水準定数 |
| proptest | 1.11.0 | dev-dependencies | PBT |
| tempfile | 3.27.0 | dev-dependencies | FS テスト |
| syn / proc-macro2 | 2.0.119 / 1.0.107 | amadeus-lint | 構文解析 |

外部依存の総量は意図的に小さい。async ランタイム・CLI パーサ・ログ基盤は計画済み未導入（`technology-stack.md` 参照）。

## 非 Cargo の依存関係

- **CI → Node 22 + `@informalsystems/quint` 0.32.0**: quint ジョブのみが Node に依存。プロダクトバイナリは Node / bun 非依存（D1）。
- **CI → cargo-llvm-cov**: coverage ジョブ。
- **検証資産の依存連鎖**: `formal/*.qnt`（契約正本）→ `tests/conformance/fixtures/`（ITF トレース）→ `modules/core/domain/tests/`（リプレイテスト）。モデルを変えたらトレース再生成が必要になる暗黙の順序依存がある。
- **ゴールデン依存**: `tests/golden/upstream-3c3146cf/` は upstream 配布物の凍結コピー（バイト変更禁止）で、`golden_parity_test.rs` が消費する。

## 既知の依存課題

1. **コンテキスト間逆依存（C13、R1 DECIDED・未履行）**: `core-domain` 内部で `workflow_definition/scope_grid.rs` が `orchestration::PlanAction` を import。クレート間ではなくコンテキスト（モジュール）間の逆流であり、Cargo では強制できない層内の穴。R1 裁定は `PlanAction` の所有を workflow_definition へ一本化する。
2. **Cargo.lock に syn 2 系 / 3 系が併存**（推定: zerocopy-derive 由来）。実害未確認だが、重複依存としてビルド時間・監査面のノイズ。
3. **detached クレートの独立 Cargo.lock**: `tools/lint` の依存更新はメイン workspace の更新と別管理になり、CI も届いていない（C27）。
4. **ADR 0005 依存表との整合再確認（C23）**: R4（逐語文言の message-catalog 移設）完了後に、ADR 記載の依存表と実 `Cargo.toml` の一致を再確認するタスクが登録済み。
