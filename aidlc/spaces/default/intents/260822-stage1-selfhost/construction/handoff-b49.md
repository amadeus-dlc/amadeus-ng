# ハンドオフ — b49: practices-discovery の受領証（`PRACTICES_AFFIRMED`）を `IntentExecution` のイベントで、`aidlc-state practices-promote` 動詞、approve の段 12 ガード（2026-09-05）

対象: #7 キュー 5 の残り（段 12 — b48 の裁定で分割。**これでキュー 5 は完了**）。設計書: [`b49-practices-receipt/design.md`](b49-practices-receipt/design.md)。
前段: [`handoff-b48.md`](handoff-b48.md)。オーナー裁定（着手前に質問、設計 §0）: メモリ層 2 本への書込は **RMU が投影する**（A）、
失敗時の `PRACTICES_OVERRIDE` は**描かない**（A）、兄弟動詞 `practices-event` は**射程外**（A）。

用語（初見向け）: **昇格** = Practices Discovery で人間が承認した「チームの決めごと」のドラフト 2 本（`team-practices.md` / `discovered-rules.md`）を、
メモリ層の正本 `team.md`（5 節を置換）と `project.md`（`## Mandated` / `## Forbidden` に `(affirmed 日付)` 印の規則を追記）へ書き写すこと。
**受領証** = 昇格が成功した事実（監査行 `PRACTICES_AFFIRMED` と状態ファイルの `Practices Affirmed Timestamp`）。**段 12** = 「practices-discovery の承認は
現在の試行（直近の開始・差し戻し・ジャンプより後）の受領証を要する」という report のガード。

## やったこと
- **ドメイン**: `IntentExecutionEvent` を 16 変種へ（`PracticesAffirmed { stage, affirming_user, sections, mandated, forbidden }` — 昇格の内容そのものを運ぶので、
  投影はドラフトを読み直さずに描ける）。`IntentExecution` に `practices_affirmed: Vec<bool>`（計画長）、クエリ `practices_stage()`（slug リテラル
  `PRACTICES_DISCOVERY_SLUG` の位置）、コマンド `affirm_practices`（取り違え・計画に無い、の 2 ガードだけ。本流の状態は見ない。再昇格は上書き）、
  `approve_gate` の段 12 `require_practices_receipt`（checkbox 前提の後・段 11 レビュー受領証の前 — upstream と同じ位置）。フロアはレビュー会計と共用
  （前進 / 読み飛ばしで立った次ステージ・`GateRejected`・`Jumped` 全ステージ。`StageRevised` は区切らない — upstream で `STAGE_REVISING` を出すのは
  `reject` の対と approve の backstop だけで、我々の `GateRejected` がその対）。
  `workspace` に純関数 3 本（`extract_section` / `replace_section` / `append_under_heading` — upstream `aidlc-lib.ts` の写し）と
  値オブジェクト `PracticesPromotion::plan(ドラフト 2 本, 正本 2 本, 日付)`（5 節の選択・規則行の trim / 印付け / 重複除去・見出し不在の拒否）。
- **DTO**: 1 変種 + スナップショット `practices_affirmed: [bool]`（欄不在は全 false）。ワイヤ形式 16 変種を両側のゴールデンで固定。
- **ユースケース**: `PromotePracticesUseCase<E, I>`（find → find intent → コマンド → store、`Conflict` 1 回再試行、定義は引かない、成功は `Ok(())`）。
- **RMU**: `ReadModel` に**メモリ面** `MemoryFaces { team, project, dirty }`。`PracticesAffirmed` は 4 面を描く（team.md の節置換 → project.md の印付き行追記
  （trim 一致で重複除去）→ 状態ファイルの `Practices Affirmed Timestamp` / `Last Updated` → 監査行 `PRACTICES_AFFIRMED` の 4 欄）。`ProjectionTargets` は
  memory ディレクトリを受け取り、`catch_up` は 2 本とも在るときだけ載せ、**dirty のときだけ** project.md → team.md → 状態ファイル → 監査シャードの順で書く
  （触っていないキャッチアップは mtime を動かさない）。`state_writers::with_field_or_insert` をドメインの `append_under_heading` へ寄せた（挿入位置が
  upstream / ゴールデン `cli/park/park/state.diff` どおり「次の `## ` 見出しの直前」に是正された）。
- **クエリ側**: `read_definition_stage` を slug で 1 引当する `DefinitionStageDao` / `DefinitionStageView`（`stage_slug` / `support_agents` の 2 列）/ 実装 / ダブル /
  `FindDefinitionStageUseCase`（行が引けないこと自体が「グラフに無い」の答え）。
- **app**: 新しい面 **`aidlc-state`**（`Face::State`）。`practices-promote` の構文段は upstream `handlePracticesPromote` の順序と逐語（usage → `--target-dir`
  未配線 → 実行カーソル → Step 1 ensemble 証跡（定義の行・ドラフト dir・contributions の identity marker）→ Step 2 / 3 読取 → Step 4 計算 → 記録 →
  `catch_up` → stdout JSON 1 行 `{"emitted":"PRACTICES_AFFIRMED","sections_written":[…],"mandated_appended":n,"forbidden_appended":n,"affirmed_at":"…","team_md":"…","project_guardrails":"…"}`）。
  失敗はすべて stderr + exit 1。他の 24 動詞は not-wired 拒否、未知は upstream `:630` の逐語。`report` の段 12 拒否は **orchestrate 自身の `error` directive**
  （`Cannot approve "practices-discovery" before practices-promote succeeds. …`）。
- **Quint v2.6**: `practicesStage`（静的 nondet、-1 許容）/ `affirmed`、`actPromotePractices`、段 12 ガード、フロア、不変条件 3 本（19 本へ）+ witness 2 本（9 本へ）。
  mutation 3/3。ITF 14 本（13 本再採取 + `trace-0x808` = `not(w_approved_practices)`、採取コマンドは設計 §9）。
- **仕様**: `docs/specs/10-orchestration.md` B10 行・§3・§6 I19・§9 v2.6・§10 実装ノート、`docs/specs/11-workspace.md`（メモリ層 2 本が投影面に）、`docs/specs/deviations.md` #6。
- **テスト**: 新規 93 本（1,974 → 2,067）。全ゲート緑（Quint 25 ステップ PASS、カバレッジ 99.13%）。

- **設計との差分 9 点**（設計 §9）: `unit` を含む 24 動詞、ゴールデン `report/approved` は非影響（合成グラフの slug が `domain-design`）、`with_field_or_insert` の
  挿入位置の是正、`ProjectionTargets::new(…, memory_dir)`、`MemoryFileWrite` の材料、クエリ側 1 表の新設、`directive_drawing::strings` の共有、
  `commit_refusal` の腕順、`MemoryFileRead` の観測に参照入力の分離。

## 積み残し（記録のみ、起票しない）
- **繰延（deviations #6）**: 失敗時の `PRACTICES_OVERRIDE` 非描画、`--target-dir` / `practices-event` / 他 23 動詞の not-wired、approve の revision backstop
  （`unrecordedRevisionSinceGateOpen` — 成果物フック由来、CP5）、`Cannot resolve the active intent for practices promotion.` の own wording。
- **到達しにくい防御枝**: `PromotePracticesUseCase` の「`store` が `Conflict` 以外で失敗」腕（`RecordReviewUseCase` と同じ既存の穴 — インメモリのポートに
  非競合の書込失敗を作る口が無い）。`practices_support_agents` の `definition_id` 解決失敗・`cannot open the event store` は構造的に到達不能。
- 段 11 の completion-evidence は slice 2、turn-shape marker は CP5（変わらず）。

## 次
#7 キュー 6 以降（#72 set-autonomy → #71 WorkspaceScanner → #77 → #82 → #53 → クリティカルパス 4〜6）。
