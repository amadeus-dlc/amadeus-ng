# 完了時の検証根拠に残る差

2026-09-09。固定本家 `a277af218f0df7f325d3b8be7b6d90fce2c5bd40` の `dist/claude/.claude/tools/aidlc-validity.ts` と `aidlc-state.ts` を確認した。実装完了の報告ではない。

## 観測された不足

`projection_golden_test` は開始時Source Baselineの是正後、18件中11成功・7失敗になった。うち通常承認とフェーズをまたぐ承認の2件には、`STAGE_COMPLETED` の `Validation Basis` 欠落がある。期待監査を変更したり、比較対象からこの欄を削除したりしていない。

## 本家の保存内容

- `aidlc-validity.ts:144` のグラフ指紋は、slug、phase、execution、condition、for_each、workspace_requires、consumes、produces、optional_produces、produces_kindsを対象にする。requires_stageや工程本文の指紋ではない。
- `:166` は各成果物をmissing、not-a-file、読取り成功、読取り不能に区別する。読取り不能のハッシュは本家のエラーメッセージ由来なので、RustのOS診断を無条件に代用できない。
- `:216` は必須成果物を不在でも数え、任意成果物は存在したものだけを数える。構造指紋と内容指紋を分離する。成果物・生成元およびinstanceの順序は本家のlocaleCompareによる。
- `:304` は入力の生成元をグラフ全体から一意に解決し、プロジェクト種別による条件を適用する。生成元のない任意入力は除く。必須入力の生成元が不明、または複数ある場合は採取失敗となる。
- `:367` のschema 3にはgraphContract、projectType、inputs、outputsを保存する。プロジェクト種別が不明ならnullを保存する。
- `:398` の採取失敗は完了操作そのものの拒否ではなく、`Validation Warning` を記録する。改行等を空白へ畳んだ本家診断を保持する。

成果物の実パスは `aidlc-artifact-resolution.ts:189` が解決する。通常成果物、spaceのcodekb成果物、Unit単位の成果物を区別し、今回のbugfixではUnitを作らない場合の工程直下パスも扱う。固定ハッシュを投影核に埋めるだけではこの契約を満たさない。

## 接続すべき境界

現在の `ReportRequest` と `ReportTransition::GateApproved` はこの観測を運ばない。必要な観測を報告時に採り、対象工程との対応を固定した保存事実として通常のReportedイベント・側ごとのDTO・RMUへ渡す必要がある。投影時に現在のファイルを読み直すと、障害回復や再構築で過去の監査が変わる。

独立した旧 `GateApproved` イベントを使う投影ゴールデンのフィクスチャも、本家と同じ初期ファイルを用意して観測を渡す必要がある。監査の期待文字列をそのままイベントへ埋めるテストでは採取経路を検証できない。

この調査後、Step 5のlink拒否から見つかった横断不足 `ERROR_LOGGED` の実装を先行している。本差分の実装はまだ着手していない。
