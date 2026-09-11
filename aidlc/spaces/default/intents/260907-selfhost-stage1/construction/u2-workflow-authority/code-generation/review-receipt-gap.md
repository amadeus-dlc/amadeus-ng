# レビュー受領の内容・ソース結合の不足

## 結論と今回の範囲

通常Claudeのbugfixでは、要求分析とコード生成のレビュー受領が必要である。現在のnative実装はレビュアー名・iteration・判定の組を保存するが、**実際に読んだ成果物とソースに受領を結び付けない**。ファイルを一度も読まないまま判定を保存でき、完了後の改変も `require_review_receipt` では検出できない。

既存のRecordReviewUseCase→IntentExecution→SQLite→RMUを拡張するのが最小の変更である。別のレビュー用ストアや計画承認の代用は要らない。ただし指紋のフィールド追加だけでは足りず、安定した成果物採取、レビュー追記部分の検査、再試行・回復の回数制約、ゲート前の鮮度再確認まで接続する必要がある。

対象は承認済みU2計画Step 5の受領結合である。unit/single、多repo、Copilot/Kimi、自律実行の一般化は含めない。製品コードは変更していない。本記録は現コードの読取り調査であり、新たなテスト成功や互換完了の報告ではない。

## 固定元の確認

比較元は `a277af218f0df7f325d3b8be7b6d90fce2c5bd40`。vendorの現HEADは `801c570062f67dc8f4952ee5fc601381d09db7ec` だが、固定元との対象ファイル差分を確認した。

- `core/tools/aidlc-log.ts`：差分なし。
- `core/tools/aidlc-lib.ts`：`.kimi-code` の認識2箇所だけ。今回参照したreview関数には差分なし。
- `core/tools/aidlc-state.ts`：ハーネス文書ディレクトリに `.kimi-code` を加える1箇所だけ。今回参照したreviewガードには差分なし。

以下の関数名が固定元での追跡点である。nativeは並行変更中なので、行番号よりファイル名と関数名を優先する。

| 固定元 | 該当契約 |
| --- | --- |
| `core/tools/aidlc-log.ts::handleReview`、`reviewAttemptSummary` | 初回、retry、判定保存、1回限定のstale-receipt回復 |
| `core/tools/aidlc-lib.ts::reviewArtifactEntries`、`readStableReviewArtifacts`、`reviewArtifactSnapshot` | 宣言成果物集合、安定読取り、追記境界と指紋 |
| 同 `existingReviewAppendixOffset`、`reviewAppendixEvidenceBytes`、`reviewAppendixDigest`、`validateReviewAppendix` | 既存Review節の識別、古い証跡再利用の拒否、Markdownの可視性 |
| 同 `freshReviewReceipts`、`usesStageLevelPerUnitArtifacts` | 現試行の受領、変更失効、stage-level判断 |
| `core/tools/aidlc-state.ts::verifyReviewerPrecondition` | ゲート表示と完了時の受領・ソース鮮度 |

U1の既存 `tests/golden/upstream-a277af21/stage1/cases.json` には `review/request` と `review/completed` がある。両観測の環境にartifact/source/summaryガードの無効化flagはない。要求分析の実測は次のとおり。

| 面 | request | completed |
| --- | --- | --- |
| stdout | emittedとstageだけ | emittedとstageだけ |
| Artifact Fingerprint | `sha256:6d7abc84613dde3fdd397bb5324e9aff18f22b868a057a81434c883f80d14211` | `sha256:5fd9c73ce23a4726762079bc915a5c544a249f431a4a49f448435fb9f282223d` |
| Request Fingerprint | なし | requestのArtifact Fingerprintと同じ |
| Review Appendix Artifact | `inception/requirements-analysis/requirements.md` | 同じ |
| Offset / Prior Digest / Prior Length | 163 / none / 0 | 同じ |

stdoutのキー一致だけでは、この監査受領の不足は検出できない。

## bugfixで必要な2段階

固定元の `core/scopes/aidlc-bugfix.md` は `review_cap: advisory`。宣言と実効クラスは区別する。

| 段階 | レビュアー | 宣言→通常bugfixの実効値 | レビュー対象とソース |
| --- | --- | --- | --- |
| requirements-analysis | aidlc-product-lead-agent | advisory→advisory、通常1回 | requirements.md＋requirements-analysis-questions.md。追記先はrequirements.md。workspaceソース結合なし |
| code-generation | aidlc-architecture-reviewer-agent | adversarial→advisory、通常1回 | code-generation-plan.md、unit-test-instructions.md、code-summary.md、traceability.json。追記先はcode-generation-plan.md。workspace source v2結合あり |

bugfixはunits-generationをSKIPするため、通常のコード生成成果物は `<record>/construction/code-generation/` のstage-levelである。宣言が `for_each: unit-of-work` であることだけを根拠に、今回 `--unit` の実装を必須にしてはならない。

**本家の既存例外も保持する。** handleReviewは、per-unit宣言だがDAG不在・unit指定なし・merged Unitなしの場合、全required成果物の存在を強制しない。bugfixのstage-levelコード生成はこの条件に入り得る。追記先そのものは通常ファイルでなければsnapshotが成立しない。他の欠落はmanifestの`missing`として結合され、後から作成すると指紋が変わる。review requestに全成果物存在の条件を独自追加しない。完了時の成果物存在ガードとは別契約である。

## nativeの現在地

| 境界 | 実装済み | 確認した不足 |
| --- | --- | --- |
| `runtime.rs::log_review` | 引数、対象実行、unit/single拒否、RecordReviewUseCase、投影後の成功JSON | 成果物・追記・ソース・summary受領の入力採取がない。review_artifact宣言の検査もない。成功JSONは現在retryだけが可変で、challenge/recovery/upgradeは未対応 |
| `record_review_use_case.rs` | 定義から方針を解決、集約コマンド、成功unit、1回の楽観競合再試行 | 受領内容を渡さない。再試行でも同じ採取内容を保持して照合する契約が必要 |
| `intent_execution_event/review_requested.rs` | stage/reviewer/iteration/retry | request fingerprint、appendixのpath/offset/prior digest/length/challenge、要求時source、recovery属性がない |
| 同 `review_completed.rs` | stage/reviewer/iteration/verdict | 対応request fingerprint、完了後artifact fingerprint、appendix結合、request/completion sourceがない。docにも指紋繰延が明記されている |
| `review_attempt.rs`、`pending_iterations.rs` | 要求数、pending iteration集合、closed判定列、試行reset | pending要求の内容やretry済みフラグがない。`retry`イベントの適用は何も変えず、同iterationの再試行を何回でも受理し得る。回復消費の記録がない |
| `IntentExecution::request_review` | reviewer一致、budget、連番、retry対象pending存在 | 通常request時に別のpendingが残ることを拒否しない。bugfixでは通常budgetが先に効くが、一般の既存adversarial動作を正当化する理由にはならない。内容確認受領を検査しない |
| `record_review_verdict` | 対応するpending iterationの存在 | 原文やソースを一切見ず、Review節のない判定でも受理する |
| `ReviewClosures::has_terminal`、`require_review_receipt` | 現試行にterminal判定が1件でもあるか | どの内容をレビューしたか、後のpending、完成後の改変を検証しない。NOT-READYの扱いも古い「指紋なし」前提 |
| RMU `workspace/projection.rs::review_requested/review_completed` | 既存の少数フィールドを公開監査へ投影 | 新しい結合フィールドと、記録操作IDで取得する可変結果がない |

なお `AnswerRecorded::SummaryConfirmed` はイベントに証拠を保持するが、現在のIntentExecutionへの適用ではpending summary質問を消費するだけで、レビュー開始が照合できる確認済み証拠を状態に残していない。review requestの確認ゲート接続では、この受領履歴の保持も必要になる。公開監査を読み直して権限へ変換する代用は採らない。

`ReviewPolicy::budget/is_terminal` のadvisory規則そのものは再利用できる。advisoryのNOT-READYはレビューの終端であり、「レビュー未完了」や自動的な人間のRequest Changesと同一視しない。人間の差戻しは別のゲート操作である。

## 最小の振る舞い差分

### 1. 初回の依頼

現在の方針・対象照合に加えて、review_artifact宣言と、当該stageのsummary-confirmation受領を検査する。本家 `checkSummaryConfirmationEvidence` の条件を保持し、質問しない段階へ架空の確認を要求しない。

成果物集合はproduces＋optional_producesから解決し、論理パス順の `[logicalPath, sha256:bytes | missing | not-file]` 配列を契約JSONで直列化してSHA-256を取る。宣言外ファイルを混ぜない。レビュー追記先の既存末尾Review節を識別し、その手前のバイトoffsetまでをrequest fingerprintへ含める。完成後fingerprintは追記部分も含む。

要求時に既存Review節があれば、その実証部分のdigestとbyte lengthを保存し、`review:<32 lowercase hex>` challengeを新しく発行する。なければprior digestは`none`、lengthは0、challengeなし。コード生成だけ要求時のworkspace source v2も記録する。

採取は「各ファイルを順に読めた」で終えない。本家はpath配下のsymlink、hardlink、通常ファイル性、境界逸脱、open前後のdev/ino/mode/nlink/size/mtime/ctime、全読取り完了後の再照合を行う。並行書換えを混ぜたsnapshotを承認対象にしてはならない。

### 2. 未完了の再試行

同じpending iterationの `--retry-pending` は1回だけ許可し、通常要求数を増やさない。要求時の本文指紋、追記境界、prior証跡、challenge、必要なsource指紋を維持する。現在の本文やソースが変わっていれば再試行を拒否し、現在値で新しい基準を作って通さない。別iterationをpending中に始めることも拒否する。

retry後にもreviewerが成果を残さなかった場合に限り、空appendix＋NOT-READYのincomplete fallbackがある。最初から空ReviewでREADYを通す経路とは分ける。

### 3. 判定の受領と拒否

要求と同じ本文・ソース、同じ追記先・offsetを要求する。追記の先頭が要求前のReview節と同一で、そこへ説明を加えただけなら拒否する。

追記は正当なUTF-8で、空行の後に正確な `## Review` から始まる末尾節である。rendered Markdown上でcanonicalなVerdict・Reviewer・Iterationが各1件、値はCLI/requestと一致する。challengeは要求時に発行した場合のみ1件必須。後続の可視H1/H2やHTML H1/H2を拒否する。コード、引用、リスト、表、リンク内の文字列を証跡行として数えない。

受理時に単一ReviewCompletedイベントへrequestとcompletionの結合を保存する。拒否は型付きエラーで、pendingを消費しない。コード生成のsourceが`unbindable`でも、本家は要求/完了の同じ値を記録し得るが、後の鮮度ガードはfail closedとなる。要求時に無条件拒否する新条件を独自に設けない。

### 4. 完成後の改変と回復

ゲート表示・承認・段階完了で、その時点の宣言成果物集合のfingerprintを保存済みのcompleted fingerprintと比較する。コード生成はソースも比較する。過去のterminalの存在だけで通さない。本文だけでなくReview節の書換え、欠落成果物の追加、削除も対象になる。

terminal後に改変が起きた場合、pendingなし・回復未消費なら次のiterationを**1回だけ** `Recovery: stale-receipt` として許可する。advisoryの通常budget=1でも、この限定回復はあり得る。原因はartifact/source/artifact+sourceを区別する。回復中はゲート不可。回復後に再び古くなった場合、次の回復は認めず、人間のRequest Changes等による新しい試行境界を必要とする。

通常のRequest Changesは既存の試行resetを使う。ただし元Review節がファイルに残っていれば、新試行の要求はそのprior digest/length/challengeを保存する。試行resetだけで古いReview本文を新証跡へ昇格させない。

## 既存部品の再利用と限界

| 部品 | 再利用できること | そのまま代用できないこと |
| --- | --- | --- |
| `source_fingerprint.rs::read` | 通常単一rootのworkspace-source-v2。固定本家のsource-fingerprintコーパスあり | 読込み不能を現時点と同じ扱いへ写す必要がある。多repo・未対応ファイル種を今回黙って一般化しない |
| 親担当の `source_baseline.rs`／domain `SourceBaseline` | 開始・jump等のソース基準、必要時のlisting共有 | baselineは「そのreviewerが読んだ要求時ソース」ではない。review requestの指紋の代わりにしない |
| `validation_basis.rs` | 成果物名→ファイル名、グラフ宣言、単一ファイルSHA-256、正準JSONの部品 | stage validityの構造指紋はreview manifestと異なる。optional欠落の扱い、path表現、snapshot安定性、Review byte境界が違う |
| `runtime/pipeline_link.rs` の読取り | O_NOFOLLOW/O_NONBLOCK、通常ファイル性、読込み前後のmetadata検査の既存例 | 単一handoffのhash＋mtimeでは、全成果物の同時snapshot、hardlink、全ファイル再照合、appendix契約を満たさない |
| PlanApprovalEvidence／PlanChallenge | 型付き入力の検証、保存→RMU→ID指定Query、challenge値の境界を分ける設計例 | 人間の計画承認とreviewer受領は別契約。plan fingerprintは計画・テスト指示の原文とTesting Contract、target、intent、directive epoch、run/source floorを正準JSONへ束縛するため、reviewのファイルmanifest指紋とは異なる。PlanChallengeのsession/回答照合をそのまま移さない |
| SummaryQuestions／SummaryEvidence | 既存summary確認の検証・可視性処理の参照例 | Review節のBun Markdown描画意味、canonical太字フィールド、byte offset検査とは契約が違う。`markdown_sections::extract_section`の文字列処理で代用しない |
| ArtifactAudit／ArtifactWriteObservation | 保存操作の監査事実、将来の凍結ガードへの入力候補 | 現在はpath/tool/context/createdだけで内容SHA-256なし。shell経由の改変もあるため、監査の有無だけで鮮度を判定しない |

レビュー専用の完全なartifact fingerprint/snapshotはnative内に見当たらない。「既存のartifact fingerprintを呼ぶだけ」で閉じる差分ではない。

## 層を保った実装案

1. 既存定義からreview対象を解決するドメインの値と、境界が読んだファイル群の観測値を分ける。安定I/Oはapp/adapter側、対象集合・指紋・appendixの受理判断はdomain側に置く。use-caseでドメインのgetterを並べて検証しない。
2. ReviewRequested/Completedに結合を保持する値を追加する。全本文・集約の写しをイベントへ積まず、必要な指紋・境界・challenge・回復属性を保存する。
3. ReviewAttempt/PendingIterations/ReviewClosureを、要求の結合・retry消費・terminal受領・回復消費を表現できる形へ変更する。setterや旧経路の並立を作らず、既存snapshot DTOも完全な状態へ合わせる。
4. RecordReviewUseCaseの成功unitと単一イベントを維持する。可変のreviewChallenge/recoveryをstdoutへ返すために成功戻り値を増やさず、呼出前に生成する内部操作IDでイベント→RMU→結果Queryを結ぶ。既存report結果の方式を利用する。公開CLIへID flagを増やさない。
5. `require_review_receipt` の呼出へ現在ファイルの観測を渡し、集約の受領と比較する。Queryが現在のファイルを読んで可否を再判断する経路は作らない。
6. RMUの両review監査投影へ本家の欄と順序を追加する。失敗は既存の公開入口 `runtime::run` がFace::Logに対して `log_failure::finish` へ渡すため、ERROR_LOGGEDの独自writerは追加しない。log_review内のCompletion::refusedだけを見て、失敗監査が未接続と誤分類しない。

## 最初に固定すべき受入ケース

| 順序 | 公開境界で先に失敗させる振る舞い |
| --- | --- |
| 1 | 既存U1のrequirements request/completedをstdoutだけでなく監査欄全文で比較 |
| 2 | 要求前summary未確認、review_artifact不在、directory/symlink/hardlink、読取り中の置換を拒否 |
| 3 | 正しい末尾Reviewだけを受理。CLIとVerdict/Reviewer/Iteration不一致、古い節の延長、二重行、隠れた偽証跡、UTF-8不正を拒否 |
| 4 | pending中の別request拒否、同iteration retry1回、2回目拒否、retry時本文/ソース変更拒否、限定NOT-READY fallback |
| 5 | code-generation request→source変更→completion拒否。要求/完了が一致してもunbindableならゲート拒否 |
| 6 | completion後の本文/Review節/ソース変更→ゲート拒否、1回の回復→受理、二度目の改変→再回復拒否 |
| 7 | 人間の差戻し後にiterationを初期化しても、残存Review節に新challengeが必要 |
| 8 | 実SQLite→RMU→操作IDQuery、再投影、並行同iteration、保存と公開の間の失敗・回復で受領を取り違えない |

今回コード・テストは追加していない。単に既存テストを増やす件数目標ではなく、本家の失敗条件が先に検出される順で進める。

## 裁定と境界

固定本家に一致させる上の受領結合自体は、承認済みStep 5内であり、新しい業務裁定を要する不一致は見つかっていない。

追加裁定が必要になるのは、次のように契約や対象を変える場合である。

- **推奨：通常bugfixのstage-level契約を完全に実装する。** advisoryのNOT-READY、code-generationのno-DAG例外、1回だけの回復、Markdownの可視性まで維持する。Rust内の既存依存にMarkdown rendererは見当たらないため、既存可視性部品の拡張か適切なparser導入を実装時に選び、本家コーパスで検証する。
- **契約を縮小する場合：人間へ裁定を求める。** 単なる行検索でReview節を判定する、指紋が取れなくてもterminalで通す、retryを無制限にする、通常advisoryをadversarialに変える、no-DAGの欠落条件を強化する等は互換条件の変更である。

review-freeze/reviewer-scopeはStep 7の別の接続責任である。現在modules内では関連監査語彙だけが見つかり、フック実装は確認できなかった。受領の鮮度判定をこのフックと共有できる形にするが、本調査からフックの一般化まで自動追加しない。受領の書込みとゲート時の再照合を実装しても、主要フック全体の完成とは呼ばない。
