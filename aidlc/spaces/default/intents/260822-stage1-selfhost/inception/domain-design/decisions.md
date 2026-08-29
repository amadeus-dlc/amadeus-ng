# decisions — Domain Design の ADR ログ（ES 設計・全面改訂版。ADR-005 は 2026-08-22 再入で完全移動へ改訂）

> Domain Design（Inception 2.6）成果物・改訂版。各 ADR は `domain-design-questions.md`（Q1〜Q9 確定）に
> 遡及する。出典: `../requirements-analysis/requirements.md`、RE 成果物（`architecture.md` /
> `component-inventory.md`）、`../practices-discovery/team-practices.md`、設計監査
> `../../../knowledge/aidlc-shared/design-audit-2026-08-22.md`（R1/R2 DECIDED — R2 の文言は
> ADR-002 が上書き改訂する。下記参照）。

## ADR-001: イベントソーシングを採用する（event-store-adapter-rs 前提）

- **Context** — 集約の遷移が監査台帳と状態ファイルの2面に永続化される upstream 構造（B9:
  真実源は監査、状態はキャッシュ）をどのパラダイムで実装するかが未裁定だった。検討過程で
  「監査先行 WAL + 同期プロジェクション」「集約がイベント列を返しステート永続化する」等の
  折衷案が出たが、オーナー裁定は「ステートソーシングなら集約でイベントを扱わない。イベントを
  扱うならイベントソーシング。中途半端な設計は保守を難しくする」。B9 を素直に読めば
  「イベント列が真実源で状態は導出可能」= ES の定義そのものである。
- **Decision** — **イベントソーシングを採用する**。機構は j5ik2o/event-store-adapter-rs の型と
  API（`persist_event` / `persist_event_and_snapshot`（同一 Tx + 楽観 version）/
  `get_latest_snapshot_by_id` + `get_events_by_id_since_seq_nr` + `replay`）に従う。
  スナップショットは毎コミット更新（通常運転の replay は 0 件）。
- **Consequences** — (+) 真実源が一意（ジャーナル）になり B9 が構造で成立。(+) パラダイムの
  混在が消え、リプレイ・監査・復旧が同一機構に載る。(−) 既存のロック/状態ファイル機構の
  大規模改修（ADR-007）。(±) upstream 互換ファイルはリードモデルとして維持（ADR-003）。
- **Alternatives Rejected** — *ステートソーシング + 集約がイベント列を返す折衷*: 1コマンド
  1イベントの規律（ADR-002）に反しリプレイ不能、パラダイム混在。*監査先行 WAL + 同期
  プロジェクション（非 ES を自称）*: 実体は ES + スナップショットの誤ラベルで、機構を
  自前発明することになる。*通知型 publisher*: イベントが真実源でなくなり B9 と矛盾。

## ADR-002: 集約 = FSM・1コマンド1イベント・decide/apply 分離（統一ルール）

- **Context** — next_decision（21分岐ラダー）の置き場（O1）とイベント発行の形が未裁定だった。
  オーナー明言（2026-08-22、統一ルール・横展開）:「集約は FSM。状態としてのデータと状態遷移の
  ための振る舞いは同じ型に閉じ込める」「コマンド実行1回につき必ず1個のイベント。これを守らないと
  リプレイができない」。
- **Decision** — ① コマンドは **decide**: `approve_gate(&mut self, ...) → Result<GateApproved,
  ApproveError>` のように**単一ドメインイベント**を返す（1コマンド1イベント・絶対）。
  ② **apply_event(&mut self, &Event)** が状態を進め、リプレイと通常実行を同一経路にする。
  ③ 遷移は `&mut self`（typestate 不採用）。④ `next_decision` は `WorkflowExecution` の
  クエリメソッド（`&self, &WorkflowDefinition, ...`）。⑤ 有効プラン畳み込み（R2）の移設先も
  `WorkflowExecution` のメソッド — **設計監査 R2 の「orchestration 側ドメインサービスへ」という
  文言はこの裁定が上書きする**（監査時点より強い Tell-Don't-Ask 形。design-audit の R2 は
  移設先の記述のみ本 ADR に置き換わり、移設自体の裁定は不変）。⑥ ユースケースは進行管理・
  フロー制御のみ（ビジネスロジック禁止）。
- **Consequences** — (+) リプレイ規律が成立（1イベント = 1 apply）。(+) FSM の状態・遷移・判断が
  単一型で Quint モデルと 1:1。(+) upstream 監査行の複合発行問題はドメインから消える
  （ADR-003 の投影が解く）。(−) ドメインイベント語彙の新設（コマンドと1:1 の 11 変種程度）。
- **Alternatives Rejected** — *Vec<DomainEvent> 返し*: 1コマンド1イベント違反、リプレイ時の
  apply 対応が崩れる。*独立ドメインサービス / ユースケース層への判断配置*: 状態の所有者の外で
  判断する Ask 型。*typestate*: 動的アクション列のリプレイ・再構成と相性が悪く過剰。

## ADR-003: SQLite ストア + upstream 互換ファイルはリードモデル + RMU

- **Context** — イベントの物理格納先が未裁定だった。upstream の監査台帳は markdown
  （`---` 区切りブロック、クローン別シャード、git 交換）で、86語彙・見出し・フィールド順が
  観測可能契約（D6）。一方 ES のストアには追記・順序・条件付き書込・差分読取が要る。
- **Decision** — **Repository → EventStoreImpl(sqlite client) → SQLite ← Read Model Updater**。
  ジャーナル・スナップショット・チェックポイントは SQLite（ローカル、git 管理外）。
  `aidlc-state.md`・監査シャード等の upstream 互換ファイルは**すべてリードモデル（投影）**とし、
  **RMU = チェックポイント付きのプロセス内差分関数**（AWS Lambda 型・常駐なし）がコマンド末尾
  （Tx コミット後・プロセス終了前）に同期キャッチアップで生成する。1ドメインイベント →
  監査行 N 行（フェーズ境界トリオ等）の描画は RMU の投影規則。クラッシュ時は次回呼出が
  チェックポイントから冪等修復（真実源はジャーナル = B9 維持）。
- **Consequences** — (+) ドメインイベント語彙と upstream 監査行語彙が分離し、両方が単純になる。
  (+) 投影は決定的・冪等で再生成可能。(−) SQLite という新しいオンディスク成果物
  （逸脱台帳へ登録 — ADR-007 と併せて）。(−) ハーネスがコマンド間にファイルを読むため、
  投影のプロセス内同期実行が必須（設計で担保）。
- **Alternatives Rejected** — *markdown シャードをイベントストアに流用*: ドメインイベントと
  監査行の語彙が癒着し、ストア形式が互換契約の人質になる。*常駐 RMU（本来のポーリング型）*:
  ワンショット CLI に常駐物を持ち込む。upstream にも無い。*読取時の遅延投影*: ハーネスが
  コマンド間に読むファイルが古くなり互換違反。

## ADR-004: WorkflowExecution が集約ルート、状態ファイルはリードモデル

- **Context** — `aidlc-state.md` の所有主張が仕様間で3通りに割れていた（O3）。ES 採用で
  スナップショットの置き場も再定義が必要になった。
- **Decision** — 集約ルートは **WorkflowExecution**（identity = intent、version / seq_nr を保持）。
  スナップショットは SQLite のスナップショットテーブル（ライブラリ管理）。`aidlc-state.md` は
  **リードモデル**であり、集約でも媒体スナップショットでもない。`state_file_io` は RMU の
  投影ライタ部品に転生。01号 §3 の集約候補表から `StateFile` を落とす（FR8.2 と同時）。
- **Consequences** — (+) 所有が一意、B-2 の `WorkflowExecutionRepository` は ES 形
  （store / find_by_id）で設計入力が確定。(−) 01号・11号の表改訂（FR8.2）。
- **Alternatives Rejected** — *StateFile 独立集約*: 媒体を集約に昇格させる不変条件が無い。
  *状態ファイル = スナップショット*: ストア外のスナップショットは楽観 version と同一 Tx に
  できず、ライブラリの機構から外れる。

## ADR-005: PlanAction の所有を workflow_definition へ一本化（R1 の設計反映 — 完全移動、2026-08-22 改訂）

- **Context** — `workflow_definition` が `orchestration::PlanAction` を import する
  コンテキスト間逆依存が残存（設計監査 C13）。01号/12号は workflow-definition 所有を宣言済み。
  初版 ADR-005 は「`orchestration` は re-export で後方互換を保つ」としたが、**2026-08-22 オーナー裁定
  （`coding-rules/module-visibility.md` 追補）: 利便性のための再エクスポートはどこでも禁止** — 別コンテキスト
  によるエイリアス再輸出は型の所有元を消費側パスから読めなくし、構造が読めなくなる。
- **Decision** — `PlanAction` を `workflow_definition` の所有とし、**完全移動**で行う: `orchestration` 側の
  定義を削除し、呼出側（orchestration コンテキスト内・ユースケース層・ゲートウェイ層・テスト）の参照パスを
  `core_domain::workflow_definition::PlanAction` へ **FR8.3 の同一 Bolt で一斉修正**する。`orchestration` による
  再輸出（`pub use workflow_definition::PlanAction`）は置かない。
- **Consequences** — (+) 依存方向と所有が仕様と一致し、消費側パスから所有元が読める。(+) 二重経路が生じない。
  (−) FR8.3 の Bolt が呼出側一斉修正を含み PR がやや大きくなる（直列 PR 運用では 1 PR に収める）。
- **Alternatives Rejected** — *現状維持*: 仕様との矛盾が恒久化。*`orchestration` からの re-export で後方互換
  （初版 ADR-005 の裁定）*: 2026-08-22 の再エクスポート禁止裁定に抵触 — 所有元が読めなくなる。段階移行は
  呼出側修正を同一 Bolt に含めることで不要。

## ADR-006: async は初期化から（tokio）。ドメインは純粋・同期のまま

- **Context** — event-store-adapter-rs の API は async。amadeus-ng は async ランタイム無しの
  ワンショット CLI だった。将来ウェブサーバの実装可能性もある（オーナー明言）。
- **Decision** — **tokio（current_thread）を導入し async main から初期化**する。コントローラ・
  ユースケースは async fn、**ドメイン（集約）は純粋・同期のまま**（`.await` は集約に現れない）。
  event-store-adapter-rs crate への直接依存は現状見送り（aws-sdk-dynamodb + tonic + Bigtable
  クライアントが feature ゲート無しのハード依存のため）、**trait 群を本家と同形でローカル定義**し、
  本家が feature 化 / SQLite 実装を得たら乗り換え可能な形を保つ（上流貢献も選択肢）。
- **Consequences** — (+) ライブラリの機構をそのまま採れる。(+) 将来のウェブサーバに拡張可能。
  (−) tokio 依存の追加（current_thread で最小化）。(−) 本家 crate との同形性維持は手動。
  (±) 永続化メソッド名 `store` は coding-rules gateway-taxonomy §2b の許容動詞
  （find_by_id / find / save / remove）に無い **ES 拡張語彙**として本 ADR が明示的に採用する
  （event-store-adapter-rs の API 同形性を優先。§2b はステートソーシング Repository の規則であり、
  ES Repository の動詞は本家ライブラリの語彙に従う — 正本側への注記追加は FR8.1 の canon 修正に
  同梱する。旧称 AuditLedgerRepository の正本残存も同修正で除去）。
- **Alternatives Rejected** — *同期ポートの自作*: 将来要件（ウェブサーバ）と本家との API 同形性の
  価値を捨てることになる。*本家 crate 直接依存*: AWS SDK + gRPC + Bigtable の全ツリーが
  ローカル CLI に入り、ビルド・監査面が不釣り合い。*先に本家へ feature 化を貢献*: stage-1 の
  最短経路から外れる（後続で実施可能）。

## ADR-007: ロック機構の退役 — SQLite Tx + 楽観 version へ置換

- **Context** — mkdir ロック（md5 dir・所有者スタンプ・reap）は「テキストファイル群への
  read-modify-write にトランザクションが無い」upstream 前提の産物。ES + SQLite でその前提が
  消える（オーナー指摘）。
- **Decision** — ストア書込は **SQLite Tx + 楽観 version**（条件付き書込）で保護し、mkdir
  ロック機構は**退役**する（ロック dir は生成しない）。リードモデル書込は冪等生成 + 単一ファイル
  原子性（tmp+rename）で足り、チェックポイントは Tx 内更新。`FsWorkspaceLock`・`WorkspaceLock`
  ポート・`LockProtocol`・`reap_eligible`・`OwnerStamp` は退役。`audit_lock.qnt` は
  「ジャーナル/スナップショット/version/チェックポイント協定」（version 競合拒否・
  チェックポイント単調性・投影冪等性）の検証モデルへ**改訂**（意味論は Quint で検証し続ける）。
  逸脱台帳に「並行制御の置換（ロック dir 非生成）・SQLite ファイルの追加・互換ファイルは
  リードモデルとして維持」を登録する。
- **Consequences** — (+) 並行制御が1機構に集約、reap 等の複雑性が消える。(+) 既存 Quint 資産は
  改訂して存続。(−) #18 で upstream 準拠させた FsWorkspaceLock 実装の退役（正しい設計への
  回収と位置づける）。(−) requirements FR1.1/FR1.2 の合格基準文言の改訂が必要（後方ジャンプで
  実施予定）。
- **Alternatives Rejected** — *mkdir ロック併存*: 二重の並行制御は「中途半端な設計」そのもの。
  *SQLite を使いつつファイルが真実源*: 真実源が2つに割れ ADR-001 と矛盾。

## ADR-008: WorkflowDefinition はエンティティ — 不変の ID と内容版 revision、WorkflowExecution からは ID で間接参照（2026-08-23 追加）

- **Context** — 12 号 §2.1 は `WorkflowDefinition` を集約ルートへ昇格させたが識別子を定義しておらず、
  実コード `WorkflowDefinition { graph, grid, scopes }` にも `stage-graph.json`（素の 33 要素配列）にも
  ID / version が無い。U2 機能設計は `WorkflowExecution` が定義を「参照渡しで使う」とだけ書き、
  どの定義で始まったかを残していなかった。オーナー指摘（2026-08-23）: 集約はエンティティであり
  ID が無いのはまずい。集約間の依存は ID による間接参照。**内容アドレスを ID にすると内容が変わった
  ときに追跡不能になり、「内容が変わっても追跡できる」というエンティティの責務に反する**。
- **Decision** — (1) `WorkflowDefinition` に `id: WorkflowDefinitionId`（内容が変わっても不変の系譜 ID。
  Repository 実装が harness.json の `name` — このハーネスにインストールされた定義 — から付与）と
  `revision: DefinitionRevision`（3 入力の正準 JSON の `sha256:`、U1 canon-json hash-canonical で
  Repository 実装が計算。**値属性であって識別子ではない**）を追加する。(2) C4 を
  `find_by_id(&WorkflowDefinitionId)` に改訂し、引数なし `find()` は**廃止**（後方互換の併存なし）。
  (3) `WorkflowExecution` は `definition_id` / `definition_revision` を `Started` に記録して保持する。
  `start` は引数の `&WorkflowDefinition` の id / revision を無条件に記録するだけ（比較対象となる既存状態が
  無い静的コンストラクタ — 検査しない）。Started 適用後に `&WorkflowDefinition` を受け取るクエリ／コマンド
  （現時点では `next_decision`）は id が一致しなければ `Err(CommandError::DefinitionMismatch)`。revision の差は
  Err にしない（計画は `Started` で自己完結、upstream も dist 更新をまたいでワークフローを続ける）。
  （2026-08-23 U2 nfr-design レビュー所見 2 により `start` の検査を削除 — rules.md BR2.6 と同期）(4) 12 号 §2.1 / 01 号の集約表へ識別子を追記（U9 の
  canon 追従 FR8.2 に同梱）。
- **Consequences** — (+) 集約間参照が ID に統一され、エンティティ / 値オブジェクトの区別が型に出る。
  (+) 来歴（どの定義・どの内容版で始まったか）がイベントに残り、ピン更新時の drift を観測できる。
  (−) Bolt B3 の範囲が core-domain を越え、use-case の trait（C4）・interface-adapter の
  `WorkflowDefinitionRepositoryImpl`（id / revision の付与、canon-json 依存）・既存テスト（golden parity /
  repository impl test / ITF 準拠）の同時修正が要る。(−) ITF 準拠テストは合成 `WorkflowDefinitionId`
  を使う。
- **Alternatives Rejected** — *内容アドレス ID（ダイジェストを ID にする）*: 値の同一性であり、内容が
  変われば別物になって追跡できない — エンティティの責務違反（オーナー却下）。*upstream ピンを ID に
  する*: ピンはデータに含まれず（`stage-graph.json` に version 無し）、テストシームのローカル差し替えを
  区別できない。*ID なしのまま `find()` を維持*: エンティティに ID が無い現状の温存。

- **追記 2026-08-29（Bolt B8 — 解決済み計画の表示属性は例外、オーナー裁定）**: 「定義の詳細を
  イベントへ複製しない」に限定的な例外を設ける。監査シャードの逐語互換（FR1.1）に必要な
  **表示属性 3 値**（ステージ番号・表題・担当エージェント名 — `StageDisplay`）と**走査結果**
  （`WorkspaceScan`）は、`WorkflowExecution::start` が計画を解決する時点の**観測事実**として
  `Started` イベントへ焼き込む。理由は NFR3（クラッシュ再構成）: 投影が定義ファイルを引く形だと、
  定義が後から編集されたとき過去イベントの再生が当時と同じ行を描けない。ジャーナルだけで
  リードモデルを完全復元できることを優先した。**定義全体（グラフ構造・依存・センサー等）の複製は
  引き続き禁止** — 例外は解決済み計画の表示属性と走査結果に限る。差分投影への計画の供給は
  イベントを太らせず `ResolvedPlan` を投影核の引数とする（取得ループが初回にジャーナル先頭から
  控える — 実装の詳細は `construction/u4-read-model-updater/developer-report-1.md` §6）。

## ADR ステータス注記

初版の ADR-001〜006（WAL + 同期プロジェクション時代）は本改訂版が**全面的に置き換える**。
初版の裁定のうち存続するもの: 集約=FSM 統一ルール（ADR-002 に吸収・強化）、
WorkflowExecution 集約ルート（ADR-004 に吸収・精密化）、PlanAction 一本化（ADR-005 — 2026-08-22 の再エクスポート禁止裁定により re-export 併用から完全移動へ改訂）、
フック4本のサブコマンド化（Q3 — CliDispatcher の behaviour に記録、独立 ADR は不要と判断）。

## ADR-009: CQRS の依存境界をクレートで物理強制する — RMU は独立クレート（2026-08-24 追加）

- **Context** — ADR-001（ES 採用）/ ADR-003（互換ファイルはリードモデル + RMU）/ ADR-004
  （状態ファイルはリードモデル）で読み書きのモデルは既に分かれている。しかし**依存の向きが
  どこにも強制されていない**。実測（2026-08-24）: 読取側の契約 `JournalReader` /
  `ProjectionName` / `GlobalSeqNr` が**コマンド側の `core/use-case` クレートに同居**しており、
  さらに U4（RMU）は unit-of-work で `embedded` 指定のため `core/interface-adapter` の中、
  `EventStoreImpl` と同居する計画だった。この形では RMU を別クレートにしても `Cargo.toml` に
  `core-use-case` が並び、team.md が謳う「依存は Cargo.toml の不在により物理的に内向き強制」が
  **CQRS 境界だけ空振りする**。
- **Decision**（オーナー明言 2026-08-24）— CQRS の依存規則を**クレート境界で物理強制**する。
  - **コマンド側はクエリ側に依存しない。クエリ側もコマンド側に依存しない**（相互に独立）。
  - **RMU が要るのはドメインイベントだけ**（オーナー訂正 2026-08-24）。RMU はイベントを
    **受信して**リードモデルを作成・更新する。それ以外は要らない — **ジャーナルを読みに行くのも、
    チェックポイントを進めるのも RMU の仕事ではない**。RMU は両者の間に立つ第三の要素として
    両側に依存してよい立場にはあるが、実際に必要なのは**イベント型とリードモデルの書込先だけ**
    である。依存は RMU → 両側の一方向で、両側から RMU への依存は無い。

    ```
    コマンド側  ←── RMU ──→  クエリ側
        ↑                       ↑
        └──── 依存しない ───────┘
    ```
  - **コマンド側は最新状態を常に集約から判断する**。リードモデルから現在状態を読むことは
    禁止であり、そもそもリードモデルは常に遅延しているので**物理的にできない**。
  - これを効かせるため **RMU を独立クレートに切り出す**（U4 は `embedded` → 独立クレート）。
  - **読取側の契約を中立クレートへ切り出す必要は無い**（初稿の `core/event-stream` 案は撤回）。
    RMU が `JournalReader` / `ProjectionName` / `GlobalSeqNr` を**使わない**からである。
    これらは `core/use-case` に置いたまま、**合成ルートが**それを使ってイベントを読み、
    RMU へ渡す。RMU の入口はドメインイベント 1 本。
  - **U4 の責務範囲を改訂する**。`unit-of-work.md` の U4 は「チェックポイント以降のイベントを
    ジャーナルから読み、…投影し、**チェックポイントを進める**冪等な差分関数」と書いているが、
    ジャーナル読取とチェックポイント前進は**呼び出す側（合成ルート / U7）へ移す**。U4 は
    「イベント列 → リードモデル」の純粋な投影に絞る。これにより RMU はコマンド側のポートを
    一切知らずに単体テストできる。
  - **2026-08-28 改訂（オーナー裁定）— 直前の 2 項目（中立クレート不要の理由 / U4 責務の縮小）は失効**。
    「JournalReader を呼び出すのは RMU ではないのか」というオーナーの指摘を受け、構造化質問で
    裁定 A に確定した: **RMU コンポーネントが取得ループを持つ** — `JournalReader::events_after`
    で差分を引き、純粋投影核 `project(events, read_model)` へ渡し、`advance_checkpoint` を
    自分で進める。「イベントだけ受信する」は**投影核**への制約として残る。SQLite にはストリームが
    無いので、AWS 版 RMU が Streams から受信するのと同じ役割を**自分で引く**形（プル型）で果たす。
    帰結:
    1. `JournalReader` trait と読取側語彙（`ProjectionName` / `GlobalSeqNr` / `JournalReadError`）の
       所有は `core/use-case` → **RMU クレート（U4）へ移す**。呼ぶ者がポートを所有する —
       `core/use-case`（U5/U6）はこの trait を一度も呼ばない（所有だけしていた匂いの解消）。
       コードの移動は U4 Bolt で実施（本項は先行する裁定の記録であり、現時点の実装は移動前）。
    2. `unit-of-work.md` の U4 原文（「ジャーナルから読み、…投影し、チェックポイントを進める
       冪等な差分関数」）は**改訂不要のまま正**となる（上の「呼び出す側へ移す」計画を破棄）。
    3. U7（合成ルート）は RMU の**起動のみ**を持ち、駆動ループを持たない。合成ルートは
       カバレッジ除外（インタビュー Q5 裁定）であり、ループの実ロジック（バッチ・チェック
       ポイント単調性・エラー処理）をテストの届かない場所に置かないため。
    4. `JournalReaderImpl` の置き場所（`core/interface-adapter` に留めて RMU クレートの trait を
       実装するか、クエリ側アダプタへ移すか）は **U4 機能設計で裁定**する。従来のアダプタ分割
       却下理由（C6 の 3 表定義の所在）は ADR-010 で失効済み — 我々が定義する表は
       `amadeus_projection_checkpoint` 1 表のみ。
  - **2026-08-29 改訂（オーナー裁定 — 層の側分割）**: 上の 4.（`JournalReaderImpl` の
    置き場所）は確定した — **RMU クレートに置く**（ジャーナルを読むことが RMU の仕事
    そのもの）。さらに「そもそも interface-adapter / use-case はコマンド側とクエリ側に
    分割する」の裁定により、本 ADR の「アダプタは 1 クレートのまま」も失効: クレートは
    `core-domain`（共有）/ `core-command-use-case` / `core-command-interface-adapter` /
    `core-query-read-model-updater`（クエリ側の全実体 — 読取語彙・SQLite 読取実装・
    取得ループ・純粋投影核・投影ライタ）となり、命名は `core-{command,query}-` 接頭辞で
    統一する。両側を知ってよいのは合成ルート（U7）だけ。共有部品の行き先: エラー分類と
    I/O 写像は側ごとに専用化、`StorePath` と直列化型判別子（manifest 定数）は
    `core-domain` へ（詳細は `construction/u4-read-model-updater/crate-structure-proposal.md`）。
    実施は B8。
  - **2026-08-29 第 2 改訂（オーナー裁定 — ドメインはコマンド側）**: `modules/core/domain` は
    **`modules/core/command/domain`（`core-command-domain`）** へ — ドメインは**コマンド側の
    持ち物**であり、**クエリ側（RMU）はドメインクレートに絶対依存しない**。前追記の
    「core-domain は共有」と「`StorePath` / manifest 定数は core-domain へ（共有語彙）」は失効。
    帰結: (1) RMU はジャーナルの **wire 形式（直列化 JSON + manifest タグ）を自前の型で
    parse** する — 両側が共有するのはコードではなくデータ契約（Published Language）であり、
    乖離は合成ルートのコントラクトテスト（コマンド側が直列化 → クエリ側が parse → 同値）で
    機械検出する。(2) リードモデル語彙（`AuditFieldKey` / 監査順序付けの純関数 / 単一行
    プリミティブ）はクエリ側の出力の整合性部品なので **RMU へ移す**（11-workspace §2.2/§2.3 の
    「domain に残す」は側分割以前の記述として失効 — 仕様側を改訂）。(3) `StorePath` は
    コマンド側に残し、RMU はストアの場所を自前の型で合成ルートから受け取る。(4) manifest 定数は
    側ごとに持ち、コントラクトテストで同値を固定する。(5) 両側がコードで共有してよいのは
    `core-infrastructure`（言語拡張）と shared の Published Language スキーマ
    （`audit-events` / `message-catalog` 等）のみ。
  - アダプタは**1 クレートのまま**（`core/interface-adapter`）とし、`EventStoreImpl` が
    `EventStore`（コマンド）と `JournalReader`（読取）の両契約を実装する。SQLite スキーマ定義
    （C6 の 3 表）が 1 箇所に残るので重複しない。
  - **判定は `Cargo.toml` を見るだけでよい**。コマンド側クレートの依存にクエリ側が現れたら違反、
    クエリ側クレートの依存にコマンド側が現れたら違反、RMU はどちらが現れてもよい。
- **Consequences** — (+) 依存規則が型検査ではなくビルドで落ちる（`Cargo.toml` の不在）。
  (+) ADR-005（内部可変性の禁止、2026-08-24）により `EventStoreImpl` は `&mut self` で排他所有に
  なったため、**コマンド側と読取側が 1 接続を共有できない** — 自然に別接続になり、CQRS が求める
  「読取側は独立に走る」形へ借用チェッカが寄せる。(+) 判定が `Cargo.toml` の目視で済む
  （依存グラフを追う必要がない）。(−) クレートが 1 つ増える（RMU）。**読取側契約の移動は
  不要になった**ので U3 の成果物への参照更新は生じない。
  (−) unit-of-work.md の U4 分類（`embedded` → 独立クレート）に改訂が要る。
- **Alternatives Rejected** — *モジュール分割だけ*（同一クレート内で `mod` を分ける）: `pub(crate)`
  で相互参照できてしまい、物理強制にならない。本プロジェクトが依存強制に採っている唯一の
  機構（クレート分離）を CQRS 境界にだけ適用しない理由がない。
  *アダプタも分割*（`EventStoreImpl` / `JournalReaderImpl` を別クレート）: 境界はより厳密に
  なるが、C6 の 3 表のスキーマ定義をどちらが持つかの判断が増える。複雑さに見合わないと判断
  （オーナー裁定）。
  *読取側契約を中立クレート `core/event-stream` へ切り出す*（本 ADR の初稿）: RMU がコマンド側に
  依存してよいので**不要**。中立クレートは「両側が相手を知らずに同じ契約を共有する」ための
  仕掛けであり、**橋が両側を知ってよい構図では役目が無い**（オーナー訂正 2026-08-24）。

## ADR-010: event-store-adapter-rs v2.0.0 へ乗り換える — ADR-006 の見送りを撤回（2026-08-26 追加）

- **Context** — ADR-006 は crate への直接依存を見送り、trait 群を**本家と同形でローカル定義**して
  「本家が **feature 化 / SQLite 実装**を得たら乗り換え可能な形を保つ」と決めた。
  **2026-08-24 公開の v2.0.0 で、その 2 条件が両方とも満たされた**（実測）:
  - `gate backends behind cargo features with empty default` — `lib/Cargo.toml` は
    `default = []`、`dynamodb` / `bigtable` / `sqlite` / `sqlite-system` が feature。
    ADR-006 が見送り理由に挙げた「aws-sdk-dynamodb + tonic + Bigtable の feature ゲート無し
    ハード依存」は**消滅した**
  - `add SQLite-backed event store behind sqlite feature` — `sqlite = ["dep:rusqlite",
    "rusqlite/bundled"]`。**我々が委任 3 で自前実装したのと同じ rusqlite**
  - `replace SDK-leaked error type with neutral OptimisticLockError(String)` — エラー型も中立化

  あわせて、**ローカル定義の写しが本家からずれていた**ことも判明した（4 点: 関連型 vs 型
  パラメータ / `usize` vs `u64` / エラー型 1 種 vs 2 種 / `Clone` 境界の有無）。特に
  `usize` → `u64` は、**我々のドメイン型に合わせて借り物の契約を書き換えた**もので、
  [`coding-rules/upstream-contracts.md`](../../../knowledge/aidlc-shared/coding-rules/upstream-contracts.md)
  違反である（契約の所有者は本家であり、我々ではない）。

- **Decision**（オーナー明言 2026-08-26「乗り換えてほしい。v2.0.0に」「腐敗防止層はなしで。
  ちゃんと書き換えろ」）—

  1. **`event-store-adapter-rs` v2.0.0 に `sqlite` feature で依存する**。ADR-006 の見送りを撤回。
  2. **Conformist を採る。腐敗防止層は置かない。** 我々のドメイン型が本家の trait を**直接実装**
     する。アダプタ型を挟んで変換する案はオーナー裁定で却下（儀式が増えるだけ）。
  3. したがって次を受け入れる:
     - ドメイン型に **serde の `Serialize` / `Deserialize`** を入れる
     - **`chrono::DateTime<Utc>`** を採る（`occurred_at` / `last_updated_at`）。
       **NFR4.1（依存最小化）の再検討が要る** — 自前 ISO 8601 整形の存在意義が変わる
     - `seq_nr` / `version` を **`usize`** にする（本家の契約に従う。`u64` への「具体化」は撤回）
  4. **本家に無いものは我々が持ち続ける** — 本家のドメインは「集約の永続化」であり、
     次は利用側の関心である:
     - **投影チェックポイント**（`JournalReader::checkpoint` / `advance_checkpoint`）
     - **全集約横断の順序読取**（`events_after(GlobalSeqNr)`）— 本家は集約単位。
       **2026-08-26 オーナー裁定（ライブラリ所有者として）**: この責務は**ライブラリのサポート外**。
       AWS 版・GCP 版でも Streams / CDC はライブラリが提供するのではなく利用側が組む。
       本家への機能要望は検討のうえ**取り下げ**た。**SQLite を使う範疇で amadeus-ng が独自に実装する**:
       本家の `journal` 表（追記専用・書込直列化ゆえ rowid = コミット順の単調カーソル）を
       同一 DB ファイルへの別接続で読み、チェックポイントは自前の表
       （本家の表と衝突しない名前）に持つ。本家スキーマへの結合はバージョンの完全固定
       （`=2.0.0`）と、スキーマが変わったら明示的に落ちるガードテストで守る。

       **2026-08-28 追記（rowid と VACUUM — PR #30 レビュー指摘への裁定）**: `journal` に
       `INTEGER PRIMARY KEY` は無いため、SQLite の仕様上 VACUUM は rowid を振り直し得る。
       ただし振り直しが値を変えるのは行削除で隙間ができた場合だけであり、`journal` は
       削除ゼロの純追記（DELETE 文は本家 v2.0.0 / v3.0.0 とも snapshot 表にしか無い —
       実測）なので rowid は隙間の無い連番 1..N のまま、再構築後も同値に保たれる。この
       前提は回帰テスト `a_vacuum_rebuild_does_not_move_the_cursor` で実挙動に釘留めした。
       多層防御（チェックポイント表に (aid, seq_nr) アンカーを併記し、読取時に journal と
       照合して不一致を明示エラーにする）は、当初 B7（v3 乗り換え）へ送る計画だったが、
       CodeRabbit の再指摘を受けて**同 Bolt 内で前倒し導入した** —
       `amadeus_projection_checkpoint` へ anchor_aid / anchor_seq_nr 列を追加し、
       `advance_checkpoint` が前進先 journal 行の識別子を記録、読取が照合し、不一致は
       `Corrupt (CheckpointAnchorMismatch)` で明示拒否する。実測補足: 現行 SQLite 3.51 の
       VACUUM は隙間があっても rowid を保持する（釘留めテストで確認）。仕様が許す
       振り直しは回帰テストで直接再現し、検出されることを証明済み。

       **2026-08-27 追記（supersede の明記）**: 上記の実装は ADR-003「Repository →
       `EventStoreImpl(sqlite client)` → SQLite」、ADR-007「チェックポイントは Tx 内更新」、
       ADR-009「`EventStoreImpl` が `EventStore`（コマンド）と `JournalReader`（読取）の両契約を
       実装し、C6 の 3 表定義が 1 箇所に残る」の各記述を supersede する。`EventStoreImpl` は
       削除され、コマンド側は `WorkflowExecutionRepositoryImpl<S>`（本家 `EventStore` を実装）、
       読取側は別接続を持つ `JournalReaderImpl` という別々の型になった。チェックポイントの更新は
       書込 Tx の外（`JournalReaderImpl` の別接続・別 Tx）で行われる。SQLite スキーマは `journal` /
       `snapshot` の 2 表が upstream 正本であり、我々が定義するのは `amadeus_projection_checkpoint`
       1 表のみである。
     - `within_write_transaction`（U7 の登録簿 read-modify-write）— **調査済み（2026-08-26）。
       本家は接続もトランザクションも露出しない**（`EventStoreForSqlite` は `Connection` を
       内部保持し、`from_connection` は private。`transaction()` は `persist_*` の内部でのみ
       使われる）。したがって**本家経由では BR2.4 を実現できない**。これは乗り換え Bolt が
       裁定すべき設計判断であり、選択肢は次の 3 つ:

       BR2.4 の意図は「`intents.json`（登録簿）の read-modify-write を SQLite の Tx で守る」
       である。ADR-007 でロック機構を退役させたため、**ファイルの排他が SQLite の Tx に
       依存している**構造になっている。

       | 案 | 中身 | 評価 |
       | --- | --- | --- |
       | (a) 別接続 | 登録簿用に**同じ DB ファイルへ 2 本目の接続**を開き、そちらで Tx を張る | SQLite のロックが直列化するので排他は成立する。本家の接続とは別 Tx になるので「イベント永続化と登録簿更新の原子性」は失われる — **その原子性が本当に要るのかを先に確かめる** |
       | (b) 登録簿を SQLite へ移す | `intents.json` をやめてジャーナルと同じ DB のテーブルにする | 原子性が自然に成立するが、`intents.json` は upstream 互換ファイル（リードモデル）なので D6 に触れる。RMU の投影対象にするのが筋 |
       | (c) 本家へ貢献 | 接続または Tx を露出する API を upstream へ提案する | ADR-006 が「上流貢献も選択肢」と既に書いている。stage-1 の最短経路からは外れる |

       **(b) が筋に見える** — `intents.json` はリードモデルであり（ADR-003/004）、リードモデルを
       コマンド側が Tx で守るという構造自体が CQRS の境界に反している
       （[`cqrs-boundaries.md`](../../../knowledge/aidlc-shared/coding-rules/cqrs-boundaries.md)）。
       ただし U7 の設計に踏み込むので、乗り換え Bolt で単独裁定せず U7 と併せて判断する。

- **Consequences** — (+) 自前実装 **約 2,400 行**（`event_store_impl.rs` 971 / `schema.rs` 179 /
  `event_store_impl_test.rs` 1,008 / ローカル `EventStore` trait 230）が消え、本家の保守に乗る。
  (+) 本家への合流・貢献が現実的になる。(+) 借り物の契約を曲げている状態が解消する。
  (−) ドメイン層に serde と chrono が入る（NFR4.1 の再検討）。(−) 集約・イベント・IntentId が
  本家 trait を実装するための改修（`Event::id` / `is_created` / `Aggregate::id` /
  `last_updated_at` / `AggregateId::type_name` の新設、`seq_nr`/`version` の `usize` 化）。
  (−) B5 でマージするコードの一部を次 Bolt で削除することになる（オーナー裁定で許容）。
  (±) Quint モデル `journal_protocol` の検証対象が「自前実装の契約」から「本家の契約 +
  我々の投影」へ移る — モデルの再確認が要る。

- **追記 2026-08-27（実装後の裁定 3 件）** — (1) **genesis の初期 version は Gateway が写しに
  1 を載せる**（オーナー承認。「version はストア側が決める」の自然な帰結 — Gateway はストア側の
  部品であり、ドメインは version を解釈しない。集約を version=1 で作る案はストアの採番規則を
  ドメインへ戻すため却下、集約 0 のまま渡す案は Quint の `version_equals_journal` を破るため
  却下）。(2) `Conflict` の `actual` は競合時のみ `get_latest_snapshot_by_id` の読み直しで得る
  （本家の公開 API にだけ結合。構造化エラーの上流提案は候補として記録）。(3) `busy_timeout` は
  本家接続に設定不可 — 単一プロセス前提の現状は受容し、U7 の並行モデルと併せて再裁定。
- **Alternatives Rejected** — *腐敗防止層*: ドメインを serde / chrono / `usize` から守れるが、
  アダプタ型と変換で数百行を足すことになる。オーナー裁定「腐敗防止層はなしで。ちゃんと書き換えろ」。
  *自前実装を維持*: ADR-006 が保とうとした乗り換え可能性という目的を捨てることになり、
  かつ本家と同じものを二重に保守する。*B5 のマージ前に作り直す*: B5 は既に 159 ファイルで
  人間レビューが困難な大きさに達しており、乗り換えを足すと追えなくなる。**独立した Bolt** として
  「本家に置き換えた」という一筋で読める差分にする（オーナー裁定 2026-08-26「次の Bolt でいい」）。

- **追記 2026-08-29（Bolt B7 — v3.0.0 乗り換え）** — 本家 v3.0.0（2026-08-28 リリース）は、
  要望書 [`upstream-request-esa-event-envelope.md`](../../upstream-request-esa-event-envelope.md)
  の 4 設計質問すべてに回答する形で `Event` / `Aggregate` trait を廃し、
  `EventEnvelope<AID, P>` / `SnapshotEnvelope<A>` に置き換えた。B7 で `=3.0.0` へ乗り換えた
  （`=2.0.0` は失効）。

  ドメイン型が実装する本家 trait は `AggregateId` だけになり、輸送のメタデータ（集約識別子・
  通番・発生時刻・型判別子）は封筒が運ぶ。旧封筒 struct `WorkflowExecutionEvent`（id /
  schema_version / occurred_at フィールド）と `WorkflowExecutionEventId` 型は削除し、
  ドメインイベントは輸送メタデータを一切持たない素の serde 型（本家の語で payload）になった。
  旧 `schema_version` 予約フィールドの後継はジャーナル列の manifest 列（値は
  `workflow-execution-event/1`）で、Repository が書き、JournalReaderImpl が不一致・欠落を
  `Corrupt(UndecodablePayload)` で拒否する（版を上げる規約は C5 参照）。

  楽観 `version` は集約と memento（`WorkflowExecutionState`）から削除し、**集約の外**を持ち回る
  形にした — `find_by_id` は再水和レコード `RehydratedWorkflowExecution`（集約 + ストア採番
  version）を返し、`store` は `expected_version: usize` を引数に取る。

  **経緯（TOCTOU）**: 初稿の「更新は `persist_event(envelope, snapshot.version())`」は、
  `store` の引数に version が無いため store 内で最新スナップショットを読み直す形にしかならず、
  TOCTOU で楽観ロックが無効化する（memory バックエンドには `(aid, seq_nr)` 一意制約が無く黙って
  二重書込になる）ため撤回し、本家移行ガイド §3 の持ち回り形へ確定した。

  **更新も `persist_event_and_snapshot`** を使う — v3 の `persist_event` は snapshot 行の
  seq_nr を進めないため、Quint モデル不変条件 `snapshot_tracks_journal`（snapSeq ==
  journalLen）を破る。genesis / 更新の分岐は `event.seq_nr == 1` から導出する（`is_created` の
  消滅に整合、本家 v3 と同型）。genesis の `set_version(1)` ハックは `FIRST_STORED_VERSION`
  定数ごと消滅した。`expected_version` は newtype 化を見送り `usize` のまま受け入れる（不透明
  トークンの旨はポート doc に明記。newtype 化は U5/U6 実装時の境界強化候補として記録済み）。

  出典: [`developer-report-1.md`](../../construction/esa-v3-migration/developer-report-1.md)
  （§2 裁定 1〜9、§4-(a) TOCTOU 経緯、§4-(b) newtype 見送り）。
