# functional-spec のレビュー履歴 — 2026-09-05（是正前の NOT-READY 判定）

> 2026-09-07 の再走（Modify）で functional-spec.md 末尾から原文のまま退避した。所見への対応は `../correction-report.md` を参照。
> 本ファイルは履歴であり、現行本文の承認・レビュー判定ではない。

## Review

**Verdict:** NOT-READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-09-05T07:04:22Z
**Iteration:** 1
**Request Challenge:** review:61b1c22701501c1e4c090dd54ec689a9

### Findings

旧レビューの番号 1〜20 を保持する。Resolved は旧所見そのものの解消を示し、後続裁定への同期完了を意味しない。新たな差異は R-21 以降へ分けた。過去の質問回答・pending-revision は履歴として読み、撤去された API や旧方式の再導入指示には用いていない。

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| 1 | Critical | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u2-domain-es-core/functional-design/rules.md > BR1.0 | 旧所見の park 中の位置変更拒否は規則に明記され、現行コードも guard_running_for で位置変更を拒否する。後続の再 park・自律切替の許可は別論点 R-24。 | 旧修正を維持し、明示的な例外は R-24 として同期する。 | Resolved |
| 2 | Major | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u2-domain-es-core/functional-design/rules.md > BR2.2 | 全ステージを文書順で扱うことが明記され、現行 Intent::create も stages_in_scope を同じ順序で使う。 | 追加対応なし。責務の移動は R-21。 | Resolved |
| 3 | Major | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u2-domain-es-core/functional-design/entities.md > c5_revision_proposal | 旧所見の残件だった Started.stages の変更宣言は追記され、C5 にも反映されている。後続のイベント設計は R-22。 | 追加対応なし。 | Resolved |
| 4 | Major | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u2-domain-es-core/functional-design/rules.md > BR1.6 | 現カーソルでは Pending を飛ばさず、介在位置では Pending も対象にする非対称は明記済み。現行モデルも維持する。後から追加された inScope 条件の未反映は R-24。 | 旧修正を維持する。 | Resolved |
| 5 | Major | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u2-domain-es-core/functional-design/rules.md > BR1.2 | active と in-flight の集合を分離済み。現行の状態射影・準拠テストも成功した。 | 追加対応なし。 | Resolved |
| 6 | Major | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u2-domain-es-core/functional-design/rules.md > BR4.2 | 削除対象を畳み込みの 2 メソッドに限定済み。現行 WorkflowDefinition には残す 6 述語があり、削除 2 メソッドの公開定義はない。 | 追加対応なし。 | Resolved |
| 7 | Major | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u2-domain-es-core/functional-design/rules.md > BR2.2 | None を SKIP に畳む規則が明記され、現行の a_missing_grid_cell_folds_to_skip_outside_initialization テストも成功した。 | 追加対応なし。 | Resolved |
| 8 | Minor | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u2-domain-es-core/functional-design/functional-spec.md > 第 2 節 stale_report | 旧設計内の戻り値は統一されたが、現在の stale_report は Result<(), CommandError>。NextDecision を返す宣言は現行 API と一致しない。 | 現行の戻り値と、完了応答を組み立てる境界を同期する。 | Unresolved |
| 9 | Minor | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u2-domain-es-core/functional-design/rules.md > BR4.1.logic | 正当な利用を除く検出式にはなったが、対象パス modules/core/domain は旧配置。現行の modules/core/command/domain/src/orchestration では定義・再輸出は 0 件だった。 | 検出先を現行の配置へ直し、パス不在を成功と扱わない条件を明記する。 | Unresolved |
| 10 | Minor | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u2-domain-es-core/functional-design/rules.md > BR4.1.statement | 「実測 10 ファイル」は旧 WorkflowExecution・旧クレート配置の一覧で、現在の移動対象一覧として使えない。元の移動は現行ファサードで確認できるが、現在の全呼出側を網羅したという証拠ではない。 | 10 ファイルを過去実績と明示し、次の変更では現行配置の影響範囲を別途確認する。 | Unresolved |
| 11 | Minor | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u2-domain-es-core/functional-design/rules.md > BR2.5 | 射影表は追加済みだが、現行 engine_loop.qnt v2.7 の初期化完了状態・受領証状態・受理条件を反映していない。ITF の現行 assert_projection / assert_signal との全対応表ではなくなっている。 | 現行モデル版と射影対象・モデル外の条件を明記する。古いモデルへ戻さない。 | Unresolved |
| 12 | Minor | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u2-domain-es-core/functional-design/rules.md > BR3.1 | 冒頭では定義 ID 検査を必須とする一方、末尾では定義引数を未使用と説明する。現行 next_decision は &Intent を受け NextDecision を返すため、予約引数という説明も古い。 | 未使用注記を撤去し、R-21 の現行境界に合わせて入力と検査を記述する。 | Unresolved |
| 13 | Minor | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u2-domain-es-core/functional-design/entities.md > relationships、および functional-spec.md > 第 6 節 | 旧所見の既存参照型の所有注記はなお不足する。さらに ER 図は WorkflowExecutionSnapshot を参照するが、YAML の見出しは WorkflowExecutionState で、そこも B12 以前の履歴とされている。派生図の参照が現行正本へ解決しない。 | 既存型の所有元を明記し、正本の同期後に ER 図と要約を同じ型集合から導出する。 | Unresolved |
| 14 | Critical | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u2-domain-es-core/functional-design/rules.md > BR1.3、および entities.md > StageEntry.phase | phase を保持し initialization 全体を非ゲートとする修正は反映済み。現行の initialization 3 段のテストも成功する。旧 Critical の索引 0 限定という原因は解消。誕生状態の後続変更は R-23。 | フェーズ判定を維持し、誕生状態は R-23 で同期する。 | Resolved |
| 15 | Major | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u2-domain-es-core/functional-design/functional-spec.md > 第 2 節・W2 | artifacts / phase_boundary の供給引数は旧指摘後に追記済み。現行 open_gate も artifacts を受ける。ただし phase_boundary は現在イベントへ載せない形に変わっており、その同期は R-22。 | 旧不足は閉じ、現行ペイロードを R-22 で整理する。 | Resolved |
| 16 | Major | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u2-domain-es-core/functional-design/entities.md > WorkflowExecution.revision_count | ステージごとの回数と加算規則を追加済み。現行 reject_gate_increments_the_revision_count テストも成功し、供給元不在という旧問題は解消。現行では apply が回数を導出する。 | 回数の存在を維持し、イベントへの搭載有無は R-22 で同期する。 | Resolved |
| 17 | Major | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u2-domain-es-core/functional-design/entities.md > c5_revision_proposal | Started.stages_in_scope から stages: list<StageEntry> への変更と phase 追加が宣言され、共有 C5 も stages を記載する。 | 追加対応なし。 | Resolved |
| 18 | Minor | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u2-domain-es-core/functional-design/functional-spec.md > 第 4.1 節 backward・redo | InProgress と redo 行は追加された。ただし backward 行は「target 以降」を Pending とし、BR1.6 / W5 の「target+1 以降を Pending、target は InProgress」となお一致しない。 | 状態表を target とその後続に分け、target を Pending と読める記述を訂正する。 | Unresolved |
| 19 | Minor | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u2-domain-es-core/functional-design/rules.md > BR2.2.logic | conditional の取得元を graph().nodes()[i].execution() と明記済み。現在は Intent::create が同じ組み立てを担う。 | 追加対応なし。所有の更新は R-21。 | Resolved |
| 20 | Minor | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u2-domain-es-core/functional-design/functional-spec.md > 第 5 節 StartError | initialization を除く旨と Empty の条件は明記済み。旧エラー条件説明の不足は解消している。開始時検査の所有移動は R-21。 | 追加対応なし。 | Resolved |
| R-21 | Major | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u2-domain-es-core/functional-design/entities.md > IntentId・WorkflowExecution・WorkflowDefinition、および functional-spec.md > 第 2 節・W1・W4 | 保存済み設計は dirName を IntentId とし、定義参照・依頼・静的計画を WorkflowExecution に保持する。現行裁定は Intent / IntentExecution を分離し、両 ID は別型、IntentId は UUIDv7。現行 intent.rs が計画解決と定義参照を所有し、IntentExecution::start は実行 ID と &Intent を受ける。WorkflowDefinition の「読取専用・ES 対象外」も現行 cqrs-boundaries / gateway-taxonomy の通常 ES Repository と一致しない。これは名称変更だけではなく、再構成と判断へ渡す情報の所有変更である。 | 旧記録を履歴としたうえで、現行の集約・ID・静的計画・実行状態の所有表と、開始／判断 API を同期する。pending-revision の kebab ID 案は再採用しない。 | New |
| R-22 | Major | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u2-domain-es-core/functional-design/rules.md > BR2.1・BR2.4・BR5.2・BR5.3、および functional-spec.md > W2・W3・第 5 節 | serde・本家 Aggregate/Event trait・memento・複合イベント ID・失敗を返す apply_event を現行設計として指示しているが、domain-persistence-neutrality / aggregate-commands はそれらを撤去済み。現行 Cargo.toml に serde / ESA の直接依存はなく、イベントは自前 UUIDv7 の id と aggregate_id を分離し、通番と時刻は apply の引数。replay / apply_event は Result を返さない。C3 の v3/B13 追記とも世代が違い、phase_boundary / revision_count 等のペイロードも現行実装と不一致。 | ドメインのイベント内容とアダプタの封筒・DTO を分離した現行契約へ同期し、復号失敗と壊れた履歴の扱いを区別する。再生方式は確定済みの「最新スナップショット＋それより後の差分」を維持する。共有契約の古い全再生指定を再導入しない。 | New |
| R-23 | Major | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u2-domain-es-core/functional-design/functional-spec.md > W1 手順 3・4・第 4 節、および rules.md > BR1.3 | W1 は初期化を InProgress で始め、birth が complete_stage を 3 回呼ぶ。一方、後続裁定と現行 engine_loop.qnt v2.2 以降は誕生時に初期化完了済みとし、現在のイベント enum に StageCompleted はなく、集約に complete_stage もない。現行 From<(Started, occurred_at)> は初期化を Completed、最初の実効対象を InProgress にする。設計の呼出順は現 API では実行不能で、旧 API を復活させると初期化完了の二重記録へ戻る。 | W1・状態表・イベント一覧から旧非ゲート完了経路を履歴へ移し、誕生時完了と対象実ステージがない縮退形の状態を現行の根拠に合わせて明記する。 | New |
| R-24 | Major | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u2-domain-es-core/functional-design/rules.md > BR1.0・BR1.3・BR1.6・BR1.7・BR1.8、および functional-spec.md > W5・W6・第 4.2 節 | コマンド受理条件が後続の互換裁定を反映していない。現行モデル v2.1 は forward の介在位置に inScope を要求、v2.3 は再 park を許可、v2.7 は park 中／Completed の自律切替を許可する。現行 approve_gate はレビュー・実践昇格の受領証ガードを持ち、switch_autonomy は人間の操作の確認を伴う。旧設計はそれらを省略し「park 中は unpark 以外拒否」「completed は終端」と一括規定するため、許可／拒否が現行と逆になる場面がある。 | 現行モデル版に対応したコマンド別ガード表を作り、位置変更と再スタンプ・権限変更・受領証記録を区別する。承認前提、forward の inScope 条件、モデル外の人間操作確認も明記する。 | New |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| aidlc-sensor-required-sections.ts（--stage functional-design、各 --output-path） | PASS: entities / rules / functional-spec、所見 0 | 追記前の H2 は 2 / 2 / 8。契約の世代差は検出しない。 |
| aidlc-sensor-upstream-coverage.ts（consumes 5 件、deliverables 3 件を明示） | PASS: unreferenced 0 | 共有入力への参照は揃う。内容の現行性は別途照合した。 |
| aidlc-sensor-traceability.ts | FAIL: missing_from_upstream_ids 32 件。他の gaps / orphans / missing_from_table / invalid_entries / invalid_targets は空 | 欠落一覧は共有 story-map 上の U2 担当外。FR8.3 / FR8.4 と、明示された横断の前提要求の BR 参照は解決する。ただし OK は現行実装への同期完了を証明しない。 |
| linter / type-check の適用判定 | 対象外・未実行 | 対象成果物に TS/JS/TSX の実行コードや該当スニペットはない。 |
| cargo test --locked -p core-command-domain --lib orchestration::intent_execution | PASS: 166 件、失敗・無視 0 | 初期化完了・差分再生・イベント ID・再 park・自律切替・受領証・ジャンプ等の関連テストを実行。検証結果は現行実装の証拠であり、旧設計への適合証明ではない。 |
| cargo test --locked -p core-command-domain --test engine_loop_conformance | PASS: 1 件、失敗・無視 0 | コミット済み全トレースの遷移と観測を現行集約へ再生。Quint ソルバーの新規実行・トレース再採取・全ワークスペース検証は今回行っていない。 |
| PlanAction 所有・公開面の静的検査 | PASS: 現行 orchestration 内の定義／再輸出 0 件 | workflow_definition/plan_action.rs とそのファサードに存在。WorkflowDefinition の残す 6 述語も実在する。旧 10 ファイル一覧の全件再監査とは区別する。 |
| 永続化依存・現行 API・モデルヘッダの照合 | 差異を確認 | domain Cargo.toml、Intent / IntentExecution / IntentExecutionEvent、engine_loop.qnt v2.7 を使用。R-21〜R-24 は実装失敗の報告ではなく、保存済み正本の同期不足である。 |
| ER 図と YAML の参照照合 | 不一致: WorkflowExecutionSnapshot / WorkflowExecutionState | 所見 13。Mermaid パーサ検査は未実行。 |

### Summary

旧 Critical 2 件は原因を解消済みだが、未解消の Major 4・Minor 7 が残るため ADVISORY 判定は NOT-READY。関連実測 167 テストは成功しており、今回確認したのは現行実装の不具合ではなく、後続裁定と実測に追従していない設計記録の問題である。過去の回答と改訂候補を保存しつつ、現行の実装指示となる正本を同期する必要がある。
