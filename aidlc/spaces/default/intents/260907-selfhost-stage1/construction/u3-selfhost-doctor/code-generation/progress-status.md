# U3 の進捗整理

2026-09-12。工程全体の完了・承認を記録する文書ではない。承認済み `code-generation-plan.md` は凍結成果物なので、進捗はこのファイルへ書く。

## 計画承認

- 指紋 `sha256:5a235659…`、Runtime Session `9d1d85aa-6e19-407b-ba66-8f78afcb9e92`、`PLAN_APPROVAL_RECORDED`、`begin` は `status: generation`。
- 承認前に指示のソース床が B1 の CI 修正前の状態で固定されていたため、`.aidlc-active-directive.json` を利用者が削除して指示を再発行し、床を取り直した（詳細はメモリ `aidlc-code-generation-guard-pitfalls`）。

## 現在地

| ステップ | 状態 | 根拠 |
| --- | --- | --- |
| 1 入力と実行基盤 | 完了 | `tdd-logs/01-runner-check.log`（`diagnostic_record_contract` 7 件成功）。C7 表・corpus 28 ケース・本家ソース（`a277af21` の該当行）を読了 |
| 2 観測 DAO | 完了 | `tdd-logs/02-observation-dao-red.log`（7 失敗）→ `02-observation-dao-green.log`（8 成功）。`core-query-interface-adapter` に `DoctorObservationDaoImpl` |
| 3 判定 use case | 完了 | `tdd-logs/03-report-use-case-red.log`（8 失敗）→ `03-report-use-case-green.log`（8 成功）。`core-query-use-case` に `DoctorReportUseCase` / `DoctorReport` / `DoctorCheck` |
| 4 CLI 入口と描画 | 完了 | `tdd-logs/04-cli-entry-red.log`（`Unknown subcommand: --doctor` で 3 失敗）→ `04-cli-entry-green.log`（4 成功）。`Request::Doctor`、`runtime/doctor.rs`、`Completion::reported` |
| 5 実行記録の副作用 | 完了 | `tdd-logs/05-record-side-effects-red.log`（4 失敗）→ `05-record-side-effects-green.log`（8 成功）。DC1 / DC2 / DC10、状態欠落を復元しない |
| 6 異常注入と本家対応 | 完了 | `tdd-logs/06-corpus-green.log`（corpus 19 観測の採用行バイト一致 + 終了値）、`06-native-injection.log`（独自診断 5 形）。`07-doctor-contract-debug.log` / `07-doctor-contract-release.log`（各 10 件）、`07-regression.log` |
| 7 統合検証・引継ぎ | 未着手（親が実施） | 報告書 [implementation-verification.md](implementation-verification.md)、[source-manifest.json](source-manifest.json) |

## 2026-09-12 担当 `u3_doctor` の着地と裁定

- Step 1〜6 を TDD で着地: `doctor_observation_contract` 8 件、`doctor_report_contract` 8 件、`doctor_contract` 10 件（debug / release）、既存回帰 7 本成功。本家対応行は corpus 19 観測とバイト一致。報告書 [implementation-verification.md](implementation-verification.md)、`tdd-logs/01〜07`。
- 裁定（[doctor-placement-questions.md](doctor-placement-questions.md)）: Q1 D3.c/D3.d の対象はコンパイル済みグラフ全体 = A。**Q2 判定の置き場所 = X「コマンド側集約が処理してイベントを吐き出し、RMU がイベントからリードモデルを作り、クエリ側がその結果を表示」**（承認済み計画の「判定を Query use case に置く」は計画側の誤り。CQRS 規則 2026-09-02 追記と C5 に従う）。Q3 初回はメモリ上のストアで同じ流れ = A。
- 次: 判定をコマンド側集約（ドメイン）へ移す再配置を担当へ委譲。

## 2026-09-12 再配置の着地と親の独立検証

再配置は段 09〜16 で着地した（担当 `u3_doctor` を再開して実施）。

| 段 | 対象 | Red | Green |
| --- | --- | --- | --- |
| 09 | リポジトリ（永続化・再構成・一時ストア） | 7 失敗 / 1 成功 | 8 成功 |
| 10 | コマンド側ユースケース `DiagnoseWorkspaceUseCase` | 6 失敗 | 6 成功 |
| 11 | RMU 投影 `WorkspaceDoctorReadModelUpdater` | 6 失敗 / 2 成功 | 8 成功 |
| 12 | クエリ側 DAO 2 本 + 表示専用ユースケース | IA 3 失敗 / UC 6 失敗 | IA 6 成功 / UC 7 成功 |
| 13 | 合成ルートと外部契約（Refactor — 不変の確認） | — | `doctor_contract` debug 10 / release 10 |
| 14 | 承認済み手順の既存回帰 + 単位限定 | — | 全 green |
| 15 | `cargo test --workspace --no-fail-fast` | — | 3,632 成功 / 2 失敗（下記） |
| 16 | 失敗の単独再実行 | — | 全 green |

段 13 に red が無いのは `doctor_contract` を変更していないためである（コメント 1 行のみ。アサートは 1 文字も変えていない）。判定の期待値はドメイン側 `workspace_doctor_contract` が持ち、クエリ側は行の写しを確かめる形へ組み替えた。テスト件数は減っていない（旧クエリ側判定 8 → ドメイン 10 + リポジトリ 8 + コマンド UC 6 + RMU 8 + クエリ DAO 6 + クエリ UC 7）。

クエリ側から判定 10 ファイル・約 1,880 行を撤去し、`core-query-use-case` の `regex` 依存も未使用化したため外した。その結果 `Cargo.lock` の差分は**ゼロに戻った**。

### 親の独立検証（申告の裏取り）

| 検査 | 結果 |
| --- | --- |
| `cargo fmt --all --check` / `cargo lint` | 成功（所見 0） |
| `cargo test -p aidlc --test doctor_contract` | 10 成功（19.18s） |
| `cargo clippy --workspace --all-targets -- -D warnings` | 成功 |
| `review_guards_contract` / `learnings_contract` / `pipeline_link_contract` | 27 / 23 / 12 いずれも 0 失敗 |

全通しで落ちた 3 件（`review_guards_contract` と `learnings_contract` が `unix_wait_status(9)`・stdout/stderr 空、`pipeline_link_contract` が並列時のみ）は、単独再実行でいずれも成功した。担当の「環境由来」の判断は妥当である。機械は共有で、計測時点の負荷は load average 4.55 / 5.99 / 6.06・ログイン 28 人、メモリは空き 94%・swap 0 だった。

### 性能への影響（不変ではない点）

診断 1 回に ES 往復（一時ストアへの書込 → 再投影 → 読取）が加わり、`doctor_contract` の所要が debug 17.22s → 19.82s（+15%）、release 4.49s → 5.43s（+21%）になった。

### 裁定 Q4 と後続

診断イベントの置き場所は [doctor-placement-questions.md](doctor-placement-questions.md) Q4 = A で確定した。未使用になった `WorkspaceDoctorRepositoryImpl::open(&StorePath)` の撤去を担当へ差し戻し中。

## 2026-09-12 Step 7（親）の実測

裁定 Q4 の帰結（未使用の `open(&StorePath)` 撤去）は担当が着地。`workspace_doctor_repository_contract` は 6 件を `open_ephemeral()` へ移植して 8 件のまま、削除 0（`tdd-logs/17-open-removal.log`）。

| 検査 | 結果 |
| --- | --- |
| `cargo fmt --all --check` / `clippy --workspace --all-targets -D warnings` / `cargo lint` | 成功 |
| `tools/lint` の fmt / clippy / test | 成功 |
| `cargo audit`（本体・`tools/lint`） | 成功 |
| `bun scripts/aidlc-sync.ts --check`、ハーネス bun テスト 7 ファイル | 成功 |
| ゴールデン bun テスト 5 ファイル、`verify-corpus.ts tests/golden/upstream-a277af21` | 成功 |
| `bash scripts/quint-gate.sh` | 成功 |
| `cargo test -p aidlc --release --test doctor_contract` | 10 成功 |
| カバレッジ | **line 98.0666%、絶対床 90.0% PASS**。113 バイナリ・3,666 件成功・0 失敗（6 分 38 秒） |

カバレッジは `scripts/coverage.sh` を書き換えず、同じ設定（`ABSOLUTE_THRESHOLD=90.0`、`PROPTEST_RNG_SEED=20260823`、除外 `main.rs`）のまま `cargo llvm-cov` を直接呼び `--no-fail-fast` を足して計測した。負荷依存の揺れで計測が 2 回とも途中終了したためで、しきい値・シード・除外は不変。最終計測は失敗 0 件で完走している。

揺れの実体（4 例）: `pipeline_link_contract` 2 件（`WouldBlock` / 出力空）、`review_guards_contract` 1 件、`learnings_contract` 1 件。いずれも `unix_wait_status(9)` か SQLite ロック待ち超過で、単独では成功する。doctor の経路とは無関係。

なお最終計測の実行時間は 6 分 38 秒で、全通しの支配項は**テスト実行ではなくコンパイル**である（再配置が依存グラフ最下層の `core-command-domain` を触るため、下流の全 crate と全テスト target が再ビルドされる）。

成果物: [source-manifest.json](source-manifest.json) 175 経路、[traceability.json](traceability.json)（FR5・FR6・NFR1・NFR2・NFR4 = OK、NFR3 = Deferred（B2 の CI が証拠））、[code-summary.md](code-summary.md)。

残り: 独立レビュー（adversarial、最大 2 反復）、Unit 完了。外部の振る舞い・テストは不変。
