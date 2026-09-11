# HookHealth・ArtifactAuditの保存backend統一

## 対象と結果

承認済みU2計画Step 6のうち、`HookHealthRepositoryImpl` と `ArtifactAuditRepositoryImpl` がSQLite固定だった不足を修正した。両方を `Impl<S>` とし、SQLiteと本家 `EventStoreForMemory` が**同じ保存・再構成実装**を通る。DomainのHookHealth・ArtifactAudit、ContinuationRepository、アプリ呼出しはこの作業では変更していない。Step 6・U2全体の完了は宣言しない。

根拠は `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/use-case-rules.md` §2、`gateway-taxonomy.md` §5、`factory-naming.md`、`domain-persistence-neutrality.md`、`abstract-data-type.md`。[承認済み計画](code-generation-plan.md)と[テスト手順](unit-test-instructions.md)のTDD・品質基準を維持する。

## 正典との対応

実装前に `IntentRepositoryImpl<S>`、`IntentAggregateKeyDto`、既存2Repositoryと対応するDTO・契約テストを読んだ。設計スキルのRepository配置・Domain Primitive・DDD構成要素は同セッションの先行作業で読了済みで、重複読込はしていない。

| 規則 | 今回の実装 |
| --- | --- |
| 1 trait 1 Impl | 各集約のRepositoryは1種類。別MemoryRepository・独自HashMap・別storeを作らない |
| 本家MemoryとSQLiteの同じ処理 | `HookHealthMemoryStore` / `HookHealthSqliteStore`、`ArtifactAuditMemoryStore` / `ArtifactAuditSqliteStore` は本家型のalias。`impl<S: EventStore<...>>` が保存・再構成を所有 |
| 完全コンストラクタ | private `new(store, location)` が唯一のRepository構築子。`open` / `in_memory` / `reopened` はすべて委譲。setterなし |
| 揮発ストアにダミーpathを作らない | `location: Option<StorePath>`。memoryはNone、SQLiteは実際のpath |
| 再オープン | 正典と同じ、本家ストアのCloneで共有先へ別のRepositoryを開く `reopened()`。手書きの共有ロックを追加しない |
| 1ファイル1公開型 | 公開Store aliasの型引数となるKey・集約DTO・EventDTOを1型1ファイルへ分離。フィールドはprivate |
| 永続化中立 | DTOとストアキーはinterface-adapter所有。Domainへserde・SQL・ストア型を追加しない |
| 既存保存形式 | DTOのフィールド順、manifest、ストアキーのtype_nameとvalueを維持。既存SQLite行の移行は不要 |
| 既存ドメイン再生 | ArtifactAuditの親担当によるreplay/apply是正を保持。再構成でイベント生成や保存をしない |

HookHealthEventDtoは分離に伴い、`of`のイベント別材料をまとめ、構造体リテラルを1箇所にした。保持フィールドやワイヤ形式は変えていない。アプリの `Impl::open` は既存の型推論で通り、型引当や呼出しの変更は不要だった。

## 実行で判明した不足と修正

1. **競合の実在versionが0固定だった。** 両Repositoryの共通契約は、初回保存後の再作成で `expected=0, actual=1`、古い版での更新で `expected=1, actual=2` を要求する。SQLiteで実行Redを確認後、正典と同じく、失敗診断の材料として実在snapshotのversionを読む形へ修正した。判定や書込にこの再読込を使わない。
2. **別集約の有効snapshotを要求IDの結果として返していた。** 本家Memoryへ、キーAと単体では有効な集約Bのsnapshotを保存すると、両RepositoryがBを返すRedになった。Repositoryが復号後の集約IDと要求IDを照合し、Corruptで拒否する。Domainを変更していない。
3. ArtifactAuditの内部エラー理由に残っていた `hook health` の誤表記を `artifact audit` へ修正した。公開監査文言ではなく、当該Repositoryが作る破損理由の記述である。

## TDDと検証の証跡

Memory入口がない段階では共通契約を実行できないため、`in_memory`未定義の**コンパイル失敗は動作Redに数えていない**。[API不在の確認](hook-repository-backends-logs/memory-api-unavailable.log)として区別する。共通契約関数を先に作りSQLiteで動作Redを確認し、Memory用の同じ呼出しを用意してから実装した。

| 検査 | 結果・証跡 |
| --- | --- |
| ArtifactAudit競合 | [Red](hook-repository-backends-logs/artifact-conflict-red.log) → [Green](hook-repository-backends-logs/common-contract-green.log) |
| HookHealth競合 | [Red](hook-repository-backends-logs/health-conflict-red.log) → 同Green |
| snapshotの要求ID不一致 | 本家Memoryの両Repositoryで [Red](hook-repository-backends-logs/snapshot-identity-red.log) → [Green](hook-repository-backends-logs/history-contract-green.log) |
| 公開Repository契約 | 2集約それぞれMemory/SQLiteが同じgeneric契約を実行。不在、初回保存、読戻し、重複、更新、古い版、別集約イベント拒否、拒否後の無変更。既存SQLite検査も保持し、計6件成功 |
| 両backendの履歴契約 | それぞれsnapshot後の差分イベント再生、reopenedからの観測、履歴件数の不変、foreign manifest、不正DTO、foreign snapshotを検証。4件成功 |
| memoryのエラー材料 | 読取・書込の失敗にダミーpathがないことを2件で確認 |
| Repository実装回帰 | `repository_impl::tests::` の47件成功。このうち今回の新規テストは上記6件であり、47件全部を新規検証とは数えない |
| アプリ結線と共有ストア | [plan_runtime_contract 5件成功](hook-repository-backends-logs/app-regression.log)。HookHealth streamとPlan streamの混在・投影/復旧が維持される |
| 静的検査 | [限定clippy成功](hook-repository-backends-logs/clippy.log)、`-D warnings`。fmt・git diff --checkも成功 |
| tools/lint | generic化、履歴拒否、DTO構築整理、テスト修正、最後の区切りで `cargo lint` を実行し、いずれも終了0を確認 |

テスト作成途中に、共通のエラー分類器がSQLite以外をOtherへ写す既存契約に対し、`std::io::Error`からPermissionDeniedを期待する誤ったfixtureがあった。実装を変更して合わせず、既存分類器が実際に扱う `rusqlite::Error::SqliteFailure(PermissionDenied)` を投入するfixtureへ修正した。これは製品の動作Redとは区別する。期待するPermissionDenied・path=Noneの判定は維持した。

全target clippyの途中で別担当のContinuation constructorのconst不足を検出したため当該担当へ連絡し、こちらではそのファイルを変更していない。最終の限定clippyは成功した。

## 変更ファイル

`modules/core/command/interface-adapter/` 配下のみ（本検証記録を除く）。

- `src/orchestration/hook_health_repository_impl.rs`
- `src/orchestration/artifact_audit_repository_impl.rs`
- `src/orchestration/hook_health_repository_impl_tests.rs`
- `src/orchestration/artifact_audit_repository_impl_tests.rs`
- `src/orchestration/dto/hook_health_dto.rs`
- `src/orchestration/dto/hook_health_event_dto.rs`
- `src/orchestration/dto/hook_health_aggregate_key_dto.rs`
- `src/orchestration/dto/artifact_audit_dto.rs`
- `src/orchestration/dto/artifact_audit_event_dto.rs`
- `src/orchestration/dto/artifact_audit_aggregate_key_dto.rs`
- `src/orchestration/dto/mod.rs` と `src/orchestration/mod.rs` の上記型の公開宣言
- `tests/hook_health_repository_contract.rs`
- `tests/artifact_audit_repository_contract.rs`

共有ファイルの他担当の変更を保持した。工程next/report/pause、commit/push、外部投稿は行っていない。

## 限界と次の統合

本家Memoryの型付きイベントAPIでは、不正JSONバイトやSQLiteの型・schema破損を同条件で注入することはできない。今回の共通破損検査は**同じEventStore APIで実際に保存できる**manifest・DTO値・snapshot識別の矛盾を対象にする。SQLite固有の破損全種やプロセス障害・全フック挙動の完全検査とは主張しない。

workspaceカバレッジ90%床、相対0.01、全CI・U2全体の独立レビューは親担当の統合検証へ残る。基準値は下げていない。

推奨は下記を独立実行してStep 6の現計画へ統合すること。新たなbackend固有の故障範囲を増やす場合は、その注入方法と契約を分けて定義する。

```sh
cargo test -p core-command-interface-adapter --test hook_health_repository_contract --test artifact_audit_repository_contract
cargo test -p core-command-interface-adapter --lib hook_health_repository_impl::tests
cargo test -p core-command-interface-adapter --lib artifact_audit_repository_impl::tests
cargo test -p aidlc --test plan_runtime_contract
cargo clippy -p core-command-interface-adapter --lib --test hook_health_repository_contract --test artifact_audit_repository_contract -- -D warnings
cargo lint
```
