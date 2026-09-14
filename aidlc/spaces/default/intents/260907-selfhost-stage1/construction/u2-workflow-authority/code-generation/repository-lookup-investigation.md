# Repositoryのオブジェクト引数とテスト用内部状態の調査

調査日: 2026-09-14。対象は現在の作業ツリー。既存の未コミット変更を含む。実装・規則・テストコードは変更していない。

この文書は調査時点の記録である。その後、利用者が案Aを選択し、改修した。変更内容と検証結果は [repository-lookup-changes.md](repository-lookup-changes.md) を参照する。

## 結論

`find_for_*` の3契約は、他のドメインオブジェクトから参照先IDを読むためだけに、そのオブジェクト全体を要求している。検索条件を型付きIDで明示する方向が妥当である。ただし、参照先の主キーによる検索と、参照元のIDによる関連検索は別の契約であり、単純な改名では置き換えられない。

ご提示の `related_lookup_id` は本番の状態ではなく、ユースケース単体テスト専用フェイクの障害注入である。本番は既にSQLiteとmemoryのイベントストアに対応している。一方、単体テスト用フェイクは通常の保存・検索と障害注入を混在させ、定義を単一スロットで保持しており、整理が必要である。

## 検索契約の実態

パスはリポジトリルートからの相対パス。呼出数は `modules/core/command/use-case/src/orchestration/*_use_case.rs` 内の該当呼出式を数えた。

| 現在の契約 | 本番実装が実際に使用する値 | 呼出数 |
|---|---|---:|
| `WorkflowDefinitionRepository::find_for_intent(&Intent)` | `intent.definition_id()` | 6 |
| `IntentRepository::find_for_execution(&IntentExecution)` | `execution.intent_id()` | 14 |
| `IntentExecutionRepository::find_for_approval_origin(&PlanApprovalOrigin)` | `origin.execution_id()` | 1 |

合計21か所。最後の引数は集約ではなく値オブジェクトだが、IDだけで足りる点は同じである。

根拠:

- `modules/core/command/interface-adapter/src/orchestration/workflow_definition_repository_impl.rs:328`
- `modules/core/command/interface-adapter/src/orchestration/intent_repository_impl.rs:254`
- `modules/core/command/interface-adapter/src/orchestration/intent_execution_repository_impl.rs:337`

いずれも内部で既存の `find_by_id` に委譲する。検索対象の選別や業務判断はしていない。

## 複雑化を誘導している規則

`aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/tell-dont-ask.md:12` は、ユースケースからのgetter呼出しを、IDを別のメソッドへ渡すだけの場合も含めて禁止している。同ファイル36行と `aggregate-references.md:47` は、その対応として `find_for_execution` / `find_for_intent` を明示している。

`tools/lint/src/domain_getter/tests.rs:29` にも、ドメインの `id()` を検出するテストがある。従って、呼出側を `repository.find_by_id(aggregate.related_id())` に直す案は、現行規則とlintの変更を伴う。現在の指摘を反映する際、この衝突を隠して実装だけ変更してはいけない。

## テスト専用フェイクの問題

`modules/core/command/use-case/src/orchestration/mod.rs:66` の `#[cfg(test)]` で `test_support` を限定している。

`test_support.rs:243` の `InMemoryWorkflowDefinitionRepository` は次を一体化している。

- 通常の保存状態: `stored: Option<(WorkflowDefinition, usize)>`
- 保存されたイベントの観測: `committed`
- 検索回数の観測: `lookups`
- 障害注入: `corrupt`、`related_lookup_id`、`interrupting_writes`

`related_lookup_id` は関連索引ではない。全検索を指定IDへ差し替え、誤った定義を返すための設定である。利用箇所は `commit_verdict_use_case.rs:515` と `record_review_use_case.rs:284` の2テストで、別定義の拒否、原因エラーの伝播、保存されないことを確認する。

通常フェイクにこの分岐を載せる必然性はない。必要な異常応答は専用スタブで表せる。取り違え拒否自体は `modules/core/command/domain/src/orchestration/intent.rs:88` にあり、同ファイルの既存テストでも検証している。ユースケースのエラー伝播・書込み抑止を確認する価値は残るので、テストを無条件に削除する案にはしない。

さらに `test_support.rs:363` は保存先IDを区別せず、唯一のスロットの版を比較し、379行で置き換える。静的に読める帰結として、Aを保存した後に未保存のBを作成すると、Aの版との比較で競合になる。本番の複数定義を保持する契約と一致しない。この挙動の新規再現テストは今回追加していない。

通常フェイクはIDをキーとする `HashMap` と、契約上必要なIDごとの楽観ロック情報を持つ構成が適する。障害注入や観測は、それを必要とするテスト用の別型に分離する。楽観ロックの契約があるため、版の情報まで削除するのは不適切である。

## IDによる検索へ移行する案

### A. 参照先IDを既存のfind_by_idへ渡す

リポジトリの単純化を優先する場合の推奨案。

```rust
let intent = self.intents.find_by_id(execution.intent_id()).await?;
let definition = self.definitions.find_by_id(intent.definition_id()).await?;
```

新しい検索ポート、関連索引、追加の関連読取りは不要。業務上の判定は引き続き集約へ委譲する。ただし「取得先IDをリポジトリへ渡す用途」をgetter禁止の対象から外す規則変更と、その範囲を検証するlint変更が必要になる。これは今回の調査からの提案であり、規則はまだ変更していない。

### B. 参照元IDで検索する正式な関連検索を用意する

`find_by_intent_id(&IntentId)` や `find_by_execution_id(&IntentExecutionId)` を用意する案。インターフェースとして検索条件が明確になる。

ただし現在の関連は `Intent.definition_id` と `IntentExecution.intent_id` にあり、検索先集約の属性ではない。定義の `HashMap<WorkflowDefinitionId, WorkflowDefinition>` だけではIntentIdから検索できない。永続化側で正本の関連を読むか、関連索引を同期して保持する設計が必要になる。イベントソーシングとコマンド側の読取規律を守り、遅延するクエリ側のリードモデルへ依存しないこと。

この案では関連元の不在・参照先の不在・破損の扱いも定義する必要がある。現契約は参照先IDを伴う `RepositoryError` なので、関連元が無い場合の表現は自動的には決まらない。また `intent.id()` も現規則では禁止されるため、呼出側が検索キーをどこから得るかまで決める必要がある。

## 改修時の検証対象

- 複数IDを同じストアへ保存しても干渉しないこと。楽観ロックもID単位であること。
- 対象不在と別IDの混同を防ぎ、定義の最新改訂を取得すること。
- 不一致や読取失敗で書き込まず、原因エラーが伝播すること。
- SQLiteと本家memoryバックエンドに同じ契約検査を課すこと。
- Aの場合は、ID受渡しの許容と業務判断の流出禁止の両方をlintで確認すること。

## 今回の実行結果

- `cargo test -p core-command-interface-adapter --test workflow_definition_repository_contract --test intent_repository_contract`: 定義18件、Intent14件が成功。両方ともSQLiteと本家memoryを同じ契約で検査した。
- `cargo test -p core-command-use-case --lib an_unrelated_definition_is_reported_with_its_cause_before_any_write`: 障害注入を使う既存2件が成功。

合計34件が成功した。これは現行の実装・テストの動作確認であり、単体テスト用の単一スロットフェイクが本番と同じ複数ID契約を満たす証明ではない。ワークスペース全体のテストやlintは今回実行していない。
