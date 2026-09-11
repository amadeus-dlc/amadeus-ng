# 診断実施記録のU2実装

2026-09-09。承認済みC7とTesting Contract `sha256:d904f82d20fc4ba2d0045d5697ecae08fac96371ae5ab4ab6ec4913ae045ab55` に従い、U3が使う診断事実の更新APIを実装した。診断項目・doctor表示・修復機能・CLI動詞は追加していない。

## APIと責任

既存の `IntentExecution` が、その作業に関する診断を `HealthChecked` という単一イベントとして記録する。独立集約は新設していない。`CommandFailed`やセッション監査を診断の代用にせず、診断実施の名前と内容を保持した。

公開APIは以下である。

```rust
use core_command_domain::orchestration::{HealthCheckResult, IntentExecutionId};
use core_command_use_case::orchestration::{HealthCheckError, RecordHealthCheckUseCase};

// repository は既存作業の IntentExecutionRepository。
// execution_id は診断対象の作業、at は診断実施の観測時刻。
let mut command = RecordHealthCheckUseCase::new(repository);
let saved: Result<(), HealthCheckError> = command
    .execute(&execution_id, HealthCheckResult::new(passed, failed), at)
    .await;
```

成功戻り値はunit。保存する材料は非負のpassed/failed件数であり、検査内容や表示文面を更新UseCaseへ持ち込まない。集約の更新は進行へ影響しないイベントとして扱い、診断のためにゲートを開閉したり、parkを解除したりしない。保存失敗と、診断結果としてfailedが1以上であることは別である。

既存の `IntentExecutionRepositoryImpl` がspaceのSQLiteへ保存する。書く側とRMU側にそれぞれ専用 `HealthCheckedDto` を置き、通常RMUの `projection.rs` がC7の監査へ投影する。

```text
## Health Check
**Timestamp**: <実施時刻>
**Event**: HEALTH_CHECKED
**Request**: /aidlc --doctor
**Details**: <passed> passed, <failed> failed
```

実際の検査は、先頭改行、空行、終端`---`とLFを含む追記バイト全体を照合している。期待文面の根拠は、本家固定 `a277af218f0df7f325d3b8be7b6d90fce2c5bd40` の `dist/claude/.claude/tools/aidlc-utility.ts:4669–4674,4737–4746` と、承認済み `inception/contract-design/contract-summary.md` C7である。

## TDD

| 振る舞い | Red | Green |
| --- | --- | --- |
| 既存実行への診断1件の保存 | `diagnostic-record-red.log`: SQLiteのジャーナルは期待4行に対し3行のままだった | `diagnostic-record-green.log`: 公開UseCase成功がunitで、1行の事実を追記 |
| C7監査の投影 | `diagnostic-projection-red.log`: 追記が空で、Health Checkの全文が欠落 | `diagnostic-projection-green.log`: 監査の全文が一致し、workflow状態本文は不変 |

最初の失敗を実行できるよう、件数値とUseCaseの無処理の入口だけを先に用意した。未宣言APIやコンパイル失敗をRedとして数えていない。

各単位コマンドは `cargo test -p aidlc --test diagnostic_record_contract <テスト名>`。その後の `cargo test -p aidlc --test diagnostic_record_contract` は7件成功・終了0。全文は `test-evidence/diagnostic-record-regression.log` に保存した。

## 検証範囲

1. 既存SQLiteの実行へ診断1件を追記する。
2. Request/Detailsを含むC7監査の逐語と、状態本文の無変更を確認する。
3. 成功のみ・失敗ありの別診断が、それぞれ別イベントIDと実際の件数を保持する。
4. SQLiteのjournal INSERTを実際に失敗させ、UseCaseがRepositoryエラーを返し、成功事実や監査を作らない。
5. 存在しない実行IDを指定しても新しい実行を作らず、NotFoundを返す。
6. ファイル公開後のcheckpoint失敗を注入し、pending公開計画と未前進checkpointを保持したまま、再投影で監査を重複させず完了する。
7. park済みの作業を診断しても、状態を変えずparkを維持する。

6の最初のテストには「checkpoint失敗なら監査ファイルも未変更」という余分な条件があった。既存 `publication_recovery_contract.rs:611` の契約は、ファイル公開後の確定失敗をpending計画から回復するものである。製品を変更せず、公開済み監査・checkpoint・pending計画と重複しない再開を検査する正しい条件へ修正した。このテスト条件の誤りは製品のRedに数えていない。

`cargo lint`は成功。新規専用ファイルはrustfmtで整形した。全target Clippyと既存event全変種の検査は並行レビュー実装の変更で停止したため、全体成功とは扱わず親へ共有した。HealthCheckedの追加に対応するenum fixtureと列挙件数27への更新は実施済みである。

## U3への引継ぎ

- doctorの起動時点で監査の存在を観測し、**監査なしならRepositoryとUseCaseを構築・呼出ししない**。これはU3の実際の呼出経路で検証する。テスト内だけの仮のif文を置いてcold成功の証明にしない、という親の確認に従った。
- 監査ありなら、選択した既存実行へ診断件数を保存し、その後通常RMUの公開を完了する。成功する診断結果を更新UseCaseから取得し直す必要はない。
- 保存・RMU公開の失敗を終了0へ丸めない。既に表示したdoctor stdoutは残り得る。診断結果のfailed件数が0でも、記録失敗はC7のエラー経路へ渡す。
- 保存済みだが公開が未完了なら、同じ診断を再記録して件数を水増しするのではなく、既存RMUのpending公開を回復する。今回のUseCaseは暗黙の再実行・修復をしない。

cold非作成の実配線、doctor項目と出力、実地Claudeスモーク、workspaceカバレッジ/CIは今回の成功範囲に含めない。

## 変更ファイル

- domain: `health_check_result.rs`、`intent_execution_event/health_checked.rs`、既存 `intent_execution.rs` / `intent_execution_event.rs` / `mod.rs` の追加。
- use-case: `record_health_check_use_case.rs`、`health_check_error.rs`、`mod.rs`。
- command interface-adapter / RMUそれぞれ: `dto/health_checked_dto.rs`、`dto/intent_execution_event_dto.rs`、`dto/mod.rs`。
- RMU: `workspace/projection.rs` のHealthChecked専用arm。
- 公開API結合検査: `modules/app/aidlc/tests/diagnostic_record_contract.rs`。

## 並行レビュー更新後の再確認

レビューAPIのコンパイル復旧後、`cargo test -p core-command-domain --lib orchestration::intent_execution_event::tests::`は7件成功し、27変種の列挙・ID・集約IDを確認した。`cargo test -p aidlc --test diagnostic_record_contract`も再度7件成功。全文は`diagnostic-event-regression.log`と`diagnostic-record-final.log`。診断用の製品実装はこの再確認で変更していない。

同時点のClippy再実行は、並行レビュー担当の両側DTO（review_binding_dto.rs / review_completion_dto.rs）の`needless_borrows_for_generic_args`で停止した。親へ具体箇所を共有済みであり、診断API7件の成功と全体Clippy未成功を分けて扱う。
