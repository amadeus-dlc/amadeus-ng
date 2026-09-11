# PendingIterationを導入し、FCCのプリミティブ要素をドメイン固有型へ移行・lintで防止する

## 問題と変更後の振る舞い

`PendingIterations` は判定待ちのレビュー回数を扱うドメインのFCC（ファーストクラスコレクション）だが、現在は `BTreeSet<u32>` を保持し、`FirstClassCollection::Item` も `u32` になっている。要素の意味を型で区別できず、呼出側にも生の数値が流れる。

`PendingIteration` を導入し、FCCが保持・公開・操作する要素をドメイン固有型へ統一する。同種の既存FCCも棚卸しして移行し、プリミティブ要素を検出する `cargo lint` のルールを追加する。

## 依頼と後続対応の位置付け

2026-09-08、利用者が「PendingIteration型を作るべき」「FCCが扱う要素はドメイン固有型。プリミティブ型は許されない」と指示し、coding-rulesへの記録、リンター追加、Issue化して後で対応することを明示的に許可した。

このIssueはstage-1実装中に判明した後続対応である。現在進行中のsetter/getter/完全コンストラクタの是正とは分け、FCC要素型の全体移行と新リンターをここで管理する。新規コードで違反を増やさない。

## 現コードの根拠

確認した基準コミットは `1dc727e00a26c27258d916cb3a5e0592c6c48c1c`。stage1作業ツリーには未コミットの変更があるため、着手時に現行コードを再確認する。

- [PendingIterations](https://github.com/amadeus-dlc/amadeus-ng/blob/1dc727e00a26c27258d916cb3a5e0592c6c48c1c/modules/core/command/domain/src/orchestration/pending_iterations.rs): `BTreeSet<u32>`、`Item<'a> = u32`。追加・削除・検索・参照・畳込み・絞込みも `u32` を扱う。
- [ReviewAttempt](https://github.com/amadeus-dlc/amadeus-ng/blob/1dc727e00a26c27258d916cb3a5e0592c6c48c1c/modules/core/command/domain/src/orchestration/review_attempt.rs): 復元時の `Vec<u32>` と `pending_iterations()` の戻り値を含め、永続化境界との対応を確認する。
- [ArtifactPaths](https://github.com/amadeus-dlc/amadeus-ng/blob/1dc727e00a26c27258d916cb3a5e0592c6c48c1c/modules/core/command/domain/src/orchestration/artifact_paths.rs) と [RuleLines](https://github.com/amadeus-dlc/amadeus-ng/blob/1dc727e00a26c27258d916cb3a5e0592c6c48c1c/modules/core/command/domain/src/workspace/rule_lines.rs): `Vec<String>` を保持し、`Item<'a> = &'a str`。同種の移行対象。
- [PromotedSections](https://github.com/amadeus-dlc/amadeus-ng/blob/1dc727e00a26c27258d916cb3a5e0592c6c48c1c/modules/core/command/domain/src/workspace/promoted_sections.rs): FCC本体の要素は `PromotedSection`。`headings() -> Collection<String>` のような派生コレクションも棚卸し対象とし、型名だけで適合と判定しない。

規則の正本は `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/first-class-collections.md`。汎用コンテナ、DTO、局所バッファと、ドメインのFCCを区別する。

## 実施内容

1. 全ドメインFCCの保持要素、関連型、操作の要素型、派生コレクションを一覧化する。単なる `Vec` の検索結果をすべてFCCと扱わない。
2. `PendingIteration` を導入し、`PendingIterations` と関連するレビュー操作へ適用する。許容範囲は現在のレビュー契約から実測し、推測で制約を増減しない。値の生成は完全コンストラクタを通す。
3. ほかのプリミティブ要素FCCも、用途に対応するドメイン固有型へ移行する。プリミティブ型のエイリアスへの改名だけでは完了にしない。
4. DTO・永続化との変換をアダプタへ置き、RMUは読取側DTOを使う。ユースケースからドメインgetterを呼ばず、Queryへドメイン判断を持ち込まない。
5. 既存 `tools/lint` に検出ルールを追加し、`cargo lint` に統合する。具体的なFCC、関連型、参照型、エイリアス、型を再エクスポートした経路をどこまで追跡するか明文化する。構文解析・マクロ展開等の限界も明記する。
6. 規則の機械強制欄と実装を整合させる。既存違反の移行と検出ルールを同じ変更で着地させ、allowで違反を残さない。

## 完了条件

- [ ] `PendingIteration` が導入され、`PendingIterations` の保持要素・Item・操作で生の `u32` を使わない。
- [ ] 全ドメインFCCの棚卸しと、確認した同種の違反の移行が完了している。
- [ ] 新リンターの検出例として `u32`、`String`、`&str`、プリミティブ型エイリアス等を用意し、実際にエラーになることを確認する。正当なドメイン要素型や汎用コンテナ・DTOを誤検出しないケースも固定する。
- [ ] 診断にルールID、場所、修正方法を含め、CLIの失敗終了も検証する。
- [ ] 昇順・重複排除・空集合・検索・追加/削除・filter/fold、保存・復元の振る舞いを維持する。保存形式と本家2.7.1の公開出力を変えない。
- [ ] TDDで実装し、`cargo lint`、整形、Clippy、関連テストおよび既存CIが成功する。

## このIssueで行わないこと

新しいレビュー仕様、コレクションの順序や重複の意味の変更、無関係なすべてのプリミティブ値のラップは行わない。ドメイン型を追加するためにユースケース/Queryへ業務判断を移さない。
