# フック 3 スライスの着地確認で見つかった差の裁定

2026-09-10。`u2_reviewer_scope_closeout`（[reviewer-scope-verification.md](reviewer-scope-verification.md)）、`u2_stage_rules_closeout`（[stage-rules-verification.md](stage-rules-verification.md)）、`u2_fold_usage`（[fold-usage-verification.md](fold-usage-verification.md)）が本家固定コミット `a277af21` のフックを実走行させて本 build と突き合わせた結果のうち、契約 C1 / C3 だけでは是正の向きが決まらないもの。契約で決まる差（監査行の `<project-dir>` 伏せ字、状態欄読取りのタブ、カーソル無しの唯一記録への後退、規則ファイルの BOM、Stop 時の台帳 flush-all）は `u2_hook_parity` が是正中で、ここには含めない。

## Q1: レビュアー差し向け記録（`.aidlc-reviewer-dispatch.json`）の文法

[Question]: 差し向け記録の `stage` / `unit` が本 build の型（`StageSlug` / `UnitName`）に合わない値のとき、本家どおり逐語で受けて強制しますか？

本家 `aidlc-reviewer-scope.ts` は `stage: "Functional Design"`（slug でない綴り）や `unit: "a/b"` をそのまま受けて越境を強制する（実走行 c24 / c25 で exit 2）。本 build は型で拒否して drop「malformed」を残し、**強制せずに通す**（fail-open）。前担当は `stage-protocol-reviewer.md:155` を根拠に意図した差だが、保護が弱まる側である。

- A. 本家に合わせ、`stage` / `unit` を逐語で受けて強制する（推奨。`ReviewerDispatch` と `ReviewerScopeBlock`・`session_audit_record.rs` の必須項目へ波及）
- B. 型で拒否する現状を保ち、fail-open ではなく fail-closed（拒否）へ倒す
- C. 現状（型で拒否・fail-open）を保ち、観測差として記録する
- X. Other (please specify)

[Answer]: A. 本家に合わせ逐語で強制 (推奨)

## Q2: `deliver-stage-rules` の `tool_name` が `null` / 非文字列のとき

[Question]: 本家は `tool_name` が `null` / 非文字列の封筒で未処理例外により exit 1 になる。本 build は素通し（exit 0、出力なし）。本家に合わせますか？

本家の exit 1 は明示の拒否ではなく `TypeError` の握り漏れ（`aidlc-deliver-stage-rules.ts` の実走行 64・72）。Claude Code の PreToolUse でフックが exit 1 を返すと stderr が利用者に見えるだけで処理は続く。本 build のテストは exit 0 を固定している。

- A. 本 build の exit 0（素通し）を保ち、観測差として記録する（推奨。例外の握り漏れは契約ではない）
- B. 本家と同じ exit 1 にする（明示の拒否として実装し、stderr の文言は本家の例外文を写す）
- X. Other (please specify)

[Answer]: A. exit 0 を保ち観測差として記録 (推奨)

## Q3: stderr の丸括弧内のエラー原因文言（Node と Rust の差）

[Question]: 規則ファイルが読めないときの stderr `Cannot load required stage rule "<path>" (<原因>). …` の丸括弧内が、本家は Node の文言（例: `ENOENT: no such file or directory, open '<abs>'`）、本 build は Rust の文言（例: `No such file or directory (os error 2)`）で違う。合わせますか？

固定部分（前後の文）はバイト一致。実走行 89 件中 2 件と、512KiB 境界 3 件がこの差だけで不一致。

- A. Rust の文言を保ち、既知の差として記録する（推奨。OS エラーの表現は本家でもプラットフォーム依存）
- B. Node の文言（`ENOENT: …, open '<abs>'` 形）へ写す
- X. Other (please specify)

[Answer]: A. Rust の文言を保ち既知の差として記録 (推奨)

## Q4: 完了監査の利用量欄（`STAGE_COMPLETED` / `WORKFLOW_COMPLETED` の Tokens / Cost USD）

[Question]: 利用量追跡が有効なとき、本家は工程・作業の完了監査へトークン数・キャッシュ数・Cost USD・モデル/agent 別内訳を足す（`aidlc-state.ts` の `stageUsageAuditFields` / `workflowUsageAuditFields`）。本 build の RMU は描かない。U2 で実装しますか？

U1 の採取 75 観測はすべて `AIDLC_DISABLE_USAGE_TRACKING=1`、`fold-usage.json` は監査を含まず、完了ブロック 207 件のどれにも欄が無い。実装するには固定コミットで有効時の `approve` / `complete-workflow` を追加採取する必要がある。欄は利用量があるときだけ付く任意項目で、無くても既存の読み手は壊れない。

- A. U2 では実装せず、切替条件 2（状態・監査の upstream 互換）の判定材料として記録する（推奨。採取と実装は別 intent）
- B. 固定コミットで有効時の完了を追加採取し、U2 で RMU へ実装する
- X. Other (please specify)

[Answer]: A. U2 では実装せず記録 (推奨)

（4 件とも 2026-09-10 の構造化質問で利用者が明示選択。原文はいずれも推奨案の選択肢ラベル）
