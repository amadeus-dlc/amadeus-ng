# カバレッジ相対ゲートの扱い

2026-09-11。2 回のカバレッジ向上後も head 98.55% で相対ゲート（`head >= base(main) 99.15% - 0.01`）に届かず（[coverage-gap.md](coverage-gap.md)、[progress-status.md](progress-status.md)）。利用者から「90% 以上なら問題なしとしたほうがいいのでは」との発言を受け、規則の扱いを裁定に付した。

## Q1: 相対ゲート

[Question]: 相対ゲートをどう扱いますか？（どの案も `team.md` の規則と `scripts/coverage.sh` / `.github/workflows/ci.yml` の変更を伴う）

- A. 絶対床 90% だけにする（相対ゲートを廃止。`coverage.sh` の `--base` と `ci.yml` の相対実行を外し、規則を改訂）
- B. 相対ゲートの許容差を広げる（例: 1.0 ポイント）
- C. 相対ゲートを維持し 3 回目をやる
- X. Other (please specify)

[Answer]: A. 絶対床 90% だけにする

利用者の原文: 「カバレッジ90％以上なら問題なしとしたほうがいいのでは。今基準はどうなっているの？」→ 基準の説明後に A を選択。
