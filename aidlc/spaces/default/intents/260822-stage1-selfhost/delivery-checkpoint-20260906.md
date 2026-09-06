# コレクション実装の完了とCI・品質管理の引継ぎ

## この記録の位置付け

2026-09-06、コレクション実装の公開済み範囲と、U10に残る文書整合を分けて記録する。
基準はmainの `53b5667e52ed7d28a395458afb3fe254911b1b45`。
この記録は作業の引継ぎであり、U10の要件確認・レビュー・ステージ完了を代行しない。

## 完了した作業

- [PR #113](https://github.com/amadeus-dlc/amadeus-ng/pull/113): 承認制御の修正とゴールデンコーパス検証の補完。
- [PR #114](https://github.com/amadeus-dlc/amadeus-ng/pull/114): `FirstClassCollection`、空を許す`Collection<T>`、非空の`NonEmptyCollection<T>`を実装し、既存7型と利用側へ展開。
- [PR #115](https://github.com/amadeus-dlc/amadeus-ng/pull/115): `map(String::as_str)`のように要素を借用する変換を一般型・非空型の双方でサポート。

3件のマージ状態はGitHubから再取得し、[取得結果](delivery-checkpoint-20260906.json)に保存した。
共通traitの契約は`len`・`is_empty`・`at`・`fold_left`・`filter`。`map`・`combine`・`divide`は具体型の制約に合わせる。
`NonEmptyCollection`の`filter`・`divide`は空になり得るため`Collection`を返す。空専用型は設けていない。
横展開の対象・対象外理由は[対象一覧](collection-rollout-inventory.md)を参照する。

## 元の作業ツリーに残っていた差分

`chore/aidlc-fork-sync`の変更・未追跡ファイルを基準mainとバイト比較した。

- 24ファイルはmainと一致。コレクション実装・テスト・展開記録は公開済み。
- コレクション規約は、mainに追加済みの例外方針が元の作業ツリーには未反映。mainの版を採用する。
- 監査ログは、mainの内容を接頭辞として保持したまま後続イベントが増えていた。接頭辞一致を検証し、後続の会話・セッション記録を保存する。

元の作業ツリーのGit管理領域はこの実行環境から書き込めないため、その差分は削除・リセットしていない。
本変更は別の書込可能な作業用cloneで最新mainを基点に作成した。元の作業ツリーが同期済みとは扱わない。

その後、元ブランチ`chore/aidlc-fork-sync`の未コミット26ファイルも、同じブランチ名へ2コミットで保存・pushした。
`a4c61c74`がコレクション実装・テスト・文書25ファイル、`302d51af`が監査ログである。
push後、26ファイルすべてが元の作業ファイルとバイト一致することを確認した。
本変更にはその監査ログの後続分も含める。元worktreeのHEAD・indexは書込制限により未同期であり、公開済み内容とdirty表示を区別する。

## 残る作業

U10は`nfr-requirements`の要約確認待ちで停止中。今回の出荷をもって承認済みにはしない。

1. [最新の改訂基準](construction/u10-ci-governance/revision-baseline-20260906.md)を基に、要約確認を完了する。
2. 要件3成果物を改訂し、独立レビューを実施する。
3. 後続の設計・実装記録を現行CIへ整合させ、対応する検証・承認を完了する。

## 今回の検証

- `bash scripts/governance/verify-ci-governance.sh --with-ruleset`: 20項目成功、失敗0。現在のGitHub rulesetも検査した。
- コレクション実装は再変更していない。過去の検証は[実装記録](first-class-collections-20260906.md)に記載されている。
- 今回カバレッジの再現性を2回測定したわけではない。許容差0.01ポイントの設定確認と、同一リビジョンを反復測定する受入確認は別である。
