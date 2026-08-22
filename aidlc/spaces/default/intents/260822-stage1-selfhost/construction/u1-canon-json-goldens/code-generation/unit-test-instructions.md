# unit-test-instructions — U1 canon-json とゴールデン（`u1-canon-json-goldens`）

> Code Generation（Construction 3.5）の単体テスト手順（Unit: U1、kind: library）。出典: `code-generation-plan.md`
> （Testing Contract: methodology tdd / strategy standard / scope classic）、`aidlc/spaces/default/memory/team.md`
> Testing Posture（TDD、カバレッジ 90% 床、`cargo test --workspace` / `cargo-llvm-cov`）、
> `../nfr-requirements/security-requirements.md`（NFR2.1〜2.3）、`../nfr-design/logical-components.md` §4（テスト配置）。
>
> **すべてのコマンドは本 Unit（クレート `canon-json`）に限定する。** Build and Test は Unit ごとにここのコマンドを
> 実行するため、`cargo test --workspace` のような全体コマンドは本ファイルには書かない（品質ゲートとしての全体実行は
> 計画 Step 16 / 19 の範囲）。

## 1. フレームワークと設定

- テストランナー: Rust 標準（`cargo test`）。追加設定ファイル不要（`Cargo.toml` の `[dev-dependencies]` に
  `proptest`（workspace 経由）を置く）。
- ユニットテスト: 各モジュールのインライン `#[cfg(test)] mod tests`（`clippy.toml` によりテスト内の `unwrap` /
  `expect` は許可）。
- 統合テスト（受入）: `modules/shared/canon-json/tests/golden_hash_canonical.rs`（受入表の全行比較）、
  `modules/shared/canon-json/tests/golden_corpus_read.rs`（cli / hooks コーパスの読取と正規化 — 委任 2 で追加）。
  テスト支援（コーパス読取・`normalize()`・行 diff）は `modules/shared/canon-json/tests/support/mod.rs`。
- PBT: `proptest`、各モジュールの `#[cfg(test)]` 内（決定性・往復・冪等性）。

## 2. 実行コマンド（本 Unit 限定）

最初の Red の前に走ることを確認済み（brownfield 実測 2026-08-22: `running 0 tests` / exit 0）:

```bash
cargo test -p canon-json
```

用途別:

```bash
cargo test -p canon-json --lib                          # インラインユニット + PBT のみ
cargo test -p canon-json --test golden_hash_canonical   # hash-canonical 受入表の全行比較（FR7.3 の合格判定）
cargo test -p canon-json --test golden_corpus_read      # cli / hooks コーパスの読取・正規化（委任 2 以降）
cargo test -p canon-json --doc                          # rustdoc 例
```

Red の記録: 失敗するテストを書いたら上記コマンドを実行し、`test result: FAILED. N passed; M failed` の要約行と失敗
テスト名を `code-summary.md` に写す（TDD の証跡、NFR2.1）。

## 3. 期待するテスト量とカバレッジ

- Standard 戦略: コンポーネント（value / profile / writer / canonical / digest / parse / facade+to_value / 比較器）
  ごとに 5〜8 本のユニットテスト、境界（ゴールデン読取）に統合テスト。目安 50〜70 本 + PBT 4 本 + 受入表の行数。
- カバレッジ: ワークスペース床 90%（`scripts/coverage.sh`）。canon-json 単体は 100% 近傍を目標:
  `cargo llvm-cov -p canon-json --summary-only`（`cargo-llvm-cov` 導入済みの環境で）。
- ゴールデン受入表は**全行一致**が合格（1 行でも不一致なら FR7.3 不合格。実装を直し、ゴールデンは直さない）。

## 4. モック・スタブの方針

- 使わない。canon-json は純粋関数群で外部 I/O を持たない。
- ゴールデンがオラクル（フィクスチャ）。ネットワークはテストでは使わない（再採取スクリプトのみが使う）。

## 5. テストデータ

- ゴールデン: `tests/golden/upstream-3c3146cf/hash-canonical/cases.json`（+ `provenance.json`）、
  `cli/<verb>/<case>/…`、`hooks/<hook>/<case>/…`、正規化規則 `normalization.json`。テストからは
  `concat!(env!("CARGO_MANIFEST_DIR"), "/../../../tests/golden/upstream-3c3146cf")` で解決する。
- ゴールデンは**読み取り専用**。更新は upstream ピン更新の intent でのみ（BR2.5）。
- PBT の生成器: NaN / ±Infinity を含まない `JsonValue`（往復性質用）と、含む生成器（非有限 → `null` の性質用）を
  分ける。失敗ケースは proptest の既定どおり `proptest-regressions/` に残す（コミット対象）。
- 一時ファイルは不要（`tempfile` は使わない）。
