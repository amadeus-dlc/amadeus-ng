# pending-revision — U3 functional-design（ステージゲートで処理）

> レビュー iteration 1（NOT-READY: Critical 1 / Major 2）の所見 3 件は**本文に反映済み**（rules.md BR1.1 / BR1.2 / BR1.3 / BR2.3、entities.md EventStore /
> WorkflowExecutionRepositoryImpl、functional-spec.md §2 / §3.1 / §3.2 — 2026-08-23、コミット 49964cf / 966aaac）。レビュー予算 1 のため再レビューは不可で、
> entities.md 末尾の `## Review` は履歴として NOT-READY のまま残る（nfr-design レビュー所見 2）。

1. ゲートで: `entities.md` の `## Review` に「3 件とも本文で解消済み（iteration 1 後の是正、再レビューは予算外）」の注記を追記するか、Request Changes で
   レビュアーを再実行して iteration 2 の受領を得る（どちらかをオーナー裁定）。
2. BR4.2 の正規表現を `^[0-9]{6}-[a-z0-9]+(?:-[a-z0-9]+)*$`（連続ハイフン拒否 — kebab の家内規約、11 号 §2.2）に是正（委任 1 設計質問 1 — 実装は拒否側）。
