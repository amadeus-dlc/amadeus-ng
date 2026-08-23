# pending-revision — U10 code-generation（ステージゲートの Request Changes で適用する改訂案）

> 回復レビュー（iteration 2、2026-08-23、NOT-READY: Critical 1 / Major 2）の所見を是正する編集案。終端の受領が凍結している
> （review-freeze フック）ため、code-generation ステージゲートで人間が Request Changes を選んだ直後に適用し、レビュアーを再実行する。
> 実装（ci.yml / review-thread-resolution.yml / scripts/governance / ruleset）は `verify-ci-governance.sh` 20/20 PASS で健全 — 直すのは記録側。

1. `code-summary.md` §1（ファイル一覧）に追記: `.github/workflows/ci.yml`（`review-thread-resolution` ジョブ = `j5ik2o/ci` 再利用 WF を SHA
   `9cf0e9a8cd74c72de704763025003ed3b7608c65` で固定呼出し、ジョブ個別 `permissions`（contents: read / checks: write / issues: read /
   pull-requests: read / statuses: write）、`ci-success` 集約ジョブ）、`.github/workflows/review-thread-resolution.yml`（新規 — レビュー系イベント
   + 15 分毎 cron + 手動で再評価）、`scripts/governance/verify-ci-governance.sh`（検査 20 本 — `ci-review-thread-gate` / `ci-success-aggregate` /
   `ci-review-thread-refresh-workflow` を含む）、`scripts/governance/ruleset-required-checks.sh`（`REQUIRED_CONTEXTS="check,quint,coverage,CI Success"`）、
   `ruleset/2026-08-23-ci-success/{before,after}.json`。
2. `code-summary.md` §2（実装判断）に「review-thread gate（superseding #9、オーナー指示 2026-08-23）」の判断を追記: 4 コンテキスト化の理由
   （`ci-success` 1 コンテキストで PR では 4 条件 / queue では 3 条件）、外部 WF を SHA 固定にした理由、`audit` は required 外のまま。
3. `code-summary.md` §3（テスト / 受入）に `verify-ci-governance.sh` 20/20 PASS（`--with-ruleset` 含む）と、review gate の実地確認
   （未解決スレッドのある PR が `CI Success` 赤 → resolve 後に緑）を追記。§5（計画からの逸脱）に #9〜#11 を追記。
4. `traceability.json`: 全 `target` をファイルパス単体にする（全角括弧の注記を除去 — 注記は code-summary 側へ）。FR9.1 / NFR2.1 / NFR4.5 の
   target を `.github/workflows/ci.yml`（+ 必要なら `scripts/governance/ruleset-required-checks.sh` を別行に分けられないため主ファイル 1 本に）、
   NFR4.4 の target を `.github/workflows/ci.yml`、FR9.x は既存どおり。センサー `aidlc-sensor-traceability.ts --stage code-generation` で
   `invalid_targets` = 0 を確認。
5. `code-summary.md` §1 Step 1 / Step 10 の検査数値（「15 項目」「16/16 PASS」）を実測（`verify-ci-governance.sh` 19/19、`--with-ruleset` で 20/20）に
   更新する（レビュアー最終報告の追加指摘）。
