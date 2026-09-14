# U4 実装要約 — 配布接続・切替準備

対象は FR5・FR7・FR8（NFR1–NFR4）、契約 C6（配布接続と補助処理の読取り）・C8（接続・ホスト識別と切替準備）。U2 の正本更新処理と U3 の診断を変更せず利用する統合単位である。

## 何を作ったか

配布の入口（`.claude/settings.json` のフック登録と、ステージ本文が呼ぶ動詞）を、この build の Rust プロセスへ接続した。新規機能の追加ではなく**接続・列挙・検証**が主眼である。

1. **必要集合の確定**: 配布資産 18 ファイルの実バイトから、bugfix スコープ 1 周（9 ステージ）が踏む面 × 動詞 **46 件**を列挙し、機械可読の正本 `scripts/aidlc-selfhost/required-surface.json` と読み物 [required-surface.md](required-surface.md) に固定した。検査は `required_surface_contract` が行う。
2. **フック接続**: `scripts/aidlc-selfhost/hook-binding.json` を接続定義の正本とし、live の `.claude/settings.json` を **native 14 本 + 配布 2 本**へ書き換えた。差分は `scripts/aidlc-sync/patches/selfhost-native-hook-bindings.patch` に記録した。
3. **読取り専用の再利用検証**: C6 の `reader_candidates` について、実行前後で状態ファイル・監査シャード・ストア・承認受領が変わらないことを `read_only_reuse_contract` で固定した。
4. **ホスト識別と切替準備**: `scripts/aidlc-selfhost/host-binding.json` と [selfhost-runbook.md](selfhost-runbook.md) に、タグ・コミット・バイナリ実体の対応、切替手順、復帰手順、`target/release/aidlc` を使うスモークの実行準備を置いた。**安定タグは作成していない**（作成は切替工程）。

## 必要集合の列挙結果

| 区分 | 配線済み | 未配線 | 配布 TS のまま | 計 |
| --- | ---: | ---: | ---: | ---: |
| 列挙した全動詞 | 16 | 27 | 3 | 46 |
| うち bugfix 一周で必ず踏む | 11 | **8** | 3 | 22 |

フックは配布登録 16 本のうち native 面にあるのが 14 本、配布 TypeScript のまま残すのが 2 本（`plan-approval-guard` / `run-sensors`）である。

**bugfix 一周が必ず踏むのに未配線の 8 件（U2 への申し送り）**: `aidlc-state lookup`、`aidlc-utility project-description` / `scope-table` / `stage-table` / `codekb-scope-diff` / `codekb-snapshot` / `codekb-path` / `codekb-publish`。C6 の分担（writer = U2 / integrator = U4）に従い U4 では実装していない。未配線の動詞だけを配布 TypeScript へ流す形は、同じ正本（`aidlc-state.md`・監査シャード）を native と TypeScript が二重に更新することになるため採らない。

## 本リポジトリでの自己診断（実測）

接続後に `./target/release/aidlc --doctor` を本リポジトリで実行した結果は **17 passed, 2 failed, 終了 1** である。

- `✓ Native hook bindings`（D2.e）、`✓ aidlc-plan-approval-guard.ts present` / `✓ aidlc-run-sensors.ts present`（D2.a）— 接続は成立している。
- `✗ Native workflow identity — …/.aidlc-execution: missing`
- `✗ Native projection consistency — …/.aidlc-store.sqlite: projection unavailable (execution cursor missing)`

接続前は 30 passed, 3 failed だった。行数が減ったのは、native へ寄せた 14 本に対応する配布 `.ts` の存在検査（D2.d）が対象外になり行を出さなくなったためである（C7「対象外の検査は行を出さない」）。**失敗として消えたのは D2.e の 1 行だけ**で、残る 2 件は接続前から同じ原因で失敗している。

原因は接続ではなく記録側にある。この作業記録を進めているのは配布 TypeScript のエンジンであり、native の実行カーソル（`.aidlc-execution`）を作らない。`HEALTH_CHECKED` も実行カーソルが無いため記録できず、その旨が stderr に出る（成功したふりをしない）。**切替条件 4 は未達**であり、その解消は未配線 8 件と同じ根に掛かっている。

## 変更ファイル

[source-manifest.json](source-manifest.json)（28 経路）。担当の申告と、U4 セッション開始（2026-09-12 18:00 JST）以降の更新時刻による独立の切り分けが一致している。

- **新規（コード・資産）**: `modules/app/aidlc/tests/` の契約テスト 4 本（`harness_binding_contract` / `required_surface_contract` / `read_only_reuse_contract` / `host_binding_contract`）、`scripts/aidlc-selfhost/` の JSON 3 本、`modules/core/command/domain/src/workspace/hook_binding_declaration.rs`、`modules/core/query/use-case/src/orchestration/port/doctor_view/hook_binding_declaration.rs`、`scripts/aidlc-sync/patches/selfhost-native-hook-bindings.patch`。
- **変更（U3 所有を含む）**: D2.e の追従で `doctor_checks/hook_checks.rs`、`hook_wiring.rs`、`workspace/mod.rs`、観測 DAO の `settings.rs`、合成ルートの `observation.rs`、クエリ側の `port/doctor_view/` 周辺、および各層の契約テスト。`.claude/settings.json`、`.github/workflows/ci.yml`、`scripts/aidlc-sync/installed.json`。
- **削除**: なし。配布資産のステージ本文・エージェント・プロトコル・コンパイル済みグラフ、`tests/golden/` の封印済み fixture は変更していない。

U3 所有ファイルへの変更により、**U3 の終端レビュー受領は失効している**。工程ゲート前に U3 の回復レビューを 1 回起票する。

## 主要な実装判断

1. **D2.e は接続定義へ追従する**（利用者裁定 2026-09-12）。`hook-binding.json` の宣言を正として照合し、宣言どおりの混在は失敗にしない。未知 native 名・未接続・宣言との食い違い（`declared native, registered distributed` など）は失敗のまま。U3 が予告していた追従である（`implementation-verification.md` §5-9「U4 が接続定義を確定したら判定基準を追従する」）。
2. **接続定義は作業ツリーから読む観測として実装した**。build へ焼き込むと、定義を持たない配布そのままの corpus fixture が D2.e で落ち、終了コードのバイト一致が壊れる。「定義の無い作業ツリーでは照合しない」が corpus を守る唯一の形だった。
3. **`undeclared hook` を失敗に含めた**。裁定の「宣言と食い違う接続」の一形態と読んだ。
4. **`installed.json` の保持設定 stamp を直接更新した**。`--apply` は管理下 849 ファイルを削除・再配置するため、走行中のセッションでは避けた。値は同期ツールが staged copy から再計算するので、誤っていれば `--check` が落ちる（実測で成功）。
5. **U3 の異常注入を「未知の native 名」へ差し替えた**。旧注入（混在）は新しい契約では失敗しないため。宣言との食い違いは U4 側の `harness_binding_contract` で固定した。

## 承認済み計画からの逸脱

1. **計画の実測表に誤りが 2 件あった**。`aidlc-testing-posture` の `render` / `fingerprint` は「配布 TS のまま」ではなく native に配線済みだった。`aidlc-audit append` の 4 箇所は禁止の記述であって呼出しではなく、必要集合に数えるべきではなかった。いずれも担当が是正した。
2. **`claude-without-bedrock.patch` の不一致という計画の疑いは誤りだった**。同パッチは配布側（`vendor/.../dist/claude/.claude/settings.json`）に当たるもので、現物の `settings.json` と比べる対象ではない。`--check` は成功する。
3. **計画の 3 前提は同時に成立しなかった**。「2 本は配布のまま」「未配線動詞は U4 で実装しない」「`statusLine` 不変」を同時に満たす接続の形が、この build の D2.a / D2.e を通らないことが実測で判明した。`doctor_pass` は C8 の切替前提なので担当は live を変更せず裁定へ回し、利用者が「D2.e を接続定義へ追従」を選んだ。
4. **CI 設定を変更した**（利用者が承認）。`read_only_reuse_contract` が配布 TypeScript を実際に回すため、`.github/workflows/ci.yml` の `check` / `coverage` へ bun 導入段を追加した。既存 `aidlc-distribution` ジョブと同じ固定 Action・同じ版（`oven-sh/setup-bun` ピン留め、Bun 1.3.13）である。

## テストと検査

| 境界 | テスト | 件数 |
| --- | --- | --- |
| フック接続 | `aidlc --test harness_binding_contract` | 9 |
| 必要集合 | `aidlc --test required_surface_contract` | 7 |
| 読取り専用の再利用 | `aidlc --test read_only_reuse_contract` | 6 |
| ホスト識別 | `aidlc --test host_binding_contract` | 7 |

既存回帰: `upstream_271_contract` 77、`claude_hook_contract` 10、`doctor_contract` 10（debug / release）、`next_branches` 44、`cli_golden_test` 10、`diagnostic_record_contract` 7、観測 DAO 9、RMU 8、command IA 8、`core-command-use-case` 176 — すべて成功。corpus 比較（`upstream_doctor_observations_agree_on_adopted_rows_and_exit_codes`）は壊れていない。

工程末の検査（親が実測）:

| 検査 | 結果 |
| --- | --- |
| `cargo fmt --all --check` / `clippy --workspace --all-targets -- -D warnings` / `cargo lint` | 成功 |
| `tools/lint` の fmt / clippy / test | 成功 |
| `cargo audit`（本体・`tools/lint`） | 成功 |
| `bun scripts/aidlc-sync.ts --check` | 成功（コピー 0・削除 0・保持設定の確認 0） |
| ハーネス bun テスト 7 ファイル、ゴールデン bun テスト 5 ファイル、`verify-corpus.ts` | 成功 |
| `bash scripts/quint-gate.sh` | 成功 |
| `cargo test -p aidlc --release --test doctor_contract` | 成功 |
| カバレッジ（絶対床 90.0%、seed `20260823`、除外 `main.rs`） | **line 98.0641%、床 PASS**。119 テストバイナリ・**3,805 件成功・0 失敗** |

カバレッジは `scripts/coverage.sh` を書き換えず、同スクリプトと同じ設定（`ABSOLUTE_THRESHOLD=90.0`、`PROPTEST_RNG_SEED=20260823`、`--ignore-filename-regex '(^|/)modules/app/aidlc/src/main\.rs$'`）のまま `cargo llvm-cov` を直接呼び、共有機の負荷依存の揺れで計測が途中終了するのを避けるため `--no-fail-fast` だけ足して計測した。しきい値・シード・除外は変えていない。この計測実行は失敗 0 件で完走している。

## 限界と申し送り

- **切替条件 1 と 4 は未達**。未配線 8 件が埋まるまで、bugfix 一周を native エンジンだけで通すことも、本リポジトリの doctor を green にすることもできない。扱い（U2 へ差し戻すか範囲を見直すか）は工程ゲートの裁定に掛ける。
- **準備完了は達成ではない**（C8 `not_proof_of_completion`）。ここでの green はすべて準備の確認であり、実地スモーク（FR7）・切替（FR8）の達成証拠ではない。`traceability.json` で FR7・FR8・NFR3 を `Deferred` にしているのはこのためである。
- **復帰手順**: `git checkout -- .claude/settings.json` → Claude Code を**完全に再起動**（フック設定はセッション開始時に読まれるため、書き換えも戻しも再起動まで効かない）→ `./target/release/aidlc --doctor` で `Native hook bindings` が `binding mismatch (declared native, registered distributed: fold-usage)` へ戻ることを確認。同期側も戻すならパッチを削除し `installed.json` の保持設定 stamp を戻して `--check` を確認する。詳細は [selfhost-runbook.md](selfhost-runbook.md)。
- **live 接続の前提**: `.claude/settings.json` の 14 本が `"$CLAUDE_PROJECT_DIR/target/release/aidlc" hook <name>` を指すため、このバイナリが消えるか古くなるとフックが動かない。次のセッション以降に効く。
- **ゴールデンの採用版は未裁定**のままである。切替条件 2 の判定前に裁定を受ける。
- **D2.e の追従で U3 の所有ファイルに触れた**ため、U3 の終端レビュー受領が失効している。工程ゲート前に U3 の回復レビューを 1 回起票する。

## Sources

- [要求書](../../../inception/requirements-analysis/requirements.md) FR5・FR7・FR8、NFR1–NFR4
- [契約書](../../../inception/contract-design/contract-summary.md) C6・C8、「所有・変更・検証の規則」「未解決事項と実装時の確認」
- [単位定義](../../../inception/units-generation/unit-of-work.md) U4
- [code-generation-plan.md](code-generation-plan.md)、[unit-test-instructions.md](unit-test-instructions.md)、[required-surface.md](required-surface.md)、[selfhost-runbook.md](selfhost-runbook.md)
- `tdd-logs/01`〜`33`、[source-manifest.json](source-manifest.json)、[traceability.json](traceability.json)
- [U2 code-summary](../../u2-workflow-authority/code-generation/code-summary.md)、[U3 code-summary](../../u3-selfhost-doctor/code-generation/code-summary.md)、U3 の `implementation-verification.md` §5-9
- 実測: `.claude/settings.json`、`modules/app/aidlc/src/runtime.rs`（`NATIVE_HOOKS`）、`scripts/aidlc-selfhost/`、`scripts/aidlc-sync/`、`.claude/aidlc-common/` 配下のステージ・プロトコル
