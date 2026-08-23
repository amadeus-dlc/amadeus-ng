# pending-revision — U10 nfr-design（ステージゲートの Request Changes で適用する改訂案）

> 回復レビュー（iteration 2、2026-08-23、NOT-READY: Major 3 / Minor 1）の所見を是正する編集案。終端の受領が凍結している
> （review-freeze フック）ため、nfr-design ステージゲートで人間が Request Changes を選んだ直後にこのとおり `security-design.md` /
> `traceability.json` を改訂し、レビュアーを再実行する。本ファイルは produces ではない（作業メモ）。

1. §1 設計方針に (d) を追加: 「未解決のレビュースレッド（ボットのコメントを含む）を残した PR をマージさせない（review-thread gate —
   オーナー指示 2026-08-23、PR #25 の教訓）」。
2. §2 権限: 「ジョブ個別の昇格なし」→「ジョブ個別の昇格は `review-thread-resolution` ジョブのみ（`contents: read` / `checks: write` /
   `issues: read` / `pull-requests: read` / `statuses: write` — 検出結果をコミットステータス『Check unresolved comments』へ反映する最小権限。
   `.github/workflows/review-thread-resolution.yml` も同じ 5 権限）。他ジョブは read のまま」。
3. §2 に「外部再利用ワークフロー（信頼境界）」の箇条を追加: `j5ik2o/ci/.github/workflows/review-thread-resolution.yml`（オーナー所有）を
   SHA 固定 `9cf0e9a8cd74c72de704763025003ed3b7608c65`（`ci_ref` も同 SHA）で呼ぶ。信頼根拠 = オーナー所有 + SHA ピン留め。更新は差分確認の
   PR のみ（`verify-ci-governance.sh` の `ci-review-thread-gate` 検査）。書込権限を持つジョブから呼ぶため、この WF だけ SHA 固定。
4. §2 ジョブ表の前書き「既存の 3 つを維持」→「`check` / `quint` / `coverage` / `CI Success` の 4 つ（2026-08-23 改訂、superseding #9）」。
   表に 2 行追加: `review-thread-resolution`（name: CI Review Thread Gate、`pull_request` のみ、再利用 WF の入力: `wait_for_other_checks: false` /
   `base_branch: main` / `required_context: Check unresolved comments` / 使用制限到達のボット自己報告を無視、再評価は
   `.github/workflows/review-thread-resolution.yml`（レビュー／コメントイベント・15 分ごと・手動）、required: いいえ（直接）— `ci-success` 経由、
   NFR2.6）と `ci-success`（name: CI Success、`needs: [check, quint, coverage, review-thread-resolution]`、`if: always()`、check / quint / coverage は
   success 必須、review gate は `pull_request` で必須・`merge_group` / `workflow_dispatch` で skipped 許容、audit は要求しない、required: はい —
   コンテキスト `CI Success`、NFR2.1）。
5. §3: 「3 コンテキスト」→「4 コンテキスト（+ `CI Success`）」（本文・手順 2 / 3・例示 JSON）。`REQUIRED_CONTEXTS="check,quint,coverage,CI Success"`。
   適用実績 `ruleset/{before,after}.json`（3、2026-08-22）→ `ruleset/2026-08-23-ci-success/{before,after}.json`（4）。
6. §4 PBT: 「受入は 2 回計測の差 0.00pp」→「PBT 由来の揺れ 0.00pp（達成）。FS ロック並行テスト由来の ±1 行（0.0175pp）は U3 退役まで残るため
   全体差は非 0 — `TOLERANCE` 暫定 0.05、退役後 0.01」。
7. §5 表に「CI Review Thread Gate」行を追加（置き場: ci.yml の 2 ジョブ + 再評価 WF + 外部 WF、障害: 未解決スレッドで赤（意図）/ ボット誤検知 /
   外部 WF 不達・変更、影響: 当該 PR のマージ停止（`merge_group` 対象外）、手当: resolve → 再評価で緑化、誤検知は
   `extra_ignored_auto_report_author_patterns`、SHA 固定・不達は手動再実行）。カバレッジ行の「差 0.00pp」→ 6 と同期。共有資源の
   「秘密情報なし（`GITHUB_TOKEN` read）」→「`GITHUB_TOKEN` は既定 read、review-thread-resolution のみ checks / statuses: write」。
8. §6 表: NFR2.1 = 4 コンテキスト + 正常系（PR #25 / #26 完走）、NFR2.4 = シード固定で PBT 揺れ 0.00pp + TOLERANCE 暫定 0.05、
   NFR4.4 = 直下 read + review-thread ジョブのみ個別権限 + 外部 WF SHA 固定、NFR2.6（新規）= review-thread-resolution + ci-success。
9. `traceability.json`: upstream_ids に NFR2.6 を追加、NFR2.1 / NFR2.2 / NFR2.4 / NFR2.5 / NFR4.2 / NFR4.4 / NFR4.5 の target を上記と同期、
   NFR2.6 の coverage 行を追加。
