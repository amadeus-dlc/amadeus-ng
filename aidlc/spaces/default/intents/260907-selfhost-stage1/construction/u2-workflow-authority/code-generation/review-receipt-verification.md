# レビュー受領の内容結合：最初の実装区切り

## この区切りの範囲

承認済みU2計画Step 5とTesting Contract `sha256:d904f82d20fc4ba2d0045d5697ecae08fac96371ae5ab4ab6ec4913ae045ab55` に従い、通常stage-levelの要求分析・コード生成について、要求本文の保存と判定時の本文・Review追記・ソース照合を接続した。

本記録は**検証可能な最初の差分**である。Step 5全体、レビュー契約の完全互換、U2完了を意味しない。親からの指示に従い、retry回数制限・完成後の回復・summary受領・ゲート前鮮度へ範囲を広げていない。現在の制限は末尾に列挙する。

## 保存と照合

- 入力境界 `runtime/review_documents.rs` が宣言成果物を読む。通常ファイル性、symlink/hardlink、O_NOFOLLOW/O_NONBLOCK、open前後と全ファイル読取後のmetadata照合を行う。不安定な観測は成功する空bindingへ変換せず、domainで拒否する。
- `ReviewDocuments` が論理パスと原文を保持し、request fingerprintと完成後fingerprintを別々に計算する。`ReviewBinding` は追記先・byte offset・既存節digest/length・challenge・sourceを保持する。
- `IntentExecution::request_review` は通常のレビュアー・予算・iteration検査の後で観測を結合する。`record_review_verdict` はpendingに保存されたbindingで本文・追記・sourceを検証し、単一ReviewCompletedイベントを返す。
- ReviewRequested/Completedの両保存DTOとRMU専用DTOに結合材料を持たせた。ReviewAttemptの履歴はsnapshotへ保存する。未結合の要求や、成果物が欠落した要求を、製品の成功経路へ流す旧口は残していない。
- RecordReviewUseCaseは引き続き成功unitである。RMUがRequest Fingerprint、Artifact Fingerprint、Review Appendix各欄と、対象段階のSource Fingerprintを公開監査へ投影する。新しい公開CLI flagは追加していない。
- code-generationでは既存 `source_fingerprint::read` のworkspace-source-v2を再利用した。PlanChallenge・計画承認の受領は流用していない。ソースが観測不能なら本家同様のunbindableを保存し得るが、その後のゲート時拒否は後続区切りに残る。

## Redと是正

| 実行 | 実際に確認した失敗 | 是正 |
| --- | --- | --- |
| `cargo test -p aidlc --test review_receipt_contract` | 文書を置かずにreview要求するとexit 0、`{"emitted":"REVIEW_REQUESTED","stage":"requirements-analysis"}`。期待exit 1に対して1件失敗 | 安定した宣言成果物と追記先がない要求をdomainで拒否。該当1件Green |
| 同targetの固定本家request/completed比較 | requestの固定指紋は一致したが、completion後のRMUでseq 3のpayload復号が失敗した | 書込みDTOとRMU DTOの内部verdict語彙を既存のReady/NotReadyへ統一。公開READY/NOT-READYとは分離 |
| 既存RMU・DTOテスト | 「Fingerprint欄は存在しない」という旧期待と、history/evidenceのないワイヤfixtureが失敗 | 新しい必須欄を具体値で期待へ追加。assert削除やフィールド除外は行わず、内部語彙の不正入力試験にも他の正常な結合材料を与えた |

途中の署名・構文エラーはビルド整合の修正であり、Redの実証に数えていない。

## 成功した検証

| コマンド | 結果 | ログ |
| --- | --- | --- |
| `cargo check -p aidlc` | 成功 | 作業時tool結果 |
| `cargo test --workspace --no-run` | 全targetのコンパイル成功 | [compile.log](review-receipt-logs/compile.log) |
| `cargo test -p aidlc --test review_receipt_contract` | 5件成功 | [cli-green.log](review-receipt-logs/cli-green.log) |
| `cargo test -p core-command-domain --lib review` | 46件成功 | [domain-review.log](review-receipt-logs/domain-review.log) |
| `cargo test -p core-command-use-case --lib record_review_use_case` | 16件成功 | [use-case-review.log](review-receipt-logs/use-case-review.log) |
| `cargo test -p core-read-model-updater --lib review` | 7件成功 | [rmu-review.log](review-receipt-logs/rmu-review.log) |
| `cargo test -p core-command-interface-adapter --lib dto::tests` | 40件成功 | [dto-review.log](review-receipt-logs/dto-review.log) |
| `cargo test -p aidlc --test intent_lifecycle` | 123件成功 | [intent-lifecycle.log](review-receipt-logs/intent-lifecycle.log) |
| `cargo clippy --workspace --all-targets -- -D warnings` | 成功 | [clippy.log](review-receipt-logs/clippy.log) |
| `cargo lint` | 成功、出力なし | [lint.log](review-receipt-logs/lint.log) |

CLI 5件は、文書なし要求、固定本家の原文指紋、本文改変の拒否と復元後の再受領、code-generationのsource変更拒否、Review欠落とReviewer/Iteration/Verdict不一致を検証する。成功後の監査行を読み、Request/Completedの異なる指紋と追記境界を確かめる。

本家の既存U1コーパス `review/request` と `review/completed` を変更していない。要求指紋 `sha256:6d7abc84613dde3fdd397bb5324e9aff18f22b868a057a81434c883f80d14211`、完成指紋 `sha256:5fd9c73ce23a4726762079bc915a5c544a249f431a4a49f448435fb9f282223d`、追記offset 163が一致した。

## 既存試験の移行

- 純粋な集約・保存試験には、空bindingではなく原文と正規のReview節を持つ合成入力を追加した。共通の `tests/support/review_fixture.rs` とdomainのtest専用fixtureは実ワークフローの承認には使わない。
- intent_lifecycleの合成定義へreview_artifactとproducesを宣言し、要求前に実際の文書を作る。判定では要求時の本文を保ってReview節を書き、要求が発行したchallengeがあれば新しい節の先頭へ含める。差戻し後に古いReviewへ単に文字を継ぎ足していない。
- レビューsnapshot欄は完全な期待JSONへ追加した。併せて同じ保存fixtureに残っていたJumped.observation:nullとgenesisのPipelineHistory境界も、保存された新しい事実として明示した。比較から除外していない。

## 変更箇所

### 実装

- `modules/app/aidlc/src/runtime/review_documents.rs`、`runtime.rs::log_review/review_refusal`、`modules/app/aidlc/Cargo.toml`、`Cargo.lock`（既存workspace getrandomをnonce採取に使用）
- domain orchestrationの `review_artifact.rs`、`review_documents.rs`、`review_appendix.rs`、`review_binding.rs`、`review_completion.rs`、`review_evidence_error.rs`、`review_record.rs`、`review_history.rs`
- 同 `review_attempt.rs`、`stage_slot.rs`、`stage_slots.rs`、`intent_execution.rs` のレビュー部分、`command_error.rs`、`intent_execution_event/review_requested.rs`、`review_completed.rs`、`mod.rs`
- use-caseの `review_log_request.rs`、`record_review_use_case.rs`
- 書込み側・RMU側それぞれの `dto/review_binding_dto.rs`、`review_completion_dto.rs`、`review_requested_dto.rs`、`review_completed_dto.rs`、関連mod。書込み側の `review_record_dto.rs`、`intent_execution_dto.rs`、`intent_execution_event_dto.rs`
- RMU `workspace/projection.rs` のreview投影、`read_tables/spelling.rs` の新しい拒否種別

### 試験

- `modules/app/aidlc/tests/review_receipt_contract.rs`、`intent_lifecycle.rs`、`journal_protocol_conformance.rs`
- domain `intent_execution_event.rs`・`intent_execution.rs`・`stage_slot.rs`・`stage_slots.rs`・`review_attempt.rs` のレビューfixture、`tests/engine_loop_conformance.rs`
- domain `review_test_fixture.rs`、use-case `record_review_use_case.rs` のtest補助接続、`tests/support/review_fixture.rs`
- 書込み側とRMU側の `dto/tests.rs`、RMU `workspace/projection.rs` とtest用 `lib.rs` 宣言

親のReported/Validation/SourceBaseline実装、別担当のHealthChecked・jump製品処理は変更していない。共有enumの拒否分類と、上記保存fixtureの明示値だけは接続のため追従した。

## 現在の制限と後続区切り

この差分を完全な本家レビュー権限と扱わない。

1. **Review節の完全なMarkdown互換は未完。** 通常のcanonical太字フィールド、末尾のATX H1/H2、既存可視性処理を使っている。Bunの描画意味にあるHTML H1/H2、setext、複雑なcontainerやraw HTML内の偽証跡、CRのみの改行等の全コーパスは未接続・未検証である。単純な行走査だけで完全互換と主張しない。
2. **challengeのstdout結果Queryは未接続。** 既存Review節がある要求ではchallengeをイベント・監査へ保存してcompletionで照合するが、公開JSONのreviewChallenge欄を操作ID指定Query経由で返す処理はまだない。成功戻り値へ表示材料を足して代用していない。
3. **no-DAG stage-levelの採取に限定。** per-unit宣言では現在の採取がrequired強制を外す。通常bugfixの想定には合うが、DAGを持つstage-levelやunit/singleの一般化は未実装。正式な対象解決とdomainでの宣言集合照合も拡張が必要。
4. retryの元binding照合は新しい保存構造に接続したが、1回だけのretry制限、pending中の別iteration拒否は未実装。既存fixtureの予算意味は維持した。
5. 完成後改変の1回限定recovery、summary-confirmation受領の保存・再利用判断、ゲート前のartifact/source鮮度照合は未実装。`require_review_receipt` はこの区切りでは変更していない。
6. stable snapshotの異常ファイル・並行置換、既存節の再利用、構文境界は専用の固定本家コーパスによる追加検査が必要。比較失敗の全診断文言も、request/retry/completionで本家どおりに分ける残作業がある。

したがって推奨する次の行動は、親側の統合検証と現在差分の確認である。その後の実装継続は、上の未完部分を含めて区切りを選ぶ。実地Claudeスモーク、90%床・相対ゲート、全CI、本物のレビュー受領をこの試験で代用しない。
