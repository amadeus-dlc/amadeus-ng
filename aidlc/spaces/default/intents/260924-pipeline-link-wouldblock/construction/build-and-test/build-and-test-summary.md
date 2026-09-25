# Build and Test の要約 — Issue #134（`replace_pipeline` を IMMEDIATE に揃える）

## ビルドの状態と前提

- 検証するのは、コミットの対象になる状態（`main` の先端 `5a2b51d3` に `journal_reader_impl.rs` の変更 1 件だけを載せたもの）。作業用のチェックアウトで検証する（手順は `build-instructions.md`）。
- 前提: Rust toolchain `1.95.0`、ネットワーク（crates.io）。追加の環境変数・ローカルサービスは不要。

## 生成したテストの種類

| 種類 | 置き場 | 備考 |
| --- | --- | --- |
| 単体テスト（再現テスト） | `construction/code-generation/unit-test-instructions.md` | Code Generation が作成。Test Strategy が Minimal なので、この段で追加の手順書は作らない |
| 既存スイート | `cargo test --workspace` | Testing Contract の scope floor「既存スイートの緑」 |
| 統合・性能・セキュリティの手順書 | 作成しない | Test Strategy が Minimal。性能・セキュリティの NFR も無い（`nfr-requirements/` と `nfr-design/` はこの scope では実行されない） |

## ステージ単位のカバレッジの見込み

Unit の分割は無い（ステージ単位の実行）。ワークスペースの行カバレッジ 90% の床（`scripts/coverage.sh`）を割らないこと。変更行は、新しい再現テストと既存の投影テストの両方で通る。

## Target Verification Matrix

確定した表と根拠は `test-results.md` にある。ここには判定だけを写す。

| Target ID | Source | Expected | Actual | Evidence | Owning Stage | Verdict |
| --- | --- | --- | --- | --- | --- | --- |
| TC-SF-1 | Testing Contract `obligations.scope_floor[0]` | 不具合を狙った回帰テストがあり、修正後に通り、修正前に落ちる | 修正後に通過、修正前は即 `WouldBlock` | `test-results.md` | build-and-test | Met |
| TC-SF-2 | Testing Contract `obligations.scope_floor[1]` / FR4.2 | `cargo test --workspace` の失敗 0 | 4058 通過・1 失敗（修正前にも起きる SIGKILL） | `test-results.md` | build-and-test | Not Met |
| TC-SV-1 | Testing Contract `obligations.strategy_volume` | 要件ごとに検証できるテストが 1 本ある | 対応するテストがある | `traceability.json` | build-and-test | Met |
| NFR1 | `requirements.md` NFR1 | busy timeout 5000ms を変えない | 差分なし | `git diff` | build-and-test | Met |
| NFR2 | `requirements.md` NFR2 | 修正 PR の CI が再実行なしで緑 | 未計測 | — | deployment-execution | Unverified |
| NFR3 | `requirements.md` NFR3 | 行カバレッジ 90% 以上 | 計測できず（SIGKILL で 2 回中断） | `test-results.md` | build-and-test | Unverified |
| NFR4 | `requirements.md` NFR4 | 出力契約・監査行の形が変わらない | 比較テストは単独で 5/5 通過 | `test-results.md` | build-and-test | Met |
| NFR5 | `requirements.md` NFR5 | fmt・clippy・lint が通る | 3 つとも通過 | `test-results.md` | build-and-test | Met |

## 準備状況

- build-ready: はい（ビルド・fmt・clippy・lint が通過）
- test-ready: 条件つき（修正の単体テストと関係する契約テストは緑。ワークスペース全体は macOS ローカルの SIGKILL で緑にならない）
- deployment-ready: 未確定（TC-SF-2 が Not Met、NFR2・NFR3 が Unverified。判断を人に仰ぐ）

## 既知の制約・残件

- NFR2 は CI でしか判定できない。この段では補助証拠（契約テスト 3 本の 20 回連続の緑）までにとどまる。
- macOS ローカルで、契約テストの子プロセスが断続的に SIGKILL（`unix_wait_status(9)`）で終わる。修正前でも同じ頻度で起き、落ちるテストは回ごとに入れ替わる。このため、ローカルではワークスペース全体の緑とカバレッジの率を得られなかった。

## 改訂 1 での確かめ直し

- 再現テストの同期を改訂した後、作業用のチェックアウトで確かめ直した。ビルド・fmt・clippy・lint と、単体テストのコマンドはすべて緑だった。ワークスペース全体は 4058 通過・1 失敗で、失敗は修正と関係しない macOS の SIGKILL である。
- 判定は 1 回目と同じ: TC-SF-2 が Not Met、NFR2・NFR3 が Unverified（PR の CI で判定する）。詳細は `test-results.md` の「改訂 1」にある。

## 訂正（2026-09-25）— FR1.2 の検証

FR1.2（並行した 2 本のうち、もう一方は `already completed` と終了コード 1 で拒否される）を `OK` / `Met` としていたのは誤りだった。`concurrent_duplicate_completions_persist_only_one_receipt` は成功した起動の数と監査の件数しか確かめておらず、拒否側の文言と終了コードは検査していなかった（#155 での CodeRabbit の指摘）。

- `code-generation/traceability.json` の FR1.2 を `Deferred` に改めた。
- 拒否側の検査は PR #156（https://github.com/amadeus-dlc/amadeus-ng/pull/156）で足す。これがマージされるまで、FR1.2 は検証済みとして扱わない。TC-SV-1 の `Met` も FR1.2 については同じ扱いとする。
