# B11 設計裁定と申し送り（2026-08-29 セッション）

本 Bolt（U5 = report ユースケース、FR2.1 のみ）に先立ってオーナーが下した裁定と、
その過程で見つかった未処理事項の記録。裁定はブリーフ `brief-1.md` の「固定裁定」に
反映済みで、本ファイルは**根拠と経緯**を残すためのもの。

## 確定した裁定

### 裁定 1 — 投影キャッチアップの起動は合成ルート（U7）

**問題**: 上流成果物どうしが矛盾していた。

- `unit-of-work.md` の U5 責務: 「… → store → **投影キャッチアップ起動**」
- `unit-of-work-dependency.md`: `U5 → U4` の依存辺（理由「コマンド末尾の投影キャッチアップ起動」）
- 一方 `coding-rules/cqrs-boundaries.md`（2026-08-29 オーナー裁定・**クレート分離で機械強制**）:
  「コマンド側・クエリ側が RMU を `Cargo.toml` に書く」は禁止パターン、「合成ルート（U7）は
  RMU を**起動するだけ**」
- `unit-of-work.md` の U7 責務にも「コマンド末尾の ReadModelUpdater 起動」が重複記載

**裁定**: 起動するのは **CLI の起動部分（合成ルート U7）**。U5 は RMU に依存しない。

**根拠の補足**: 本プロジェクトは常駐プロセスを持たず、`aidlc report ...` は 1 回走って終わる
CLI である。AWS 版なら DynamoDB Streams が RMU を非同期に叩くが、SQLite にストリームは
無いので、同一プロセス内で誰かが `catch_up()` を明示的に呼ばないと最新化が起きない
（`cqrs-boundaries.md`「SQLite にはストリームが無いので、AWS 版 RMU が Streams から
受信するのと同じ役割を、ここでは自分で引く形で果たす」）。その「引く」きっかけを誰が出すか、
が論点だった。**RMU が最新化を行うこと自体は論点ではない。**

**反映**: `unit-of-work.md` の U5 責務文と `unit-of-work-dependency.md` の
mermaid 辺・text fallback・辺の根拠表を、いずれも失効注記付きで in-place 修正済み。

### 裁定 2 — フェーズ境界は集約が内部導出する

**問題**: `workflow_execution.rs:532` は「`phase_boundary` は**呼出側が導出**して渡す投影材料
（C5）」として `approve_gate` の引数にしている。一方オーナー統一ルール（project.md
Corrections 2026-08-22）は「集約は FSM。判断（クエリメソッド）を集約型に閉じ込め、
ユースケースは進行管理・フロー制御のみ（**ビジネスロジック禁止**）」。
「この承認がフェーズ境界をまたぐか」は集約状態からの判断であり、現状のままだと U5 に
ビジネスロジックが入る。

**裁定**: **集約が自分で導出する**。`approve_gate` の `phase_boundary` 引数を廃止。

**オーナー補足（重要）**: これは**本家仕様からの逸脱ではなく、内部の実装方法の選択**である。
したがって外形は 1 バイトも変えない — `GateApproved` の payload 形（`phase_boundary` を持つ）、
監査シャードのバイト列、`aidlc-state.md` の差分はすべて現状どおり。ゴールデン
（`cli/report/approved-across-phases` 等）と RMU の投影ゴールデンテストで機械的に固定する。

### 裁定 3 — `StoreVersion` newtype 化は却下

**問題**: B7 の申し送りに「楽観 version の newtype 化は U5/U6 の境界強化候補」とあった
（`workflow_execution_repository.rs` の trait doc）。

**裁定**: **却下**。楽観ロックの判定そのものは本家 `event-store-adapter-rs` v3.0.0 が行っており
（`EventStore::persist_event_and_snapshot(..., expected_version: usize)`、失敗時は
`optimistic lock failed, aid=..., expected_version=...`）、`usize` はその本家が定めた語彙である。
こちら側で専用型の衣を着せるのは Conformist 方針（`=3.0.0` ピン・腐敗防止層なし、
`coding-rules/upstream-contracts.md`）に反する。`usize` のままポートを往復させる。

**残るリスク**: `version` と `seq_nr` がどちらも `usize` なので型では取り違えを止められない。
緩和はポート doc の 3 か条（ストア採番のトークン・`seq_nr` と混同しない・集約へ入れない）と
テストに委ねる。

### 裁定 4 — 用語「再水和」を使わない

新しく書く日本語散文では「再水和」を使わず「**再構成**」（= 保存済みイベントを古い順に再生して
現在の状態を組み立て直すこと）と書く。既存 48 ファイルと Rust 型名
`RehydratedWorkflowExecution` の一括置換は**別 PR**（差分が大きく U5 と混ざるため）。

### 裁定 5 — 本 Bolt は FR2.1 のみ

U5 は FR2.1（遷移コミット）と FR2.2（レシート述語 + verification 面）からなるが、
**本 Bolt は FR2.1 のみ**。FR2.2 は次の Bolt。理由は下記「FR2.2 の設計上の衝突」。

## 申し送り（未処理）

### (a) FR2.2 の設計上の衝突 — 次の Bolt で裁定が要る

upstream の `hasFreshPracticesAffirmationReceipt`（`aidlc-orchestrate.ts:4749`）は、ゲート承認を
受理してよいかの判定のために **`aidlc-state.md` の `Practices Affirmed Timestamp` と
`readAllAuditShards()` による全監査シャード**を読んでいる。しかし `cqrs-boundaries.md` の
禁止パターン先頭は「**コマンド側のユースケースがリードモデル（状態ファイル・監査シャード）を
読んで判断する**」であり、U5 が直接読むことはできない。

**見えている回避策**（`use-case-rules.md` §4 の I8「Controller が読んで `&` で渡す」と同型。
**未裁定**）:

1. 読取と新鮮判定は **RMU 側のクエリ関数**に置く（`read_all_audit_shards` は
   `modules/core/read-model-updater/src/workspace/audit_shard.rs` に既存）。実ロジックを
   カバレッジ除外領域（合成ルート）に落とさないため。
2. **U7 がそれを呼び**、結果を型付きの事実として U5 へ渡す。
3. **判定そのものは集約**が行う（`approve_gate` が事実を受け取り、そのステージで必要かを
   自分で決めて拒否する）。

**用語の未確定**: FR2.2 の「**B10 述語**」が何を指すかは文書内に定義が無い。上記のレシート
新鮮判定と読んだのは**推定**である（Issue #7 の 3-B「report_dispatch ＋ B10 述語最小 ＋
verification モジュール」と、同 Issue 項目 3 の「レビュアーレシート述語」から）。次の Bolt の
着手前にオーナー確認が要る。

### (b) `CorruptCause` がユースケース層にあること — **一旦許容**（オーナー裁定 2026-08-29）

`modules/core/command/use-case/src/orchestration/corrupt_cause.rs` は、バリアントがすべて
永続化機構の語彙（`MissingSnapshot` / `UndecodablePayload` / `SequenceGap` /
`InvariantViolation`）でありながら、フロー制御の層に置かれている。実測すると:

- 構築するのはアダプタ実装のみ（10 箇所）
- **分岐に使う消費者はゼロ**（`match` している箇所が無い）
- 参照は `RepositoryError` の定義とアダプタのテストの等値アサートのみ
- doc 自身が「RMU の同名型と**同じ名前の別の型**」と認めており、同じ分類が 2 箇所に複製されている

置かれている経緯自体は筋が通る（DIP でポート trait がユースケース層にある以上、その戻り値の
`RepositoryError` も同層にあり、材料の `CorruptCause` も引きずられた）。

**裁定の分かれ目**: 将来 U7 が原因別に違う文言・復旧案内を出すなら契約に残す価値がある。
出さないなら `RepositoryError::Corrupt` を材料なしにして分類をアダプタ内部へ畳むのが素直。
なお upstream に ES ストアは無いので、この分類は本家仕様には存在しないこちら独自の追加である。

**現時点の扱い**: オーナー裁定により**一旦許容**（現状維持）。U7 の文言設計が固まった時点で
再評価する。

### (c) 投影キャッチアップのクラッシュ窓（U4 既知事項・受容済み）

`updater.rs:150-152` が明記しているとおり、リードモデルを書いた直後・チェックポイント前進の
直前に落ちると、次回に同じ差分を再投影する。状態ファイルは冪等なので同じ位置に落ち着くが、
**監査シャードには同じブロックがもう一度並ぶ**。「欠落しない」ことと引き換えに受容している。

チェックポイントの置き場所（SQLite の `amadeus_projection_checkpoint` 表）は
**オーナー裁定により現状維持**（2026-08-29「適切な場所に最新のチェックポイントが保存されて
いればいい」）。塞ぐには監査シャード自身が「どこまで書いたか」を語れる必要があるが、
監査シャードのバイト列は upstream 逐語互換のゴールデンで固定されており目印を足せない。
**「どう実現するか」自体が未解決の設計課題**である。

## 追補（実装着手後に判明した事項・2026-08-29）

### 訂正 — ブリーフ初版の `Conflict` 再試行の記述は誤りだった

初版ブリーフは固定裁定として「再試行の政策は持たない（`Conflict` も再試行しない — ポート doc
C3 ③）」と書いていた。これは**メインセッションによる C3 ③ の誤読**であり、委任先の指摘を受けて
正本を確認して撤回した。

正本は `inception/contract-design/contract-design-questions.md` の **Q6 = A**（オーナーが
`[Answer]: Looks correct` で承認済み）:

> 楽観 version 競合は即 `Err`（**ユースケースが 1 回だけ再水和して再試行**、それでも競合なら
> CLI がエラー終了。ワンショット CLI で同時実行は稀）

C3 ③「`Conflict` **以外**のエラーはリトライしない」は「Conflict **だけ**が再試行の対象であり、
その政策はユースケースが持つ」の意である。したがって:

- `repository_error.rs` の `Conflict` doc（「ユースケースは再水和して 1 回だけ再試行する」）は
  **正しい**。変更しない。
- `ReportUseCase` は `store` の `Conflict` に対し、**再構成（`find_by_id`）からやり直して**
  1 回だけ再実行する。2 回目も `Conflict` なら伝播。再試行後に集約コマンドが `Err`（別クローンが
  先に承認しゲートが閉じた等）を返す場合も伝播。

**教訓**: 委任ブリーフに固定裁定として書く前に、根拠として引く 1 次資料（ここでは C3 の条項）を
**当たり直すこと**。要約（trait doc の 1 行）を根拠に裁定を書くと、要約の圧縮で意味が反転しうる。

### 所有ファイル規律の例外承認

裁定 2 で `approve_gate` の引数が 1 つ減るため、所有ツリー外の呼出側がコンパイル不能になる
（`no-backward-compatibility.md` により互換オーバーロードは残さない）。`approve_gate` の呼出は
**全 7 箇所**（委任先の初報は 5 箇所で、所有ツリー内の domain tests 2 箇所が漏れていた）。
すべてリテラル `None` を渡しているだけなので、**`None,` の削除に限り**所有ツリー外 5 箇所の
編集を承認した。詳細は `brief-1.md` の「所有ファイル・規律」を参照。

### C3 ④ とコーディング規則の緊張（記録のみ・是正不要）

契約 C3 ④（2026-08-27 / ADR-010 改訂）は「テストダブル型は無く、テストは
`XxxUseCase<WorkflowExecutionRepositoryImpl<…>>` で組む」と定めるが、これを use-case クレート内の
テストで literal に満たすには `core-command-interface-adapter` を dev-dependency に足す必要があり、
`use-case-rules.md` §1 の機械強制（「`core-use-case` の `Cargo.toml` に `core-interface-adapter` が
無いこと」）が壊れる。加えて依存が循環する。

**採った解**: 網羅テストは use-case クレート内のスクリプト可能な fake で持ち（`Conflict` を意図的に
起こすにはどのみち fake が要る）、C3 ④ は `interface-adapter/tests/` に結線テスト 1 本を置いて
満たす。両方の要求が同時に満たされるため、C3 ④ の改訂は不要と判断した。
