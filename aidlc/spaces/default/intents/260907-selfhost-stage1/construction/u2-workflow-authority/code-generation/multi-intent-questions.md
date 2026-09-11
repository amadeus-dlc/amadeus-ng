# 複数intentの投影範囲の確認

## Q1

同一スペースで2件目以降のintentも扱えるよう、投影対象の選択とチェックポイントの識別を今回のB1で是正するか。

現在の保存先は `runtime.rs::store_path` の `StorePath::for_space` でスペース単位。RMUの `read_model_updater.rs::resolve_plan` はStartedのintent_idが2種類以上あれば `CatchUpError::MixedIntents` を返す。単一intentの履歴を別intentの計画で描かないという安全条件は維持し、外側の適切な振分けが必要になる。

本家のrecord名はYYMMDD-label（slugify上限24）、重複時は-2〜-999で、新しい作成だけを合わせる。既存記録の移動は不要。命名を合わせることと、複数intentの投影を可能にすることを区別する。

- A. B1に含める — 同一スペースで次の作業を始められるよう、対象別の投影と履歴の取り違え防止を修正・検証する。（推奨）
- B. 今回は見送る — 単一intentの実地完走を先に確認し、2件目以降の制約を残した切替の可否は別途判断する。
- X. Other (please specify)

[Answer]: A. B1に含める

複数intentの対応方法はこの回答前に実装しない。命名の純粋な契約、単一intentの開始出力、その他の承認済み作業は独立して進める。要求・完了条件を無断で書き換えず、制約を残す場合も継続利用が検証済みとは表示しない。
