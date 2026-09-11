# セッション補助フックの再開時検証

2026-09-09。承認済みTesting Contract `sha256:d904f82d20fc4ba2d0045d5697ecae08fac96371ae5ab4ab6ec4913ae045ab55` に従い、Step 7のセッション部分を継続した。計画本文と承認記録は変更していない。

## 開始時の実装と残差

session-start/end、subagent完了、セッションnavigationと開始文面の実装が既に存在し、`cargo test -p aidlc --test session_hooks_contract` の8件は成功した。古い `step7-session-inventory.md` の「入口なし」は開始時の棚卸しであり、今回の状態ではない。

残っていたPreCompactの `validate-state` 入口を追加した。状態の必須2セクション、警告、復旧breadcrumb、現在の監査シャードがある場合だけの `SESSION_COMPACTED` を本家2.7.1の固定観測と比較する。監査は既存SessionAuditのCommand→SQLite→RMUを通る。稼働事実は既存HookHealthを通り、復旧breadcrumbはgitignore済みの機械ローカルな観測印として書く。

## RedとGreen

| 振る舞い | Redの実測 | Greenの実測 |
| --- | --- | --- |
| 正常PreCompactの監査 | `session-precompact-red.log`: 公開CLIが `Unknown hook: validate-state` と終了1を返した | `session-precompact-green.log`: 正常監査、stdout/stderr、状態無変更が一致 |
| 復旧breadcrumb | `session-breadcrumb-red.log`: `.aidlc-recovery.md` が存在しなかった | `session-breadcrumb-green.log`: 本家の固定文面と一致 |
| 必須セクション欠落 | `session-invalid-red.log`: 本家にあるwarningが欠けた | `session-invalid-green.log`: warning、invalid監査、breadcrumbが一致 |
| 所有する指示の文脈失効 | `session-context-invalidation-red.log`: 同じ所有sessionでもkindがrun-stageのままだった | `session-context-invalidation-green.log`: kind:error、context epoch増加、owner保持、workflow状態無変更を確認 |

コマンドはいずれも `cargo test -p aidlc --test session_hooks_contract <各テスト名>`。ログは同ディレクトリの `test-evidence/` にある。新しいテスト未検出・並行編集によるコンパイルエラーはRedの証拠に数えない。

cold/監査なしの追加検査では、最初に内部共有SQLiteも不在とする過剰な条件を書いた。既存HookHealthは稼働事実を共有SQLiteへ保存する承認済み経路なので、その失敗は実装欠陥やTDDのRedとして扱わない。修正後の検査は、workflow用spaceストア・状態・監査・復旧記録の必要性を区別する。

## 文脈失効の分担

本家は同じsession/project/intent/stateが所有する指示だけを失効し、context epochを増やして共有承認をリセットする。通常Claudeが作るsessionless ownerをPreCompactの入力だけで横取りしない。

今回の調査でこの状態が従来のActiveDirectiveに不足していたため、親から別担当へ集約・保存DTO・RMU・承認失効の拡張を分担した。公開CLIを使う `session_compaction_contract.rs` では、所有一致は公開Repositoryに保存した合成fixtureで検査し、通常Claudeのsessionless、別session、不正入力、変更後のstateで無変更を検査する。実際のユーザー受領は変更しない。

`runtime::plan_approval::invalidate_context` へ接続済み。session_id/sessionIdの文字列を検査し、実行カーソルがある場合に限り、元の状態本文と対象IDを集約側の照合へ渡す。別session・変更済みstate・通常Claudeのsessionlessは無変更を維持する。

## 最終検査

`cargo test -p aidlc --test session_hooks_contract --test session_compaction_contract` は最終の保存DTO整理後にも終了0、計17件成功。全文は `test-evidence/session-final-regression.log`。

- `session_hooks_contract`: 13件成功。既存8件にPreCompactと終了境界を追加した。
- `session_compaction_contract`: 4件成功。同じ所有sessionの失効、通常Claudeのsessionless、欠落/不正/別session、state変更後の無変更を含む。
- `cargo lint`: 終了0。ログは `test-evidence/session-precompact-lint.log`。
- 担当3ファイルの `rustfmt --edition 2024 --check`: 終了0。
- `cargo clippy -p aidlc --all-targets -- -D warnings`: 同時に変更中の `validation_basis.rs` における `serde_json::to_value` 禁止で停止。親へ具体箇所を共有した。この担当範囲の指摘は出ていないが、Clippy全体の成功とは扱わない。

Step 7全体、workspaceカバレッジ、CI、実地スモークの達成をこの記録で代替しない。文脈失効の集約・保存・RMU側の検証は別担当の証跡と合わせて確認する。

## 担当変更

- `modules/app/aidlc/src/runtime.rs`: `validate-state` の許可名と分岐だけ。
- `modules/app/aidlc/src/runtime/session_hooks.rs`: PreCompactの観測と保存・投影への接続。
- `modules/app/aidlc/tests/session_hooks_contract.rs`: 固定本家比較、cold、監査なし、終了セッション境界。
- `modules/app/aidlc/tests/session_compaction_contract.rs`: 保存済み指示の所有・状態とPreCompactの契約。

既存コーパスの期待値は変更していない。固定参照は `tests/golden/selfhost-stage1/session-hooks.json` の `a277af218f0df7f325d3b8be7b6d90fce2c5bd40`。
