# units-generation-questions — Unit 分割計画の確認質問

> Units Generation（Inception 2.7）の質問票。出典: `../domain-design/components.md`（11 コンポーネント）、
> `../domain-design/decisions.md`（ADR-001〜007、ES 設計）、`../requirements-analysis/requirements.md`
> （FR1〜FR9 / NFR1〜NFR5）、`../practices-discovery/team-practices.md`（Bolt = PR、直列運用、
> squash-merge）、`user-stories` はスキップ済み（`../user-stories/user-stories-assessment.md`）のため
> トレーサビリティは **FR → Unit** で取る。
>
> 本ステージが決めるのは「何を Unit（まとまった作業単位）にし、Unit 同士がどう依存するか」という
> **形（DAG）**だけである。どの Unit から着手するか・クリティカルパスはどれかは次の delivery-planning
> （2.9）が決める。
>
> **前提注記（矛盾の検出）**: `requirements.md` FR1.2 は「audit-first 遷移をロック区間（既存
> `WorkspaceLock`/`LockProtocol`）と結合する。合格 = `audit_lock.qnt` ITF 準拠維持」と書いているが、
> domain-design の ADR-007 は mkdir ロック機構を退役し SQLite Tx + 楽観 version に置換、
> `audit_lock.qnt` は改訂すると裁定した（ADR-007 自身が「FR1.1/FR1.2 の合格基準文言の改訂が必要」と
> 注記）。Unit 割当に影響するため Q9 で扱いを確認する。

---

## Q1. Unit の境界戦略 — 何を軸に Unit を切るか

コンポーネント 11 個・FR 9 群・設計監査の束 A〜D という 3 つの切り口がある。

- A. 要求（FR）起点 — FR 群をそのまま Unit にする（FR1〜FR9 ≒ 9 Unit 前後）。Bolt = PR がそのまま要求単位になる
- B. コンポーネント起点 — components.md の 11 コンポーネントをクレート境界で束ねる（ドメイン / ユースケース / ゲートウェイ+投影 / CLI+ハーネス / 共有部品）
- C. ハイブリッド（推奨）— 縦串（ユースケース・CLI・フック・doctor）は FR 起点、横断の基盤（ES ストア・投影・canon-json）と非コード作業（docs 正本修正・CI）は作業種別起点で独立 Unit 化する
- D. 設計監査の束（A〜D 束）起点 — 監査修正束をそのまま Unit にし、新規実装を別途足す
- X. Other (please specify)

[Answer]: C

## Q2. Unit の粒度 — 1 Unit の大きさ

Bolt = PR の直列運用（オープン PR は常に 1 本、数時間〜1 日でマージ）なので、粒度はそのまま PR の大きさになる。

- A. 粗め（4〜6 Unit）— 1 Unit = 複数 FR。PR は大きくレビュー負荷が高いが回数は少ない
- B. 中（7〜10 Unit）（推奨）— FR 単位前後。1 PR が数時間〜1 日に収まる大きさで、直列運用と噛み合う
- C. 細め（11 Unit 以上）— FR サブ項目単位。PR は小さいが回数が多くオーバーヘッドが増える
- X. Other (please specify)

[Answer]: B

## Q3. 非コード作業（FR8 docs 正本修正・FR9 CI/ガバナンス）の扱い

- A. それぞれ独立 Unit（FR8 = spec、FR9 = packaging）（推奨）— コード Unit と混ぜず PR も別。設計成果物は最小で済む
- B. 関連するコード Unit に同梱 — 例: FR8.1 の canon 修正と FR8.3/8.4 の PlanAction 移動・畳み込み移設を同じ Unit にする
- C. FR8 は独立 Unit、FR9 は Unit 化せず code-generation 後の build-and-test / ci-pipeline ステージで扱う
- X. Other (please specify)

[Answer]: A

## Q4. FR7（canon-json 実装 + 0b ゴールデン採取）の配置

FR3（continue_token）・FR4（CLI 出力ゴールデン）・FR5（フック ゴールデン）が FR7 に依存する。

- A. 独立の基盤 Unit 1 つ（ゴールデン採取 FR7.1/7.2 とクレート実装 FR7.3 を同居）（推奨）
- B. ゴールデン採取（FR7.1/7.2 — upstream ツールを bun で動かす採取作業）とクレート実装（FR7.3）を別 Unit に分ける
- C. FR3（next/continue）の Unit に吸収する
- X. Other (please specify)

[Answer]: A

## Q5. ES 基盤（ADR-001/003/007）の Unit 化 — SQLite EventStore・Repository 実装・ロック退役・投影（RMU）

- A. 「ES ストア + WorkflowExecutionRepositoryImpl + ロック退役」で 1 Unit、「ReadModelUpdater（状態ファイル・監査シャードへの投影）」で別 1 Unit（推奨）— 書く側と描く側は変更理由が別
- B. 全部まとめて 1 つの基盤 Unit
- C. 独立 Unit にせず、FR1（監査台帳）と FR2（report）の Unit に分散して吸収する
- X. Other (please specify)

[Answer]: A

## Q6. Unit 間の依存の表し方（順序は 2.9 が決める。ここでは DAG の形だけ）

- A. 厳密な依存だけを辺にする（推奨）— 「無いとコンパイル/テストできない」依存のみ。依存の無い Unit の集合（並列可能な組）は明示する（PR は直列でも、どれを先に出すかの選択肢として 2.9 に渡す）
- B. 望ましい順序も依存辺として入れる（保守的 — DAG は一本道に近くなり、並列機会は表現しない）
- X. Other (please specify)

[Answer]: A

## Q7. contract-design（2.8）で形式化すべき Unit 間の境界 (select all that apply)

Unit 同士が並行に進められるよう、境界の契約を次ステージで文書化する。どれを対象にするか。

- A. ポート trait — `WorkflowExecutionRepository`（store / find_by_id）・`WorkflowDefinitionRepository`（find）・event-store-adapter-rs 同形の EventStore trait
- B. ドメインイベント語彙（`WorkflowExecutionEvent` 11 変種程度）と RMU の投影規則（1 イベント → 監査行 N 行・状態ファイル差分）
- C. SQLite スキーマ（journal / snapshot / checkpoint テーブル）
- D. CLI 動詞・directive JSON・フック 4 本の入出力（upstream 互換面 — ゴールデンで固定される契約）
- E. 上記すべて
- X. Other (please specify)

[Answer]: A, B, C, D

## Q8. Unit の kind（種別）タグ — 構築フェーズでどの設計成果物を書くかを決める

kind は service（実行可能物）/ spec（契約・文書）/ ui / packaging（ビルド・配布）/ library（単独では動かないコード）の 5 種。

- A. ドメイン・ユースケース・ゲートウェイ・投影・canon-json の Unit は library、CLI バイナリ+フック+doctor の Unit は service、docs 正本修正は spec、CI/ガバナンスは packaging（推奨）
- B. コードの Unit はすべて service（単一バイナリに埋め込まれるため区別しない）
- C. タグ無し（全 Unit にフル設計成果物マトリクスを適用する）
- X. Other (please specify)

[Answer]: A

## Q9. FR1.2 と ADR-007 の矛盾（前提注記）の扱い

- A. FR1.2 を ADR-007 の内容で読み替えて Unit に割り当てる（推奨）— 「audit-first を SQLite Tx + 楽観 version と結合、合格 = 改訂版 `audit_lock.qnt` の ITF 準拠」。`requirements.md` 自体の文言改訂は後方ジャンプで別途行う（units-generation の成果物には読み替えを明記）
- B. 今ここで requirements-analysis へ後方ジャンプし、FR1.1/FR1.2 を改訂してから units-generation をやり直す
- C. FR1.2 を本 intent から外す（ロック退役で要求自体が消滅したとみなす）
- X. Other (please specify)

[Answer]: B（「いやー改訂しないとまずくないか。」→ Q9a で確定）

## Q9a. Q9 追問 — requirements.md 改訂の影響分析と実施タイミング

Q9 への回答「いやー改訂しないとまずくないか。」（Other）を受けた追問。影響分析（後方ジャンプで
requirements-analysis に戻った場合に改訂する箇所）:

- FR1.1 — 監査シャード（`<record>/audit/<host>-<clone>.md`）は RMU の投影（リードモデル）、
  台帳本体は SQLite ジャーナル。合格基準（逐語契約との一致テスト）は維持
- FR1.2 — 「ロック区間との結合」→「SQLite Tx + 楽観 version との結合」。合格 = 改訂版
  `audit_lock.qnt` の ITF 準拠
- FR1.3 — `AuditLedgerRepository` を廃止し `WorkflowExecutionRepository`（ES: store / find_by_id）へ。
  O2（AuditLedger の位置づけ）は ADR-001/003 で裁定済みとして close
- NFR3 — 「監査台帳から再構成」→「ジャーナルから再構成、互換ファイルは投影で再生成」
- §7 O1（next_decision 配置）/ O3（StateFile 所有）— ADR-002 / ADR-004 で解決済みとして close
- NFR1 の逸脱台帳件数注記 — ADR-003/007 の逸脱登録（SQLite ファイル・ロック dir 非生成）を反映

戻った場合の流れ: requirements-analysis（Modify → 製品リード再レビュー → 再承認）→ user-stories /
refined-mockups（再び skip 判定）→ domain-design（Keep）→ units-generation（本ファイルの回答を引き継ぎ
確認のうえ再開）。

- A. 今すぐ戻って改訂する（推奨）— 上記を改訂してから本ステージを再開する
- B. このステージを終えてから戻る — Q9 の A で割り当てて承認後に後方ジャンプ（units-generation は
  結局再実行対象になるため二度手間）
- C. 改訂は不要 — Q9 の A（読み替え）で進む
- X. Other (please specify)

[Answer]: A

## Consolidated Summary Confirmation

requirements-analysis への後方ジャンプ（Q9/Q9a）を完了し、改訂版 `requirements.md` と domain-design（ADR-005 完全移動
改訂を含む）を入力として再開。Q1〜Q8 の回答は変更なし。

- Unit の境界は**ハイブリッド**（Q1 = C）: 縦串（ユースケース・CLI・フック・doctor）は FR 起点、横断の基盤（ES ストア・投影・canon-json）と非コード作業（docs 正本修正・CI）は作業種別起点で独立 Unit
- 粒度は**中（7〜10 Unit）**（Q2 = B）— Bolt = PR 直列運用で 1 PR が数時間〜1 日に収まる大きさ
- 非コード作業は独立 Unit（Q3 = A）: FR8 docs 正本修正 = spec、FR9 CI/ガバナンス = packaging
- FR7（canon-json 実装 + 0b ゴールデン採取）は独立の基盤 Unit 1 つ（Q4 = A）
- ES 基盤は 2 Unit（Q5 = A）: 「SQLite EventStore + WorkflowExecutionRepositoryImpl + ロック退役」と「ReadModelUpdater（投影）」
- 依存辺は**厳密な依存のみ**（Q6 = A）— 「無いとコンパイル/テストできない」関係だけを辺にし、並列可能な Unit の組を明示。順序は 2.9 が決める
- contract-design（2.8）で形式化する境界（Q7 = A,B,C,D）: ポート trait / ドメインイベント語彙と投影規則 / SQLite スキーマ / CLI 動詞・directive JSON・フック入出力
- kind は役割ごとに使い分け（Q8 = A）: コード Unit は library、CLI バイナリ+フック+doctor は service、docs 正本修正は spec、CI/ガバナンスは packaging
- FR1.2 と ADR-007 の矛盾（Q9）は後方ジャンプで requirements.md を改訂して解消済み（Q9a = A「今すぐ戻って改訂する」を実施）

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct
