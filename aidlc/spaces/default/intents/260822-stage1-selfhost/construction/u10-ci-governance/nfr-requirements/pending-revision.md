# pending-revision — U10 nfr-requirements（ステージゲートの Request Changes で適用する改訂案）

> **2026-09-06 更新**: 以下は2026-08-23時点の改訂案を履歴として保存したもの。現在の適用手順には使わない。
> 最新の改訂基準は [CI・品質管理の引継ぎ](../revision-baseline-20260906.md) を参照する。
> 特にカバレッジ許容差を0.05へ戻す記述、配布物検証を含まないCI集約条件、再現性を再計測なしに達成済みとする記述は適用しない。

> 回復レビュー（iteration 2、2026-08-23、READY: Major 2 / Minor 3）の所見を是正する編集案。終端の受領が凍結している（review-freeze フック）
> ため、nfr-requirements ステージゲートで人間が Request Changes を選んだ直後にこのとおり `security-requirements.md` / `tech-stack-decisions.md` /
> `traceability.json` を改訂し、レビュアーを再実行する。本ファイルは produces ではない（作業メモ）。

1. §1 信頼境界に (d) を追加: 外部再利用ワークフロー `j5ik2o/ci`（オーナー所有、SHA 固定）。「実地の現状」段落の末尾に 2026-08-23 現在の追記
   （required checks 4 コンテキスト、review-thread gate 稼働、`review-thread-resolution` ジョブの個別権限）。
2. NFR2.1: 「`required_status_checks`（strict、3 コンテキスト）」→「strict、`check` / `quint` / `coverage` / `CI Success` の 4 コンテキスト」。
   合格基準に正常系を追加: 「全緑の PR が merge queue を経て squash-merge される（実績: PR #25 2026-08-22T23:44Z、PR #26）」。
3. NFR2.4: 「差が 0.00pp」「`TOLERANCE=0.01`」→「PBT 由来の揺れは 0.00pp（`PROPTEST_RNG_SEED` 固定で達成）。FS ロック並行テスト由来の残差
   0.0175pp により全体差は非 0 — `TOLERANCE` 暫定 0.05、U3 ロック退役（ADR-007）後に 0.01 へ引き締める」。
4. NFR2.6（新規）: 「未解決のレビュースレッド（ボット含む）を残した PR をマージさせない — `review-thread-resolution` ジョブ（外部再利用 WF、
   SHA 固定）+ `ci-success` 集約 + 再評価 WF。`merge_group` では skipped 許容」。合格 = 未解決スレッドのある PR で `CI Success` が赤、resolve 後に
   緑化（実地 1 回）。出典: オーナー指示 2026-08-23、superseding #9。
5. NFR4.2 合格基準の分離: 機械検証「ローカルと CI の `rustc --version` が同一」／運用規範「toolchain 更新は PR でのみ」を要求文側へ移す。
6. NFR4.4: 「ジョブ個別の昇格なし」→「`review-thread-resolution` ジョブのみ `checks: write` / `statuses: write` / `issues: read` /
   `pull-requests: read` を個別付与（未解決スレッド検出の最小権限）。他ジョブは read」。合格基準「3 ジョブ + audit が read 権限で成功」→
   「check / quint / coverage / audit / ci-success は read、review-thread-resolution は上記の個別権限で成功」。
7. §3 STRIDE: Elevation of Privilege 行に review-thread ジョブの個別権限と外部 WF の SHA 固定を追記。Denial of Service 行に
   「Dependabot（github-actions / cargo）も SHA ピン留めと同様に本 intent では見送り、後続 intent で検討」を追記（Minor 5）。
8. `tech-stack-decisions.md` §1: 「required_status_checks（check / quint / coverage …）」→ 4 コンテキスト。行を追加: 「レビュースレッドゲート —
   `review-thread-resolution`（j5ik2o/ci 再利用 WF、SHA 固定）+ `ci-success` 集約 + 再評価 WF。代替: GitHub ネイティブの『会話の解決を必須』
   ルール（ボットスレッドの扱い・再評価タイミングをオーナーの WF に合わせるため不採用 — amadeus-dlc/amadeus に倣う）」。
9. `traceability.json`: NFR2 の target に NFR2.6 を追加。
