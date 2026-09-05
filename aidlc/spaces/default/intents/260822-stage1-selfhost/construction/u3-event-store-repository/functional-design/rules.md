# rules — U3 イベントストアと Repository

> 2026-09-05 是正。下の YAML が正本、要約表は同じ規則を示す。
> 出典: `../../../inception/units-generation/unit-of-work.md`、
> `../../../inception/units-generation/unit-of-work-story-map.md`、
> `../../../inception/requirements-analysis/requirements.md`、
> `../../../inception/contract-design/contract-summary.md`（C3 / C6）、
> `../../../inception/domain-design/decisions.md`（ADR-001 / ADR-007 / ADR-010）。
> BR番号はU3内で定義する。以前の未定義BR参照は、共有契約・規則ファイルへの明示的な出典へ置き換えた。

## 1. 規則（正本）

```yaml
rules:
  - id: BR1.1
    statement: "IntentExecutionRepository は core-command-use-case、実装は core-command-interface-adapter が所有する。find_by_id(&IntentExecutionId) は集約を返し、store(&event, &aggregate) は &mut self。JournalReader 一式は U4 の core-read-model-updater が所有する。版と通番は本家どおり usize"
    category: policy
    applies_to: [IntentExecutionRepository]
    trigger: "設計・実装・検証"
    logic: "現行ポートのコンパイルと依存クレートの照合"
    violation: "根拠と実装・試験を照合して是正する"
    source: "C3のB8/B13追記、現行IntentExecutionRepositoryポートに記録された2026-08-30の版所有裁定、gateway-taxonomy.md"
  - id: BR1.2
    statement: "find_by_id は最新スナップショットの DTO を検査付きで集約へ戻し、base.seq_nr()+1 を包含下限として後続イベントだけを取得する。通番連続性、manifest、DTO復号、payloadのaggregate_idを検査して replay し、snapshot.version() を with_version で保持する"
    category: validation
    applies_to: [IntentExecutionRepositoryImpl]
    trigger: "設計・実装・検証"
    logic: "基底なし・journalなしはNotFound、基底なし・journalありはCorrupt。差分の分類可能な破損はCorrupt。基底以前のjournalは検査しない。末尾欠落を独立に検出する仕組みはなく、ジャーナル保持要件を緩和するものではない"
    violation: "根拠と実装・試験を照合して是正する"
    source: "FR1.3、ADR-010、2026-08-30と2026-09-05の差分再生方針"
  - id: BR1.3
    statement: "store は同一コマンドの単一イベントと適用後集約を受ける。event.aggregate_id()!=aggregate.id() は本家ストアを呼ぶ前に Corrupt(source=WriteContract) として拒否する。型だけで対の対応を保証するという旧主張は撤回。expected_version は aggregate.version() をそのまま使い、seq_nr から導かない"
    category: validation
    applies_to: [IntentExecutionRepositoryImpl]
    trigger: "設計・実装・検証"
    logic: "genesisはseq_nr=1かつ版0。初回必須、以後はSnapshotStrategyが要求する場合だけsnapshotを保存し、それ以外はイベントのみ保存する。両経路のTx/CASは本家が所有する。Conflictのactual取得は失敗診断のためだけに再読取し、取得失敗時は0。store成功でも呼出側の版は変わらない。再試行政策はユースケースが所有する"
    violation: "根拠と実装・試験を照合して是正する"
    source: "FR1.2、C3/C6、ADR-010の不透明version裁定、2026-09-05の不整合対実測"
  - id: BR1.4
    statement: "JournalReader は after より大きいglobal通番を昇順に走査し JournalBatch を返す。未登録checkpointはZERO、前進は単調。投影表と公開処理の詳細は U4 所有の現行traitに従う"
    category: validation
    applies_to: [JournalReader]
    trigger: "設計・実装・検証"
    logic: "core-read-model-updater/src/orchestration/journal_reader.rs と app/aidlc の横断適合試験を参照する"
    violation: "根拠と実装・試験を照合して是正する"
    source: "C3/C6、FR1.1（実装主担当U4）、NFR3"
  - id: BR1.5
    statement: "RepositoryError<Id> は NotFound / Conflict / Io / Corrupt。公開の CorruptCause を書込ポートへ持ち込まない。診断は source 連鎖。ストア復号失敗、DTO検査失敗、差分通番飛び、foreign manifest、別実行payloadを Corrupt へ写す"
    category: policy
    applies_to: [RepositoryError]
    trigger: "設計・実装・検証"
    logic: "読取I/OはIo。書込のContractViolation/SerializationErrorはCorrupt。DTOとして成立した差分の未知ステージ等はdomain replayのクラッシュ境界であり、全異常がCorruptになるとは約束しない"
    violation: "根拠と実装・試験を照合して是正する"
    source: "error-handling.md、C3の2026-08-30裁定6"
  - id: BR2.1
    statement: "StorePath は space 単位の .aidlc-store.sqlite を表す。open は本家SQLiteストアを開き、親ディレクトリを新設しない。依存は event-store-adapter-rs =3.0.0 に固定する"
    category: policy
    applies_to: [StorePath]
    trigger: "設計・実装・検証"
    logic: "独自PRAGMA user_versionを版契約としない。busy_timeoutや並行モデルの旧未決記録を本Unitで再裁定せず、実装済みのI/O分類と関連Unitの現行契約を参照する"
    violation: "根拠と実装・試験を照合して是正する"
    source: "Q1=A、ADR-010、Cargo.toml"
  - id: BR2.2
    statement: "journal / snapshot のDDLは本家所有。RMUの投影表・チェックポイントはU4所有。旧自前の3表DDLを作成しない"
    category: policy
    applies_to: [IntentExecutionRepositoryImpl]
    trigger: "設計・実装・検証"
    logic: "本家スキーマとの接点はスキーマガード/横断読取テストで検証する"
    violation: "根拠と実装・試験を照合して是正する"
    source: "C6、ADR-010"
  - id: BR2.3
    statement: "イベント追記とCASは本家の原子的操作に委譲する。スナップショット更新経路はpayloadも更新し、イベントのみの経路は基底を維持して行のversionを進める"
    category: validation
    applies_to: [IntentExecutionRepositoryImpl]
    trigger: "設計・実装・検証"
    logic: "両バックエンドで古い版の競合拒否と成功時の再取得を検証。versionとseq_nrの等式はRepositoryの規則にしない"
    violation: "根拠と実装・試験を照合して是正する"
    source: "FR1.2、C6、SnapshotStrategy"
  - id: BR2.4
    statement: "旧 within_write_transaction は失効した。接続・Txを公開する独自EventStoreポートを再導入しない。登録簿の生成・更新はU7の現行Repository/投影境界に従う"
    category: policy
    applies_to: [IntentExecutionRepositoryImpl]
    trigger: "設計・実装・検証"
    logic: "旧登録簿直列化案をU3の実装契約として使わない"
    violation: "根拠と実装・試験を照合して是正する"
    source: "ADR-010、C3/C6の失効注記"
  - id: BR2.5
    statement: "永続化表現はアダプタ所有の IntentExecutionDto / IntentExecutionEventDto / IntentExecutionAggregateKeyDto。本家serdeが格納し、DTO::to_domainが検査付き再構成へ変換する。ドメインserde-mementoや旧StateWireを再導入しない"
    category: policy
    applies_to: [IntentExecutionDto]
    trigger: "設計・実装・検証"
    logic: "格納payloadはupstream観測面の正準JSON契約とは別。正確なフィールドは現行DTO定義で確認し、旧16/17属性表を維持しない"
    violation: "根拠と実装・試験を照合して是正する"
    source: "domain-persistence-neutrality.md、ADR-010、C6、U1 functional-design/rules.mdの正準JSON規則"
  - id: BR2.6
    statement: "輸送封筒の外側ID、seq_nr、occurred_atは適用後集約から組む。イベントのpayloadはイベントDTOから組み、manifestは intent-execution-event/1 とする"
    category: policy
    applies_to: [IntentExecutionRepositoryImpl]
    trigger: "設計・実装・検証"
    logic: "Repositoryが時計を持って時刻を再採番しない。payloadのaggregate_id一致はBR1.3で別途検査する"
    violation: "根拠と実装・試験を照合して是正する"
    source: "ADR-010、C5/C6"
  - id: BR2.7
    statement: "本家memoryとSQLiteへ同じRepository契約テストを課す。in_memoryは同じImplのバックエンド選択である"
    category: validation
    applies_to: [IntentExecutionRepositoryImpl]
    trigger: "設計・実装・検証"
    logic: "正常保存、再取得、Conflict、呼出側集約不変、別実行イベントの保存前拒否と既存状態維持を両方で検証する"
    violation: "根拠と実装・試験を照合して是正する"
    source: "gateway-taxonomy.md、FR1.3"
  - id: BR2.8
    statement: "Repositoryはドメイン公開APIとアダプタDTOだけを使い、ドメインの非公開表現やRMUのDTOを共有しない"
    category: policy
    applies_to: [IntentExecutionRepositoryImpl]
    trigger: "設計・実装・検証"
    logic: "クレート依存とDTO復号経路を照合する"
    violation: "根拠と実装・試験を照合して是正する"
    source: "abstract-data-type.md、field-visibility.md、domain-persistence-neutrality.md、cqrs-boundaries.md"
  - id: BR3.1
    statement: "ADR-007により旧WorkspaceLock/FsWorkspaceLock/LockProtocol/LockIdentity/ProcessProbe/audit_lockと専用lintを退役する"
    category: policy
    applies_to: [RetiredLockMachinery]
    trigger: "設計・実装・検証"
    logic: "履歴記録を除き旧ロック実装を新設しない"
    violation: "根拠と実装・試験を照合して是正する"
    source: "FR1.2、ADR-007"
  - id: BR3.2
    statement: "U3の永続化競合制御は本家ストアのTxと楽観versionを使い、旧mkdirロックdirの生成を復活させない"
    category: policy
    applies_to: [RetiredLockMachinery]
    trigger: "設計・実装・検証"
    logic: "投影公開の排他など他責務をこの退役規則と混同しない"
    violation: "根拠と実装・試験を照合して是正する"
    source: "FR1.2、ADR-007、deviations #4"
  - id: BR3.3
    statement: "journal_protocol.qnt は1集約・writer2・投影1、SnapshotStrategy::every(1)の構成を対象にする。snapshot_tracks_journal(snapSeq==journalLen)はその設定限定。既定10/間欠更新までの保証として引用しない"
    category: validation
    applies_to: [JournalProtocolModel]
    trigger: "設計・実装・検証"
    logic: "既存モデルの8不変条件と4witnessを保持。version_equals_journalはモデル内の採番抽象であり、ドメインがversionをseq_nrから作る根拠にはしない。任意Nの基底更新と差分再生は実装テストで検証する"
    violation: "根拠と実装・試験を照合して是正する"
    source: "FR1.2、NFR3、ADR0003、modules/app/aidlc/tests/journal_protocol_conformance.rs"
  - id: BR3.4
    statement: "Quintの型検査・不変条件・witnessと変異の検出力を検証する。変異結果は記録された検証の範囲で扱い、過去の合格を現在の実測に置き換えない"
    category: validation
    applies_to: [JournalProtocolModel]
    trigger: "設計・実装・検証"
    logic: "scripts/quint-gate.sh と対応する検証記録を参照する"
    violation: "根拠と実装・試験を照合して是正する"
    source: "ADR0003、team.md Testing Posture"
  - id: BR3.5
    statement: "tests/conformance/fixtures/journal_protocol のITFを modules/app/aidlc/tests/journal_protocol_conformance.rs で再生する。Repositoryはevery(1)を明示し、JournalReaderImplと投影進捗を突合する"
    category: validation
    applies_to: [JournalProtocolModel]
    trigger: "設計・実装・検証"
    logic: "load/store_ok/store_conflict/catchup/crash/idleを駆動し、journal長・snapshot版/通番・checkpoint等を照合する。モデルの成功後版取得は再取得を含むテスト手順への射影で、storeが借用集約を書き換える意味ではない"
    violation: "根拠と実装・試験を照合して是正する"
    source: "FR1.2、NFR3"
  - id: BR4.1
    statement: "IntentIdとIntentExecutionIdは別のUUIDv7値型。旧kebab識別子は受理しない"
    category: policy
    applies_to: [IntentId]
    trigger: "設計・実装・検証"
    logic: "小文字36字、version 7 / variant 10xxをparseで検証する"
    violation: "根拠と実装・試験を照合して是正する"
    source: "U2 pending-revision 8、01号§3.3"
  - id: BR4.2
    statement: "IntentDirNameは ^[0-9]{6}-[a-z0-9]+(?:-[a-z0-9]+)*$、全体64字以下。連続ハイフンと空区間を拒否する"
    category: policy
    applies_to: [IntentDirName]
    trigger: "設計・実装・検証"
    logic: "260822-a-bは受理、260822-a--b/260822-a-/260822-は拒否。予約ラベルは生成側の責務"
    violation: "根拠と実装・試験を照合して是正する"
    source: "11号§2.2、pending-revision項目2"
  - id: BR4.3
    statement: "旧WorkflowExecutionSnapshot/WorkflowExecutionStateとdomain serde-mementoを現行の再構成経路に採用しない。現行はIntentExecutionDtoから検査付きコンストラクタを経由して集約を復元する"
    category: policy
    applies_to: [IntentExecutionDto]
    trigger: "設計・実装・検証"
    logic: "旧改名の履歴から廃止済みState型を復活させない"
    violation: "根拠と実装・試験を照合して是正する"
    source: "U2 pending-revision 9の後続裁定、domain-persistence-neutrality.md"
  - id: BR5.1
    statement: "現行設計のYAML・要約・署名・配置・要求対応を同期し、過去Reviewは変更せず残す。FR1の集約対応はU3、FR1.1実装担当はU4"
    category: policy
    applies_to: [SpecDocument]
    trigger: "設計・実装・検証"
    logic: "出典を名前だけで推測せず、現行ポートと裁定へ結び付ける"
    violation: "根拠と実装・試験を照合して是正する"
    source: "unit-of-work-story-map.md、B4申し送り"
  - id: BR5.2
    statement: "検証は対象と構成を明記し、Repository契約/差分再生/ITF/Quint/関連lintを確認する。workspace全体、coverage、audit、CIの合否は実測の証拠がある範囲だけ報告する"
    category: validation
    applies_to: [IntentExecutionRepositoryImpl]
    trigger: "設計・実装・検証"
    logic: "文書是正だけで正式レビュー合格やゲート承認を宣言しない。実行済みと静的照合、未実行を是正報告で区別する"
    violation: "根拠と実装・試験を照合して是正する"
    source: "U3合格条件、NFR2/NFR3/NFR4"
```

## 2. 規則の要約

| ID | 分類 | 現行規則 |
|---|---|---|
| BR1.1 | policy | IntentExecutionRepository は core-command-use-case、実装は core-command-interface-adapter が所有する。find_by_id(&IntentExecutionId) は集約を返し、store(&event, &aggregate) は &mut self。JournalReader 一式は U4 の core-read-model-updater が所有する。版と通番は本家どおり usize |
| BR1.2 | validation | find_by_id は最新スナップショットの DTO を検査付きで集約へ戻し、base.seq_nr()+1 を包含下限として後続イベントだけを取得する。通番連続性、manifest、DTO復号、payloadのaggregate_idを検査して replay し、snapshot.version() を with_version で保持する |
| BR1.3 | validation | store は同一コマンドの単一イベントと適用後集約を受ける。event.aggregate_id()!=aggregate.id() は本家ストアを呼ぶ前に Corrupt(source=WriteContract) として拒否する。型だけで対の対応を保証するという旧主張は撤回。expected_version は aggregate.version() をそのまま使い、seq_nr から導かない |
| BR1.4 | validation | JournalReader は after より大きいglobal通番を昇順に走査し JournalBatch を返す。未登録checkpointはZERO、前進は単調。投影表と公開処理の詳細は U4 所有の現行traitに従う |
| BR1.5 | policy | RepositoryError<Id> は NotFound / Conflict / Io / Corrupt。公開の CorruptCause を書込ポートへ持ち込まない。診断は source 連鎖。ストア復号失敗、DTO検査失敗、差分通番飛び、foreign manifest、別実行payloadを Corrupt へ写す |
| BR2.1 | policy | StorePath は space 単位の .aidlc-store.sqlite を表す。open は本家SQLiteストアを開き、親ディレクトリを新設しない。依存は event-store-adapter-rs =3.0.0 に固定する |
| BR2.2 | policy | journal / snapshot のDDLは本家所有。RMUの投影表・チェックポイントはU4所有。旧自前の3表DDLを作成しない |
| BR2.3 | validation | イベント追記とCASは本家の原子的操作に委譲する。スナップショット更新経路はpayloadも更新し、イベントのみの経路は基底を維持して行のversionを進める |
| BR2.4 | policy | 旧 within_write_transaction は失効した。接続・Txを公開する独自EventStoreポートを再導入しない。登録簿の生成・更新はU7の現行Repository/投影境界に従う |
| BR2.5 | policy | 永続化表現はアダプタ所有の IntentExecutionDto / IntentExecutionEventDto / IntentExecutionAggregateKeyDto。本家serdeが格納し、DTO::to_domainが検査付き再構成へ変換する。ドメインserde-mementoや旧StateWireを再導入しない |
| BR2.6 | policy | 輸送封筒の外側ID、seq_nr、occurred_atは適用後集約から組む。イベントのpayloadはイベントDTOから組み、manifestは intent-execution-event/1 とする |
| BR2.7 | validation | 本家memoryとSQLiteへ同じRepository契約テストを課す。in_memoryは同じImplのバックエンド選択である |
| BR2.8 | policy | Repositoryはドメイン公開APIとアダプタDTOだけを使い、ドメインの非公開表現やRMUのDTOを共有しない |
| BR3.1 | policy | ADR-007により旧WorkspaceLock/FsWorkspaceLock/LockProtocol/LockIdentity/ProcessProbe/audit_lockと専用lintを退役する |
| BR3.2 | policy | U3の永続化競合制御は本家ストアのTxと楽観versionを使い、旧mkdirロックdirの生成を復活させない |
| BR3.3 | validation | journal_protocol.qnt は1集約・writer2・投影1、SnapshotStrategy::every(1)の構成を対象にする。snapshot_tracks_journal(snapSeq==journalLen)はその設定限定。既定10/間欠更新までの保証として引用しない |
| BR3.4 | validation | Quintの型検査・不変条件・witnessと変異の検出力を検証する。変異結果は記録された検証の範囲で扱い、過去の合格を現在の実測に置き換えない |
| BR3.5 | validation | tests/conformance/fixtures/journal_protocol のITFを modules/app/aidlc/tests/journal_protocol_conformance.rs で再生する。Repositoryはevery(1)を明示し、JournalReaderImplと投影進捗を突合する |
| BR4.1 | policy | IntentIdとIntentExecutionIdは別のUUIDv7値型。旧kebab識別子は受理しない |
| BR4.2 | policy | IntentDirNameは ^[0-9]{6}-[a-z0-9]+(?:-[a-z0-9]+)*$、全体64字以下。連続ハイフンと空区間を拒否する |
| BR4.3 | policy | 旧WorkflowExecutionSnapshot/WorkflowExecutionStateとdomain serde-mementoを現行の再構成経路に採用しない。現行はIntentExecutionDtoから検査付きコンストラクタを経由して集約を復元する |
| BR5.1 | policy | 現行設計のYAML・要約・署名・配置・要求対応を同期し、過去Reviewは変更せず残す。FR1の集約対応はU3、FR1.1実装担当はU4 |
| BR5.2 | validation | 検証は対象と構成を明記し、Repository契約/差分再生/ITF/Quint/関連lintを確認する。workspace全体、coverage、audit、CIの合否は実測の証拠がある範囲だけ報告する |
