# 作業単位の依存関係

## 依存関係

以下の`depends_on`は、その単位が必要とする直接の成果を表す。優先順位、推奨する着手順、クリティカルパスを表すものではない。

```yaml
units:
  - name: u1-upstream-acceptance
    kind: packaging
    depends_on: []
  - name: u2-workflow-authority
    kind: service
    depends_on: [u1-upstream-acceptance]
  - name: u3-selfhost-doctor
    kind: service
    depends_on: [u2-workflow-authority]
  - name: u4-selfhost-integration
    kind: packaging
    depends_on: [u2-workflow-authority, u3-selfhost-doctor]
```

## 接続点と必要な成果

| 依存する単位 | 依存先 | 必要な成果 | 契約設計で固定する境界 |
| --- | --- | --- | --- |
| U2 | U1 | 本家2.7.1の採取結果・来歴・比較規則 | 入力、期待する終了/出力/状態/監査、非決定値の扱い |
| U3 | U2 | 実装されたCLI/フック入口と状態・監査の契約 | 診断項目、正常/異常の判定材料、読取りの副作用 |
| U4 | U2 | 正本を更新するRust操作と、その投影結果 | 配布手順からの呼出し、補助処理の読取り、実行許可の所有 |
| U4 | U3 | 自己診断の入口と結果 | 切替準備・スモークからの呼出しと成功/失敗の扱い |

U1は受入基盤、U2は対象実装の2.7.1適合を担当するため、U1の採取完了がU2の検証成功を意味しない。U4の準備が完了しても、実地スモークや切替は後続工程の実行証拠を必要とする。

## 並列性と統合

この依存関係では、依存成果を待たずに別単位を完成させられる並列集合を設けない。実装は利用者の指定どおり直列とし、同じ物理ファイルの変更は操作と役割ごとに責任を分ける。

単位をどのBoltへまとめてCIを成功させるかは実行計画で決める。古い期待値を残したまま移行全体を完了とすること、検査を弱めて単位を独立完了させることは認めない。

## 検証

単位名は各1回宣言し、依存先をすべて宣言済みの単位に限定する。自己依存と循環は許さない。機械可読の一覧は上のYAMLを正本とし、人間向けの表と一致させる。

## Sources

- [単位定義](unit-of-work.md)
- [確認済み分割方針](units-generation-questions.md)
- [要求書](../requirements-analysis/requirements.md)のFR1–FR8と変更対象表
