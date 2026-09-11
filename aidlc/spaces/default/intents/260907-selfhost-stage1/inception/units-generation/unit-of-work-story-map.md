# 要求と作業単位の対応

## 対応表

独立したUser Stories工程は省略されているため、架空のシナリオIDを作らず、要求書のFR1–FR8を直接対応付ける。`traceability.json`の`OK`は担当の割当が成立していることを示し、実装や受入検証が完了したという意味ではない。

| Requirement ID | 内容 | Primary Unit ID | Directory | 支援単位 | 最終的な検証 |
| --- | --- | --- | --- | --- | --- |
| FR1 | 本家2.7.1へ受入基準と実装を揃える | U2 | u2-workflow-authority | U1: 採取・比較基盤 | 対象実装とテストの移行を完了し、2.7.1の比較結果を確認 |
| FR2 | Claudeから開始・質問・承認・完了へ到達する | U2 | u2-workflow-authority | U4: 実配布接続 | CLI/フック境界の検証と、後続の実地スモーク |
| FR3 | 必要フックと承認受領をRustで成立させる | U2 | u2-workflow-authority | U4: 実発火の統合確認 | 正常・拒否・再入・古い受領の検証と実地確認 |
| FR4 | 更新処理と再利用する補助処理を分離する | U2 | u2-workflow-authority | U4: 読取り互換・副作用検証 | 正本更新の一元化と再利用操作の前後比較 |
| FR5 | bugfix/featureの配布資産を利用できる | U4 | u4-selfhost-integration | U3: 資産不足の診断 | 資産と参照先の検査、bugfixでの実利用 |
| FR6 | 切替に必要な自己診断を提供する | U3 | u3-selfhost-doctor | U4: 実環境での呼出し | 正常/異常の識別と、切替構成での診断成功 |
| FR7 | 本リポジトリの実地スモークを完了する | U4 | u4-selfhost-integration | U2・U3: 対象機能 | 準備完了後、検証・切替工程で実フックと人の承認による一周を実施 |
| FR8 | 検証済みの版をホストとして切り替える | U4 | u4-selfhost-integration | U3: 切替前診断 | 切替工程で対象コミット・安定タグ・参照先・復帰先を確認して実施 |

## 単位ごとの要求責任

- U1: FR1の採取・比較基盤を所有する。実装の適合完了はU2と全体検証まで追跡する。
- U2: FR1–FR4の対象実装と受入検証を所有する。
- U3: FR6の診断実装を所有し、FR5・FR8の確認を支える。
- U4: FR5の配布接続、FR7・FR8の実行準備を所有し、最終的な実地検証・切替へ引き継ぐ。

## 共通の品質要求

| 要求 | 対応する単位 | 確認すること |
| --- | --- | --- |
| NFR1 | U1–U4 | 本家2.7.1の対象観測契約を維持する |
| NFR2 | U1–U4 | TDD、90%床と相対ゲート、Quint/ITF、既存検証を維持する |
| NFR3 | U1–U4 | 対象コミットと起動イベントに対応するCI全ジョブを確認する |
| NFR4 | U1–U4 | 必須経路の差分に限定し、既存構造・配布資産・日本語の規律を守る |

## 網羅性と実装順の扱い

機能要求8件をすべて主担当に割り当て、4単位すべてに要求上の責任がある。依存関係は[依存一覧](unit-of-work-dependency.md)に定義する。

各単位内の具体的なコード変更・テストの順は、TDDの方針と次の契約設計に従って実装計画で定める。ここで経済的な優先順位やBolt順を決めない。U4の準備をFR7・FR8の実地達成へ読み替えない。

## 自動検証の是正

R-01の原因は、上流にシナリオ資料がない場合にFRを検証対象へ選ぶ一方、対応表の読取りがUSだけを抽出していたことだった。表の列順や、要求の割当内容に問題はなかった。

`scripts/aidlc-sync/patches/traceability-fr-unit-mapping.patch`で、シナリオ資料の有無に合わせてFRまたはUSを対応表から抽出するよう修正した。架空のシナリオIDは追加していない。`scripts/aidlc-traceability.test.ts`で、FRと下位要求の正常な対応、割当漏れ・誤った単位への対応の拒否、既存US対応の維持を検証する。

実際の本対応表に対する`aidlc-sensor-traceability.ts --output-path <このディレクトリ>/traceability.json`は、`pass: true`、`findings_count: 0`を返した。列順は原因ではなかったため元に戻している。

## Sources

- [要求書](../requirements-analysis/requirements.md): FR1–FR8、NFR1–NFR4
- [単位定義](unit-of-work.md)
- [確認済み分割方針](units-generation-questions.md)
