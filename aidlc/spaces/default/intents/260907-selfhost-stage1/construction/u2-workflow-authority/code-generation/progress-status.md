# U2の進捗整理

2026-09-09。利用者の「u2 step1まだ完了しないのか。どういう状態？」を受け、計画と実行証跡を照合した。工程全体の完了・承認を記録する文書ではない。

## 計画の9ステップと現在地

| ステップ | 状態 | 根拠と残り |
| --- | --- | --- |
| 1 入力・実行基盤 | 完了 | TDD証跡冒頭に、実装前の既存reportテスト30件成功を記録済み。U1入力と基準コミット・CIの確認はintegration-baseline.mdにある。未更新だった計画のチェックだけを是正した |
| 2 報告事実 | 完了 | 現在のコードで報告ユースケース32件成功。単一Reported、no-op、拒否、1回だけの競合再試行と対象固定を再確認。更新成功はunitのみ |
| 3 保存・投影・結果Query | 完了 | 現在のコードで保存/結線1+1、結果投影5、DAO5、公開復旧33、CLI結合118件が成功。対象Clippy・整形・差分検査も成功。詳細はstep2-3-verification.md |
| 4 開始・進行CLI | 一部完了 | 実走査、next入力、continue等を接続。開始時のSource Baselineは実装・独立検証済み。工程境界のBaselineや残る2.7.1差は未完了 |
| 5 質問・回答・承認 | 一部完了 | 内容確認・計画承認の主要経路を実装。link等の未接続操作と残る拒否境界がある |
| 6 主要フック | 追加修正中 | step6-verification.mdの適用表・公開契約・障害回復を検証済み。Continuation/HookHealth/ArtifactAuditのSQLite・Memory共通契約も成功。配布への実接続はU4 |
| 7 補助更新 | 実装中 | session-start/end・subagent完了・PreCompactの接続に着手。learnings persist・診断実施記録等の必須接続が残る |
| 8 受入基準移行 | 実装中 | 基盤・配布JSON・監査描画の移行を進めた。開始時Source Baseline是正後、状態/監査投影の18テスト中11成功・7失敗。CLI比較は10件中9成功・1失敗（再報告時の根拠再検証）。いずれも残タスク総数ではない |
| 9 統合検証・引継ぎ | 未完了 | 全workspace、カバレッジ絶対/相対ゲート、release、最終成果物、独立レビュー、CIと統合が残る |

Step 1で実装が停止していたのではなく、チェック更新を怠ったまま複数の後続ステップを進めていた。計画の本文・Testing Contractは変更していない。

## 遅延と進捗表示の問題

- 旧コーパス消費者を2.7.1へ切り替える作業が後になり、監査情報等の実装漏れを遅れて検出した。
- 設計規則に反する構築・setter/getter・更新結果の返却等の是正が途中で発生した。
- 局所的な成功件数を中心に報告し、9ステップ全体の残量を示せていなかった。「8件」は一部の投影テストの失敗数であり、U2完了までの残り8作業ではない。
- 一時停止と再発行により計画承認を再度求める進め方を採り、余分な確認を発生させた。

## 裁定待ち

センサー由来の監査3種類を今回のnative互換判定でどう扱うかは `sensor-audit-comparison-questions.md` の回答待ち。これは未完了項目の一部であり、この回答だけでU2が完了するわけではない。

根拠は `code-generation-plan.md`、`tdd-evidence.md`、`integration-baseline.md`、`stop-checkpoint.md`、各 `*-migration.md`。

## Step 6の追加進捗

次の部品検証に続き、step6-verification.mdの全適用項目と最終回帰を完了した。

- Claude会話履歴の判定: 本家87観測一致。`stop-transcript-verification.md`。
- 状態に束縛された共有再開待ちの読取り: 本家127観測一致。`stop-resume-wait-verification.md`。
- 成果物監査の再構成: コマンド再実行を除去。ドメイン7件、実SQLiteでの差分復元、CLIフック10件成功。`artifact-replay-verification.md`。
- 停止結果の要求IDによる復旧、不正な初回カウンターの拒否、共有再開待ちの維持: 実装担当とは別に各テストを実行して成功。

上記のロック・古い状態・公開失敗・回復を含む残項目を解消し、Step 6のチェックを完了へ更新した。詳細と断面はstep6-verification.mdとstep6-source-checkpoint.jsonを参照する。

ユーザーの「tools/lint定期的に実行せよ」に従い、各実装・修正の区切りで `cargo lint` を実行して違反を是正する。最終検査だけに集約しない。

## 成果物監査の追加発見

Step 7の調査を契機に、同じ対象へのWrite→Editで作成監査が重複する不具合を追加再現した。既存テストは作成・更新行の存在だけを見ており、重複と最新Queryを検査していなかった。Step 6のチェックを一時的に未完へ戻し、親担当が差分監査と構造化投影を是正・再検証する。

## 2026-09-09 再開後の照合と追加検証

- 保存監査の重複は `artifact-repeat-verification.md` のとおり前回後半に是正済みだった。古い「修正中」の行だけを根拠に再実装していない。
- pipeline完了情報の表示と完了前提も接続済みだった。今回の公開契約12件と静的検査が成功し、SQLite投影失敗・復旧の検査を1件追加した。`pipeline-consumption-verification.md`。
- PreCompactの入口・監査・復旧記録を接続し、セッション関連17件が成功。保存した所有者に限る文脈失効を含む。`session-precompact-verification.md`。
- Validation Basisを報告時に保存して監査へ描く経路を追加した。欠落Red→Green、本家22観測一致、CLI結合122件成功。最終静的検査と共有変更後の再検証は継続中。`validation-basis-verification.md`。
- 状態/監査の全比較は、再開直後の18件中11成功・7失敗からまだ完了判定していない。次工程Source Baselineとjumpの保存・投影を実装中。センサー監査3種類の比較範囲は回答待ちを維持する。

これらはStep 4〜8の部分進捗であり、Step 7全補助操作・Step 9統合検証・U2全体の完了ではない。

## 2026-09-10 の棚卸し（承認済み計画は書き換えない）

利用者の「CG の Step 4 を完成させないといけない」「計画は終わっているかもしれないので更新して」を受けて実測した。

**承認済みの `code-generation-plan.md` は凍結された成果物であり、進捗をそこへ追記してはならない。** 一度追記したところ、承認指紋（計画本文のバイト列に対するハッシュ）が失効し、担当 `u2_continuation_rejection` の Rust 実装・テスト・cargo 実行がすべてガードに拒否された。追記を取り消して承認時のバイト列へ戻したところ、`aidlc-testing-posture.ts verify` が `ok: true`（`fingerprintValid` / `receiptValid` とも true）へ復帰した。**進捗はこのファイルへ書く。**

### Step 4（未完了）

完了している範囲: 実走査、next 入力、continue、工程開始時の Source Baseline（[証跡](report-source-baseline-verification.md)）。narration と conductor_persona は `cli_golden_test.rs` が欠落を許す例外を設けず本家一致を固定している（`upstream_271_contract.rs` の `the_first_reverse_engineering_persona_and_narration_match_upstream`）。

**未完了は継続トークンの拒否 3 経路である。** 計画の「古い/不正/別対象のトークンを拒否する」に対し、本家 `aidlc-orchestrate.ts` は 4 種類の拒否文言を持つが、本 build は 8366 の「不正（開封不能）」だけを実装している。

| 本家の行 | 条件 | 本 build |
| --- | --- | --- |
| 8366 | 開封不能 | 実装済み（`wording.rs:86` の `INVALID_CONTINUATION_TOKEN`） |
| 8488 | 同じトークンの再提示（superseded） | 未実装 |
| 8489 | 準備中に作業文脈が変化 | 未実装 |
| 8493 付近 | 継続調整のロック競合 | 未実装 |

`grep -rn "no longer current for this workflow" modules/` は 0 件。親は本セッションでこの差を実際に踏んだ（消費済みトークンの再提示に対し TypeScript のエンジンは 8488 を返した）。担当 `u2_continuation_rejection` へ委譲済み（記録先 `continuation-rejection-verification.md`）。

### Step 5（部分完了）

`link` は `cli/request.rs:192` で `Request::LogLink` へ接続済みであり、旧記録の「link 等の未接続操作」は解消している。decision / answer / review、内容確認、実装計画承認の主要経路も実装済みで、本セッションでも計画承認の decision / answer が `PLAN_APPROVAL_RECORDED` まで通った。残る拒否境界の全数は未棚卸しであり、Step 8 の全数差分で確定させる。

### Step 7（大半が完了、2 件進行中、1 件未着手）

棚卸し（[remaining-step7-inventory.md](../remaining-step7-inventory.md)）の各行に対する現在地。

- **完了**: セッション開始/終了・subagent 完了・PreCompact、作成帰属、診断実施記録の U2 API、TaskUpdate 同期（`sync-workflow-state`）、runtime-graph 再構築（[証跡](runtime-graph-verification.md)）、review-freeze（[証跡](review-guards-verification.md)）、シェル書込み先の解析（[証跡](shell-write-targets-verification.md)）、learnings surface / persist（[証跡](learnings-verification.md)）
- **進行中**: reviewer-scope の越境拒否と `REVIEWER_SCOPE_BLOCKED`（`u2_reviewer_scope`）、規則受渡し `deliver-stage-rules`（`u2_stage_rules`）
- **未着手**: fold-usage
- **裁定待ち**: per-unit 受領証の配線（[step7-divergence-questions.md](step7-divergence-questions.md) Q1。利用者は実走行の採取を選択し `u2_upstream_capture` へ委譲済み）

### 統合の実測（2026-09-10）

4 スライスが同じ木に載った状態で `cargo test --workspace --no-fail-fast` が **97 スイート・2,921 件成功・0 失敗（exit 0）**。`cargo fmt --all --check`、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo lint`、`bash scripts/quint-gate.sh`（全 29 ステップ）、`bun scripts/aidlc-sync.ts --check` もすべて成功。全文ログは [step9-logs/workspace-test-2026-09-10.log](step9-logs/workspace-test-2026-09-10.log)。

これは Step 9 の一次証拠であって、Step 9 の完了ではない。release バイナリ契約、カバレッジ床と相対ゲート、CI 全ジョブ、最終成果物（source-manifest / traceability / code-summary）、独立レビュー、Unit 完了はいずれも未了である。

## 今回の差分を閉じた時点

親の最終検査で、主要app結合8target・175件が成功し、workspace全target Clippy、独自lint、fmt、diffの検査も成功した。詳細はresume-validation.md末尾。

現在の3つの裁定待ちは、センサー監査、direct execute、TaskUpdate同期。レビュー受領はsnapshot結合のsliceまで完成し、retry/recovery/summary/ゲート前鮮度等は未完了。診断記録はAPIとSQLite結合まで完成し、実doctorとcold非呼出しの配線はU3の責任。Unitや工程の完了を記録せず、回答後に残る実装と全体検証を続ける。

## 2026-09-10 裁定 Q1 = B の実装（per-unit 受領証方針の配線）

利用者が [step7-divergence-questions.md](step7-divergence-questions.md) Q1 に **B「`ReviewPolicy::per_unit` を配線する」** を選択したため、`u2_per_unit_policy` が実装した。証跡は [per-unit-policy-verification.md](per-unit-policy-verification.md)、生ログは [per-unit-policy-logs/](per-unit-policy-logs/)。

受入の中心「ゼロ Unit の per-unit ステージで凍結しない」を満たした。採取スクリプトを再利用した配線後の走行で、条件 B の 2 宛先が**両側とも exit 0・`REVIEW_FREEZE_BLOCKED` 0 行**になり差が消えた。陽性対照（条件 A）と陰性対照（条件 D）は配線前と同じく一致したままである。

**配線後も残る差を実測した（推測ではない）。** ステージ水準の受領証しか無いとき、本家は Unit 宛先の書込みを通すが、本 build は拒否する（exit 2・1 行）。依頼が「Unit 有りの場合の凍結は維持する」と明示したとおりの状態であり、向きは「本 build のほうが厳しい」である。切替条件 2 の判定前に人間の裁定が要る。

同じ走行で、裁定前は組めなかった条件 C（Unit 鍵の受領証）が本家側で成立した。本家は Unit 鍵の受領証が在るときステージ水準の曖昧な宛先を閉じる側へ倒し、兄弟 Unit は通す。本 build は `aidlc log review --unit` が未配線でこの状態へ到達できないため、この分岐は写していない（依頼の対象外）。

これは Q1 の実装完了であって、Step 7 全体・Step 9・U2 の完了ではない。

## 2026-09-10 午後の再開（Step 7 の着地、Step 8 の棚卸しと裁定）

計画承認を取り直した（指紋 `sha256:a22159d4…`、詳細は [current-resume-note.md](current-resume-note.md)）。

### Step 7（すべて着地）

- **fold-usage**: `u2_fold_usage` が台帳の畳み込みを実装。本家採取 40 観測（bare 17・intent 17・rotate 5・disabled 1）で終了コード・stdout・stderr・`aidlc/` 配下の全ファイル・`.aidlc-sessions` の生成物一覧までバイト一致。[fold-usage-verification.md](fold-usage-verification.md)。
- **deliver-stage-rules**: `u2_stage_rules_closeout` が固定コミットの実バイトを 14 個の一時ワークスペースで走らせ、corpus 89 件中 80 件完全一致、BOM の写し漏れ 1 件を TDD で是正。[stage-rules-verification.md](stage-rules-verification.md)。
- **reviewer-scope**: `u2_reviewer_scope_closeout` が本家フックを 32 ケース実走行させ、`cwd` 欠落時の基点の写し漏れ 1 件を TDD で是正。parity 165 件差分 0。[reviewer-scope-verification.md](reviewer-scope-verification.md)。

### Step 8（棚卸し → 裁定 → 是正中）

`u2_step8_audit` が 5 箇条を実測で棚卸しし（[step8-migration-verification.md](step8-migration-verification.md)）、親が主セッションで classic scope の probe を実行して本 build と本家採取を突き合わせた。利用者の裁定は [step8-decisions-questions.md](step8-decisions-questions.md)（Q1〜Q4 すべて A）。

- 契約で是正が決まる差 D1〜D4・D6（作業なしの `next` の error、Greenfield での reverse-engineering の SKIP、`Request` 欄の `/aidlc ` 前置、`bundle` の `sha256:` 接頭辞、再報告時の根拠再検証）と D5（print directive の綴りと `next --stage` の意味、裁定 Q1 = A）を `u2_classic_parity` が是正中。先頭 4 ケースを `classic_corpus_contract.rs` へ固定する（裁定 Q4 = A）。
- 旧コーパス `upstream-3c3146cf/` と `supplemental-3c3146cf/` を `git rm`、`coding-rules/README.md` の正本記述を a277af21 へ更新（裁定 Q2 = A）。
- コーパス内の旧ピン説明文を修正し、`reviewer-scope/cases.json` の追加分を含めて `corpus-manifest.json` を再封印。`verify-corpus.ts` 成功、採取テスト 47 件成功（R1、裁定 Q3 = A）。
- 残る未駆動ケースは [switchover-condition2-findings.md](switchover-condition2-findings.md) F4 に記録（裁定 Q4 = A）。

### フック着地確認で出た差（裁定 → 是正中）

利用者の裁定は [hook-closeout-decisions-questions.md](hook-closeout-decisions-questions.md)（Q1〜Q4 すべて A）。

- 契約で是正が決まる差（監査値の `<project-dir>` 伏せ字、状態欄読取りのタブ、カーソル無しの唯一記録への後退、規則 BOM の RMU 側、Stop 時の台帳 flush-all）を `u2_hook_parity` が是正中。
- 差し向け記録の `stage` / `unit` の逐語強制（裁定 Q1 = A）を `u2_dispatch_grammar` が実装中。
- `tool_name` null の exit、stderr の原因文言、完了監査の利用量欄は記録のみ（F5〜F7）。

### Step 9（未着手）

3 担当の着地後に、fmt / clippy / lint、workspace 全体の通し実行（全文ログ）、Quint ゲート、カバレッジ（90% 床と相対ゲート）、release バイナリ契約、`cargo audit`、`aidlc-sync --check` を親が 1 回行う。その後 `source-manifest.json` / `traceability.json` / `code-summary.md` を書き、独立レビューへ進む。

## 2026-09-10 夕方の再開（利用量上限からの復帰）

前セッションは 3 担当を出した直後に利用量上限で止まり、3 担当とも作業ツリーへの変更を残していなかった。計画承認を取り直し（指紋 `sha256:f1d7f6e0…`、詳細は [current-resume-note.md](current-resume-note.md)）、fmt / clippy / `cargo lint` / verify-corpus / aidlc-sync --check がすべて成功する状態から、残作業を 2 担当へまとめ直して再開した。

- `u2_classic_parity`: Step 8 の是正 D1〜D4・D6、Q1 = A、classic 先頭 4 ケースのテスト固定、R7・R9 → [classic-parity-verification.md](classic-parity-verification.md)
- `u2_hook_parity`: フック差の是正 5 件（`<project-dir>` 置換、状態欄のタブ、lone-intent 後退、Stop の flush-all、差し向け記録の逐語強制）と reviewer-scope の残課題 3・4 → [hook-parity-verification.md](hook-parity-verification.md)

Step 9 と最終成果物・独立レビュー・Unit 完了は、2 担当の着地後に親が行う。

## 2026-09-10 夜の再開（2 担当の途中変更を 1 担当で着地中）

前セッションは 2 担当（`u2_classic_parity` / `u2_hook_parity`）が 21:02 頃まで TDD を進めた途中で止まり、**両担当の未完変更が作業ツリーに残り、報告書は未作成**だった。計画承認を取り直し（指紋 `sha256:b83cf443…`、詳細は [current-resume-note.md](current-resume-note.md)）、再開直後に実測した。

| 検査 | 結果 |
| --- | --- |
| `cargo check --workspace --all-targets` | exit 0 |
| `cargo fmt --all --check` | 5 ファイルに差 → `cargo fmt --all` で整形済み |
| `cargo test --workspace --no-fail-fast` | **3,098 件成功・25 件失敗**（[全文ログ](step9-logs/workspace-test-2026-09-10-resume-baseline.log)） |

### 25 件の失敗の内訳（両担当の所有ファイルにまたがる）

- classic 側: `directive_drawing` 5 件・`turn` 4 件（`aidlc` lib）、`classic_corpus_contract` 2 件（`next --stage` の print 文言が本家逐語になっていない、`continue` の `consumes` が本家 `[]` に対し codekb 6 パス）、`intent_lifecycle` 2 件、`next_branches` 1 件、`runtime_graph_contract` 1 件、`task_update_contract` 2 件
- hook 側: `claude_hook_contract` 1 件（`ARTIFACT_CREATED` が監査に出ない）、`learnings_contract` 1 件、`upstream_271_contract` の stop 系 6 件

### 現在の担当

並行させると同じテストファイルで衝突するため、残作業を 1 担当 `u2_parity_closeout` へ直列で引き継いだ（記録先 `parity-closeout-verification.md`、`parity-closeout-logs/`）。引き継ぎ対象は前セクション「2026-09-10 夕方の再開」の 2 担当分すべて（classic: D1〜D4・D6、Q1 = A、4 ケース固定、R7、R9 / hook: 5 件の是正、差し向け記録の逐語強制、reviewer-scope 残課題 3・4）。

### Step 9（未着手）

担当の着地後に親が行う。順序は [current-resume-note.md](current-resume-note.md) の「この後の順序（親）」のとおり。

### 計画のチェック更新（2026-09-10 夜）

利用者の指示で `code-generation-plan.md` のチェックを実態へ合わせた。Step 4（継続トークン拒否 3 経路は [continuation-rejection-verification.md](continuation-rejection-verification.md) で着地、残課題 7.1 は裁定待ちとして記録）、Step 6、Step 7 を `[x]` にした。Step 5（受領の拒否境界の全数は Step 8 の差分で確定）、Step 8（是正中）、Step 9（未着手）は `[ ]` のまま。チェック更新後も `aidlc-testing-posture.ts verify` は `ok: true`（承認指紋は失効しない）。

### 担当 `u2_parity_closeout` の着地（2026-09-11 未明）

- `cargo test --workspace --no-fail-fast`: **25 件失敗 → 0 件失敗**（101 スイート・3,123 件成功、exit 0）。[全文ログ](parity-closeout-logs/workspace-test-final.log)
- fmt / clippy `-D warnings` / `cargo lint` / verify-corpus / `aidlc-sync --check` すべて成功。報告書は [parity-closeout-verification.md](parity-closeout-verification.md)、TDD 生ログ 27 本は `parity-closeout-logs/`。
- classic 側（D1〜D4・D6、Q1 = A の print directive 逐語化と `next --stage` の execute 名指し、classic 先頭 4 ケースの固定、R7、R9）と hook 側（5 件の是正、差し向け記録の逐語強制、reviewer-scope 残課題 3・4）はすべて完了。
- **裁定待ち F-H1**: カーソルが名指す実在ディレクトリに `aidlc-state.md` が無いときの扱い（本家: 無視して唯一記録へ後退 / 本 build: 記録として選びジャーナルから復元）。担当は復元側へ倒して着地、人間の裁定を求める。
- 記録のみ: F-C1（`next --compose` の print 文面の構造差、スモーク経路外）、F-C2（gate-start の残り 3 ガードは初回にも無い別件）、F-C3（Brownfield の `consumes` 解決先、今回の 4 ケースでは観測されない）、F-X1（`pipeline_link_contract` の高負荷時の揺れ、変更と無関係）。
- Step 8 は F-H1 の裁定を残して着地。Step 9 はこれから親が行う。

### F-H1 の裁定と是正（2026-09-11）

利用者は [lone-intent-fallback-questions.md](lone-intent-fallback-questions.md) Q1 に **B「本家に合わせる」** を選択。`layout.rs` `shared` の実在判定を本家どおり `aidlc-state.md` の有無へ戻し、`intent_lifecycle` の復元テスト 2 件を「カーソル無し・唯一記録」の前提へ見直す作業を担当 `u2_lone_intent_upstream` へ委譲中（記録先 `lone-intent-upstream-verification.md`、`lone-intent-upstream-logs/`）。着地後に Step 9 へ進む。

### F-H1 = B の是正着地（2026-09-11 10:50 頃）

`u2_lone_intent_upstream` が `layout.rs` `shared` の判定を本家どおり `aidlc-state.md` の有無へ戻し、復元テスト 2 件を「監査シャード・memory の投影欠損を復元する」前提へ見直した（状態ファイル自体を失った記録は本家同様「状態なし」）。`cargo test --workspace --no-fail-fast` は **101 スイート・3,124 件成功・0 失敗**、fmt / clippy / `cargo lint` 成功。報告書 [lone-intent-upstream-verification.md](lone-intent-upstream-verification.md)、ログ `lone-intent-upstream-logs/`。F-H1 は解消し、切替条件 2 の判定材料から外す。Step 8 完了。Step 9 を親が開始。

### Step 9 の進行（2026-09-11、親）

| 検査 | 結果 | ログ |
| --- | --- | --- |
| `cargo test --workspace --no-fail-fast` | 101 スイート・3,124 件成功・0 失敗 | [lone-intent-upstream-logs/workspace-test-final.log](lone-intent-upstream-logs/workspace-test-final.log) |
| fmt / clippy `-D warnings` / `cargo lint` | 成功 | lone-intent-upstream-logs/06 |
| `bun scripts/aidlc-sync.ts --check` | 成功 | step9-logs/s9-aidlc-sync-check.log |
| CI の bun 配布テスト 7 ファイル | 125 件成功・0 失敗 | step9-logs/s9-bun-distribution-tests.log |
| CI の bun ゴールデン採取テスト 5 ファイル | 47 件成功・0 失敗 | step9-logs/s9-bun-golden-tests.log |
| `verify-corpus.ts tests/golden/upstream-a277af21` | 成功 | step9-logs/s9-verify-corpus.log |
| `tools/lint` の fmt / clippy / test | 成功 | step9-logs/s9-tools-lint.log |
| `cargo audit`（workspace と `tools/lint/Cargo.lock`） | 成功 | step9-logs/s9-cargo-audit.log |
| Quint ゲート / release ビルド / release 契約 / カバレッジ | 実行中 | step9-logs/s9-quint-gate.log ほか |

計画のチェックは Step 5・8 を `[x]` にした（受領の拒否境界は Step 8 の全数差分で確定、Step 8 は F-H1 解消で着地）。Step 9 は残りの検査と最終成果物・独立レビュー・Unit 完了まで `[ ]`。

#### Step 9 追加結果（2026-09-11）

| 検査 | 結果 | ログ |
| --- | --- | --- |
| `bash scripts/quint-gate.sh` | 全ステップ成功 | step9-logs/s9-quint-gate.log |
| `cargo build --release` | 成功 | step9-logs/s9-release-build.log |
| `cargo test -p aidlc --release --test upstream_271_contract` | 75 件成功・0 失敗 | step9-logs/s9-release-upstream-271.log |
| `bash scripts/coverage.sh --base main`（1 回目） | **失敗**: llvm-cov 計測下で `claude_hook_contract` の 2 件（`state_guard_ignores_invalid_utf8_stdin_without_initialization`、`state_transition_guard_matches_the_four_fixed_upstream_boundaries`）が「一時ディレクトリが空」の検査で落ちた。原因は `env_clear` で `LLVM_PROFILE_FILE` が落ち、計測済み子プロセスが cwd へ `.profraw` を書くこと（計測の副産物で、フックの挙動ではない） | step9-logs/s9-coverage-red-profraw.log |
| 是正 | `claude_hook_contract.rs` の子プロセス起動 12 箇所に `coverage_profile_env()`（`LLVM_PROFILE_FILE` が設定されているときだけ引き継ぐ）を追加。通常実行 10 件成功、`cargo llvm-cov -p aidlc --test claude_hook_contract` でも 10 件成功。閾値・アサートは変更していない | — |
| clippy / `cargo lint` / coverage（再実行） | 実行中 | step9-logs/s9-clippy-lint-final.log、s9-coverage.log |

最終成果物の準備: `source-manifest.json` を作業ツリーの変更（1,301 経路。削除済み旧コーパス 2 本はディレクトリ claim、`memory/` は除外）から生成した。`traceability.json` は既存（FR1〜FR4・FR7・NFR1〜NFR4、FR7 は Deferred）で対象ファイルの実在を確認済み。`code-summary.md` は coverage の結果を待って書く。

#### カバレッジゲートの結果（2026-09-11）

`.profraw` の副産物を `tests/support/coverage_profile_env.rs`（18 テストファイルの `env_clear` 直後で `LLVM_PROFILE_FILE` を引き継ぐ）で是正した後、`scripts/coverage.sh --base main` は **絶対床 PASS（head 94.77%）・相対ゲート FAIL（base 99.15%、差 4.39 ポイント）**。未カバー 4,532 行を約 745 行以下へ減らす必要がある。詳細と per-file 一覧は [coverage-gap.md](coverage-gap.md)。ゲートは緩めず、進め方を利用者へ提示中。clippy / `cargo lint` は最終状態で成功（step9-logs/s9-clippy-lint-final.log）。

#### カバレッジ向上の委譲（2026-09-11、裁定 A）

利用者は「A. テストを足してゲートを通す」を選択。crate 所有で衝突しないよう 5 担当へ並行委譲した（共通 brief と対象一覧は `coverage-logs/`）。プロダクトコードは変更せずテスト追加のみ、dead code は削除せず候補として報告させる。

| 担当 | 所有 crate | 対象の未カバー行 |
| --- | --- | ---: |
| `u2_cov_app` | `aidlc`（app） | 1,194 |
| `u2_cov_cmd_ia` | `core-command-interface-adapter` | 576 |
| `u2_cov_rmu` | `core-read-model-updater` | 988 |
| `u2_cov_domain_uc` | `core-command-domain` / `core-command-use-case` | 1,088 |
| `u2_cov_harness_query` | `harness-*` / `core-query-*` / `core-infrastructure` | 517 |

各担当の着地後に親が fmt / clippy / lint / workspace 全通し / `coverage.sh --base main` を 1 回行い、`code-summary.md` → 独立レビュー → Unit 完了へ進む。

- `u2_cov_harness_query` 着地: 対象 517 行 → 71 行（86.3% 減）。追加テスト 95 件、所有 crate 524 件成功。harness-infrastructure 64.8%→98.2%、harness-claude 77.2%→96.1%。dead code 候補 8 件（特に `shell_write_targets.rs:485` の `${PWD}` 分岐、本家との同一性未確認）は裁定待ちとして [u2_cov_harness_query-verification.md](coverage-logs/u2_cov_harness_query-verification.md) に記録。残り 4 担当は作業中。
- `u2_cov_rmu` 着地: 対象 746 行（llvm 行判定基準）→ 126 行（83.1% 減、閾値 85% には約 14 行届かず）。テスト 62 件追加、`core-read-model-updater` 594 件成功、fmt / clippy / lint 成功。残りは I/O 失敗時のみの `?` Err 分岐と `#[cfg(test)]` 内の失敗メッセージ行。dead code 候補 6 箇所は [u2_cov_rmu-verification.md](coverage-logs/u2_cov_rmu-verification.md)。残り 3 担当は作業中。
- `u2_cov_cmd_ia` 着地: crate 81.9% → 97.1%、対象 977 行（crate 基準）→ 128 行（86.9% 減）。テスト 40 件追加（実 SQLite の `WouldBlock` / `Io` / `InvalidData` を再現）、339 件成功、fmt / clippy / lint 成功。dead code 候補は [u2_cov_cmd_ia-verification.md](coverage-logs/u2_cov_cmd_ia-verification.md) §4。残り 2 担当（app / domain+use-case）は作業中。
- `u2_cov_app` 着地: crate 92.5% → 96.1%、対象 1,195 行 → 638 行（46.6% 減、閾値 85% 未達）。テスト 90 件追加、889 件成功、fmt / clippy / lint 成功。残り 638 行のうち 362 行は一度も呼ばれない `map_err` 等の 1 行クロージャ（関数単位で数えられる種類）。**上流不一致（裁定要）**: 配布 scope ファイルの `keywords:` はブロック列だが本 build の frontmatter 読取はフロー列 `[a, b]` しか受理しない。詳細は [u2_cov_app-verification.md](coverage-logs/u2_cov_app-verification.md)。残り 1 担当（domain+use-case）は作業中。
- `u2_cov_domain_uc` 着地: 対象 1,066 行 → 297 行（72.1% 減、閾値 85% 未達）。テスト 104 件追加、両 crate 0 失敗、fmt / clippy / lint 成功。残りの 86 行はテストコード自身の `panic!` 腕。所見: `pipeline_link_error.rs:89-98` の `Display` 書式に doc コメント行が混入（裁定対象）。詳細は [u2_cov_domain_uc-verification.md](coverage-logs/u2_cov_domain_uc-verification.md)。
- 5 担当すべて着地。親が fmt / clippy / lint / workspace 全通し / `coverage.sh --base main` を実行中。

#### 5 担当着地後の再計測（2026-09-11）

| 検査 | 結果 | ログ |
| --- | --- | --- |
| `cargo fmt --all --check` / clippy `-D warnings` / `cargo lint` | 成功 | step9-logs/s9b-fmt.log、s9b-clippy-lint.log |
| `cargo test --workspace --no-fail-fast` | **3,514 件成功・0 失敗**（exit 0） | step9-logs/s9b-workspace-test.log |
| `scripts/coverage.sh --base main` | 絶対床 PASS、相対ゲート **FAIL**: head **98.18%**（94.77% から +3.41）vs base 99.15%。未カバー 4,532 → **1,608 行**、通すには 760 行以下（残り約 850 行） | step9-logs/s9b-coverage.log、s9b-coverage-head-per-file.json |

残りの内訳（未カバー行）: app 651、RMU 343、domain 245、command-IA 155、use-case 116、harness 54、infra 36、query 8。各担当の報告では、残りの多くが「一度も呼ばれない `map_err` 等の 1 行クロージャ」「I/O 失敗時のみの `?` Err 分岐」「テストコード自身の `panic!` 腕」「dead code 候補」である。

裁定待ち（[coverage-round2-questions.md](coverage-round2-questions.md)）: 配布 scope の `keywords:` ブロック列の不一致、`pipeline_link_error.rs` の `Display` 混入、dead code 候補の削除可否、カバレッジ 2 回目の進め方。

#### 裁定（2026-09-11）と 2 回目の委譲

利用者の裁定は Q1〜Q4 すべて A（[coverage-round2-questions.md](coverage-round2-questions.md)）。順序: まず `u2_keywords_display`（Q1 `keywords:` ブロック列の読取追加、Q2 `Display` 混入の修正）と `u2_dead_code`（Q3 到達不能コードの削除、`${PWD}` 分岐は本家と突き合わせ）を並行で着地させ、その後に app / RMU 中心のカバレッジ 2 回目（Q4）を委譲する（src の `#[cfg(test)]` 追加と削除が同じファイルで衝突しないよう直列にする）。
- `u2_dead_code` 着地: 候補 37 箇所のうち 13 箇所を削除（プロダクト到達不能行 約 71 行）、24 箇所は理由付きで残置。`${PWD}` 単独分岐は本家 `review-freeze-command.ts` の `normalizeShellTarget` と同じ帰結（宛先なし）で不一致ではなく、分岐を削除。削除前後で workspace 0 失敗（3,514 → 3,517）、clippy / lint 成功。追加の裁定候補 3 点（`ResumeMenu` 変種の廃止、`foreign approval` 検査の削除可否、`AuditFieldKey` 無謬コンストラクタ）は [dead-code-verification.md](dead-code-verification.md) 末尾。`u2_keywords_display` は作業中。
- `u2_keywords_display` 着地（親が仕上げ）: Q1 は本家 `listField` の受理範囲（ブロック列・フロー列・空）へ TDD で揃え、配布 12 ファイルのうちブロック列 8 ファイルが従来は空だったことを Red で確認。Q2 は `Display` の混入を修正。3 crate のテスト 2,229 件成功、親の fmt / clippy / lint 成功（[keywords-display-verification.md](keywords-display-verification.md)、`keywords-display-logs/`）。担当が背景待ちでターンを終える状態を繰り返したため、最終検査は親が実行した。
- カバレッジ 2 回目（Q4 = A）を `u2_cov2_app` / `u2_cov2_rmu` へ委譲。
- `u2_cov2_rmu` 着地: crate 92.7% → 96.0%、workspace 基準の見積り 343 → 約 182 行。テスト 46 件追加、641 件成功、fmt / clippy / lint 成功。裁定候補 3 点（同一監査シャードへの複数追記計画が `PublicationConflict` になる、DTO 拒否形の不揃い、`scan_range` の manifest 非選別）は [u2_cov2_rmu-verification.md](coverage-logs/u2_cov2_rmu-verification.md) §8。`u2_cov2_app` は作業中。
- `u2_cov2_app` 着地: crate 未カバー 633 → 521 行（96.2% → 96.9%）。テスト 40 件追加、928 件成功、fmt / clippy / lint 成功。目標 300 行は未達（残りの約 7 割は「隣の open は成功しこの 1 呼出しだけ失敗する」状況が要る 1 行クロージャと防御腕）。dead code 候補 12 群と上流突合候補（`source_baseline::read` の採取失敗の握り潰し）は [u2_cov2_app-verification.md](coverage-logs/u2_cov2_app-verification.md)。
- 2 回目着地後の親の再計測を実行中（fmt / clippy / lint / workspace 全通し / `coverage.sh --base main`）。

#### 2 回目着地後の再計測と、全通しの遅さの調査（2026-09-11 夜）

| 検査 | 結果 | ログ |
| --- | --- | --- |
| fmt / clippy `-D warnings` / `cargo lint` | 成功 | step9-logs/s9c-fmt.log、s9c-clippy-lint.log |
| `scripts/coverage.sh --base main` | 絶対床 PASS、相対ゲート **FAIL**: head **98.55%**（98.18 → +0.37）vs base 99.15%。残り約 520 行 | step9-logs/s9c-coverage.log |
| `cargo test --workspace --no-fail-fast`（単独） | 遅さの調査のため途中で停止（coverage.sh 内の計測実行が全通しを兼ね、そちらは 0 失敗で完走） | step9-logs/s9c-workspace-test.log |

**全通しが数時間かかる原因（実測）**: 契約テストが 39 MB の `aidlc` バイナリを試験ごとに一時ディレクトリへ `fs::copy` し子プロセスとして起動しており、macOS では新しい実行ファイルの初回起動ごとに OS の検査（dyld の同期通知待ち、`sample` で確認）で待たされる。ベンチ: コピー 10 回の起動 2 分 39 秒、シンボリックリンク 10 回 0.15 秒、ハードリンク 10 回 0.34 秒。これはテスト装置の非効率（利用者の指摘どおり不具合に近い）であり、プロダクトの問題ではない。

**是正**: `tests/support/tool_link.rs`（ハードリンク → シンボリックリンク → コピーの順で配置）を追加し、app crate の契約テスト 13 ファイル・20 箇所の `fs::copy` を置き換えた。ただし `upstream_271_contract.rs` はバイナリをワークスペース内 `bin/` に置き Source Baseline の走査対象になる（リンクだと採取が `unbindable` になり床の sha が立たない）ため、この 1 ファイルだけは実体コピーのまま（理由をコメントで明記）。是正後: `upstream_271_contract` 77 件成功、`cargo test -p aidlc` は `review_guards_contract::only_the_dispatched_reviewer_is_enforced_against` が高負荷下で 1 回失敗（単独再実行で成功。負荷依存の揺れとして F-X2 に記録）。単独スイートは数秒〜十数秒で走るが、crate 全体の直列実行は依然 27 分（CPU 20%）で、機械の負荷（同時ログイン 27）と OS の起動検査が支配的。

**カバレッジの現在地**: 2 回目までで 94.77 → 98.55%。担当報告では残りの大半が「隣の呼出しは成功しこの 1 呼出しだけ失敗する」状況が要る 1 行クロージャ・I/O 失敗時のみの分岐・テストコード自身の `panic!` 腕であり、テスト容易化 API を足さない制約では到達しにくい。進め方の裁定を利用者へ提示中。

#### 相対ゲートの廃止（2026-09-11、裁定 [coverage-gate-questions.md](coverage-gate-questions.md) Q1 = A）

利用者の「90% 以上なら問題なし」の方針を受け、相対ゲートを廃止して絶対床 90% のみにした。変更: `scripts/coverage.sh`（`--base` / `TOLERANCE` を撤去し `--base` は明示的に拒否）、`.github/workflows/ci.yml`（全イベントで `bash scripts/coverage.sh` のみ、`fetch-depth: 0` を撤去）、`scripts/governance/verify-ci-governance.sh`（`ci-coverage-base-condition` / `coverage-tolerance` を新方針へ。19 検査 PASS）。直近の正式計測 head 98.55% は絶対床を満たす（step9-logs/s9c-coverage.log）。`team.md` / `project.md` の該当規則（「相対ゲートを維持」）は直接編集せず、工程末の学習記録（§13）で改訂を提案する。

#### 最終確認（2026-09-12 未明、相対ゲート廃止・tool_link 是正後）

| 検査 | 結果 | ログ |
| --- | --- | --- |
| fmt / clippy `-D warnings` / `cargo lint` | 成功 | step9-logs/s9d-fmt-clippy-lint.log |
| `scripts/coverage.sh`（計測実行 = 全テスト） | **3,599 件成功・0 失敗**、head **98.55%**、絶対床 90% **PASS** | step9-logs/s9d-coverage.log |
| `verify-ci-governance.sh` | 19 検査 PASS | — |

Step 9 の検査はすべて成功。残りは最終成果物（source-manifest 再生成 / traceability 確認 / code-summary）→ 独立レビュー → Unit 完了。

#### 最終成果物と独立レビュー（2026-09-12）

- `source-manifest.json` を再生成（1,325 経路。`aidlc/` 配下は記録・規則の除外対象なので申告から外し、`coding-rules/README.md` の更新は code-summary に記す）。
- `traceability.json` は sensor PASS（gaps 0）、`code-summary.md` は required-sections PASS（H2 6 本）。
- 差し向け記録を書き、`REVIEW_REQUESTED`（iteration 1）を記録して独立レビュー（adversarial、最大 2 反復）を起票。レビュー中は成果物・manifest・申告ソースに書かない。

#### チェックポイントのコミットと PR（2026-09-12）

利用者の指示で未コミット 2,006 経路を 4 コミット（`feat(u2)` / `test(golden)` / `chore(harness)` / `docs(aidlc)`）に分けて `stage1` へコミットし、`origin/stage1` へ push、B1（U1 + U2）の PR を作成した: [#125](https://github.com/amadeus-dlc/amadeus-ng/pull/125)。独立レビューは進行中で、所見があれば追加コミットで対応する。監査シャードはフックが追記し続けるため、最後にまとめてコミットする。

#### CI の失敗と修正（2026-09-12）

`aidlc-distribution` ジョブが `capture-doctor.test.ts`「初回doctorは生出力を保存し作業記録を生成しない」で失敗。原因は Linux の bun が採取用 HOME の下 `~/.bun/install/cache` に `.pile` を書き、変更ファイルとして観測されたこと（macOS の置き場 `Library` / `.cache` しか除外していなかった）。`capture-observation.ts` で `.bun` も除外し、ローカルで bun テスト 47 件と `verify-corpus` を通して `b35eb913` として push。

#### 独立レビューの結果と再確認（2026-09-12）

- iteration 1: **READY**（Critical / Major なし）。Minor 3 件を提案として記録: R-01 計画 Step 9 のチェック未更新（Unit 完了時に親が更新）、R-02 相対ゲート廃止の memory 正本への未反映（§13 で改訂提案）、R-03 `traceability.json` の FR7（Deferred）target が FR1 と同一ファイル。レビュアーはターン上限で一度止まり、手元の根拠で判定を書くよう再開させて完了。
- CI 修正（申告ソース `capture-observation.ts`）を作業ツリーへ戻したため受領が失効。stale-receipt 回復レビュー（iteration 2、差分のみ）を起票中。
- レビュアーが計画ファイル末尾へ `## Review` を追記すると承認指紋が失効する（計画バイトに含まれる）。再発行したディレクティブの下で計画承認を取り直した（指紋 `sha256:608e6bc1…`、利用者 Approve Plan）。iteration 2 の追記後も同じ理由で失効しうるが、以後の作業は記録ディレクトリ内と framework ツールのみ。
