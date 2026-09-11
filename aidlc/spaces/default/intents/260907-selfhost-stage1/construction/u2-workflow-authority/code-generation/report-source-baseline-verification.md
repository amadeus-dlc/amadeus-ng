# 工程開始時のソース基準

## 実装

本家2.7.1の通常前進で、次の工程がworkspace_requiresの場合に採取するSource Baselineを接続した。固定配布グラフで該当するのはcode-generationである。採取済みのソース一覧と配布上の該当工程集合を報告要求へ渡し、集約が実際に開始する次工程だけを選ぶ。既存SourceBaseline型を使い、承認/読み飛ばし以外、no-op、最終工程の完了には次工程の基準を付けない。

報告時の一覧を単一Reportedイベントへ保存し、通常RMUがSTAGE_STARTEDの指紋と.source-reviewのスナップショットを同じ公開処理へ積む。スナップショットの配置は既存の`.aidlc-source-review/code-generation/`。再投影時に現在のファイルから一覧を再作成しない。読み側/書き側それぞれのSourceBaselineDtoで、未採取と採取不能（unbindable）を区別する。

## RedとGreen

- 実際の公開CLI結合で、domain-design承認後のcode-generation開始行にSource BaselineがないRedを確認し、イベント・投影接続後にGreen。
- 開始後に追加したsrc/lib.rsの実バイト・ファイルモードから得られる一覧と、監査指紋、保存スナップショットが一致。ソースを再変更して完了済み工程を報告し直しても、新スナップショットを作らず旧一覧を維持する。
- no-opに工程開始のソース基準を付けた不正なReportedを構築できるRedを確認。構築拒否を追加してGreen。テストの存在しないID採番API使用によるコンパイル失敗はRedに数えていない。
- 変更後のintent_lifecycle全123件が成功。全workspaceやCIの成功ではない。

証跡はtest-evidence/stage-baseline-*.log、baseline-noop-*.log、report-integration-final.log。保存領域が読めない/公開できない場合の共通処理は既存RMUを再利用し、今回の追加ソース基準に対する障害注入の最終確認は残る。

## 残る範囲

jump時のSource Baselineと無効化情報は別担当が実装中。旧GateApprovedを直接使う2.7.1投影ゴールデンの入力移行と、センサー監査3種類の扱いも別事項である。カバレッジ床・相対ゲート・release・全CI・実地スモークをこの局所検証で達成扱いしない。
