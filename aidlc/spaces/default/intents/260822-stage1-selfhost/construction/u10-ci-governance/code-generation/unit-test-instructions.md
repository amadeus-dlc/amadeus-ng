# unit-test-instructions — U10 CI・品質管理

> 対象: u10-ci-governance（packaging）。現行 `code-generation-plan.md` とTesting Contract、`../nfr-requirements/security-requirements.md`
> のNFR2.1〜2.5 / NFR4.1〜4.5、`../nfr-design/security-design.md` §7に従う。以下はすべて本Unitに限定する。
> 2026-08-22の旧手順は `unit-test-instructions-history-2026-08-22.md` に全文保存した。

## 1. ランナーと設定

packaging Unitのため「単体テスト」は、設定の事実を機械検査するbashスクリプト `scripts/governance/verify-ci-governance.sh` と、本UnitがCIへ
組み込んだ `tools/lint` の既存自己テストである。追加の設定ファイル・ランナー・モックは導入しない。`jq` を使い、`--with-ruleset` 指定時のみ
`gh`（読取のGETだけ）でGitHubへアクセスする。Rustの版は `rust-toolchain.toml`、依存は各 `Cargo.lock` を正本とする。

## 2. Unit限定コマンド

ワークスペースルートで実行する。`bash -n` は最初のファイルしか解析しないため、ファイルごとに個別に実行する。

```sh
bash -n scripts/coverage.sh
bash -n scripts/governance/verify-ci-governance.sh
bash -n scripts/governance/ruleset-required-checks.sh
bash -n scripts/governance/toolchain-inputs.sh
bash scripts/governance/verify-ci-governance.sh                 # 設定の機械検査（ネットワーク不要）
bash scripts/governance/verify-ci-governance.sh --with-ruleset  # 上記 + gh api でruleset「main」の必須コンテキスト（読取のみ、ネットワーク要）
bash scripts/governance/toolchain-inputs.sh                     # rust-toolchain.toml から channel / components を導出
cargo test --manifest-path tools/lint/Cargo.toml                # tools/lint 自己テスト（CI組込み対象）
```

計画準備時（2026-09-06）の `verify-ci-governance.sh --with-ruleset` は20項目成功・失敗0であった（`../revision-baseline-20260906.md`）。
実行担当は上記を再実行し、件数・結果・完了日時を `code-summary.md` へ残す。ワークスペース全体の `cargo test --workspace` はCIの
品質ゲートであり、本ファイルのUnit限定コマンドではない。

## 3. 合格基準と検証範囲

- `bash -n` 4本がすべて終了コード0。
- `verify-ci-governance.sh` が既定で19項目、`--with-ruleset` で20項目すべて成功（期待値はスクリプト内の定数: channel `1.95.0`、
  components `rustfmt clippy llvm-tools`、profile `minimal`、`TOLERANCE=0.01`、除外式 `(^|/)modules/app/aidlc/src/main\.rs$`、
  シード `20260823`、必須コンテキスト集合 `CI Success` / `check` / `coverage` / `quint`）。期待値を書き換えて成功させない。
- `toolchain-inputs.sh` の出力が `channel=1.95.0`、`components=rustfmt,clippy,llvm-tools`。
- `tools/lint` 自己テストが成功し、対象が0件に減っていない。件数は実際の出力から記録し、過去の31本に固定しない。

受入（Unit限定コマンドの外側、計画Step 3）は次を実測して記録する。設定の存在と実働の成功を区別する。

- `bash scripts/coverage.sh` を同一リビジョン・同一ツールチェーン・同一シードで2回実行し、生のhead値（%）と差を記録する。絶対ゲート90%が
  2回とも成功すること。差0.00ポイントは受入目標であり、未達なら未達のまま原因を記録し、`TOLERANCE` や除外を変えない。
- `cargo audit`（workspace）と `cargo audit --file tools/lint/Cargo.lock` の結果、走査crate数、advisory DB取得可否。未導入・取得失敗は
  成功と書かない。
- `unsafe` を含む一時変更で `cargo check` が拒否されること（workspaceメンバー1クレートと `tools/lint`）。確認後に必ず戻す。
- `rustc -V` が1.95.0。

Unit限定コマンドと上記実測の成功は、全CI実行、マージキューの成功・失敗両経路の実働、レビュー再評価の反映、外部再利用ワークフロー内部の
検証の代替ではない。全体検証をUnitごとに繰り返すコマンドはここへ置かない。

## 4. データとテスト支援

検査対象は実ファイルと実ruleset（読取のみ）。`ruleset-required-checks.sh` は今回実行しない（`--dry-run` を含め、設定変更の意図がないため）。
過去の前後JSONは `ruleset/` 配下と `../ruleset-observed-20260906.json` を読取専用で参照する。認証トークン・認証ヘッダーを記録へ混ぜない。

## 5. 失敗時

失敗した検査名・コマンド・出力を記録する。設定と要件・設計の不一致であれば、`verify-ci-governance.sh` へ検出項目を先に追加する（Red）変更案を
親セッションへ返し、計画を更新してから設定を変える。今回の記録是正のために設定を壊して人工的なRedを作らない。
