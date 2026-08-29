> **⚠️ 2026-08-30 裁定バナー（Bolt B13）**: 本書に書かれた intent の再構成設計
> （`from_material` / 6 引数 `create` / `Created` の集約埋め込み / `IntentRepositoryError` /
> `IntentExecutionSnapshot`）は**オーナー裁定で置換済み**。現行の正は
> `coding-rules/aggregate-commands.md`「再構成の形」・`factory-naming.md`「集約の基本コンストラクタ」・
> `error-handling.md`「Repository エラーはジェネリック 1 本」。本文は歴史記録として残す。

# B12 委任ブリーフ 1 — 集約 WorkflowExecution → Intent の改名

Conversation language: 日本語
委任先モデル: Opus（機械的改名だが範囲が広く、改名対象外との切り分け判断を含む）
最終責任: Fable 5 メインセッション（全 diff レビュー・検証の独立再実行・受入判定）

## 裁定（オーナー確定 2026-08-29。変更禁止）

**集約 `WorkflowExecution` を `Intent` に改名する。** 根拠:

1. intent の生きた状態（ステージ位置・承認・park・スコープ・進行）を持ち、遷移し、判断する
   のはこの集約そのものであり、ドメインの一次名詞「intent」の実体はこの集約である。
   `intents.json` の行 `{uuid, slug, dirName}` は一覧用の索引にすぎない。
2. 監査語彙 `WORKFLOW_*` はリードモデルのバイト列であり、リードモデルは要件の違う別データ
   （オーナー裁定）。その語彙は集約の名前を縛らない。
3. 「workflow」の名は定義側（`WorkflowDefinition` = ステージグラフ + スコープグリッド）の
   持ち物。現状は workflow の語が定義と実行の二重語義になっており、`Intent` が
   `WorkflowDefinition` を参照して進む、と直せば語義が一対一に戻る。
4. これにより `IntentId` は Entity + Id 法則（ID は必ずエンティティ名 + Id）にそのまま適合する。

## 改名の一族（すべて同時・機械的）

| 旧 | 新 |
|---|---|
| `WorkflowExecution`（集約） | `Intent` |
| `WorkflowExecutionEvent` | `IntentEvent` |
| `WorkflowExecutionState` | **`IntentSnapshot`**（訂正 2026-08-29 — 状態を担うのは集約であり「State」を名乗らせない。正体はスナップショットの直列化形（memento）。あわせて**クレート内私有へ降格**— ドメイン外から使う箇所ゼロを実測済み。serde の `into`/`try_from` 文字列パスも追随） |
| `WorkflowExecutionStateBuilder` | **`IntentSnapshotBuilder`**（オーナー確定 2026-08-29 — 名前が示すとおり組むのは `IntentSnapshot`。`build()` は従来どおりスナップショットを返し、検査は `from_state` / `TryFrom` 側が担う。可視性はスナップショットと同じくクレート内私有） |
| `WorkflowExecutionRepository`（ポート） | `IntentRepository`（gateway-taxonomy「集約名 + Repository」に自動追従） |
| `WorkflowExecutionRepositoryImpl` | `IntentRepositoryImpl` |
| `InMemoryWorkflowExecutionRepository`（テスト fake） | `InMemoryIntentRepository` |
| `RehydratedWorkflowExecution` | `RehydratedIntent` |
| `AGGREGATE_TYPE_NAME = "WorkflowExecution"`（本家 trait `type_name`） | `"Intent"` |
| ファイル名 `workflow_execution*.rs` | `intent*.rs` 系へ（`git mv` を使う） |

- 集約内のフィールド `intent_id` とアクセサ `intent_id()` は **`id` / `id()` へ**（`Intent { intent_id }` は冗長。
  スナップショットの serde フィールド名が変わるが、ジャーナル・スナップショットはクローンごとの
  使い捨てランタイム＝gitignore 済みで互換問題なし。`no-backward-compatibility.md` どおり互換シムも置かない）。
- `RepositoryError::NotFound { intent_id }` は「探した intent の id」という材料名なので**そのままでよい**。
- `IntentId` は**改名しない**（正しい名前だったのはこちら）。`intent_id.rs` の見出し doc
  「集約 `WorkflowExecution` の識別子」は「集約 `Intent` の識別子（`intents.json` の uuid・
  記録ディレクトリの id8）」へ是正。
- doc コメント内の「集約 WorkflowExecution」「再構成される WorkflowExecution」等の散文も全て追随。

## 改名対象外（触ったら違反）

- **`WorkflowDefinition` 一族**（`WorkflowDefinitionId` / `WorkflowDefinitionRepository` / …）— workflow の名の正当な持ち主
- ドメインイベントの**変種名**（`Started` / `GateOpened` / `GateApproved` / …）と payload 構造
- `IntentId`・`IntentDirName`
- 監査語彙（`WORKFLOW_STARTED` 等）と投影出力 — **外形は 1 バイトも変えない**
- `tests/golden/**`（1 バイト不変）、`formal/**`（Quint モデルは Rust 型名を参照しない）、
  `docs/**`、`coding-rules/**`（正本の失効注記はメインセッションが実施）、
  `aidlc/**`（本ブリーフ・報告書を除く）、`.claude/**`

## 所有ファイル・規律

- 書いてよい: `modules/**`（上記対象のみ）、報告書
  `aidlc/spaces/default/intents/260822-stage1-selfhost/construction/intent-aggregate-rename/developer-report-1.md`
- `git add -A` 禁止（明示パス。`git mv` 後の状態確認を怠らない）。**push 禁止**。
  検証は `CARGO_TARGET_DIR=$PWD/target-delegate`。コミットは意味単位・日本語・`b12: ` 接頭辞。
- 固定フィクスチャ（ITF・ゴールデン・逐語アサート）に旧名や `"intent_id"` フィールドの
  **バイトが埋まっていた場合は止めて報告**（事前 grep では検出 0 件だが、発見したら独断で
  書き換えない）。

## 受入基準（すべて緑）

1〜7. B11 と同一（fmt / clippy / cargo lint / `cargo test --workspace` 退行 0 / quint / coverage 相対 / unwrap 0）
8. **外形不変**: `tests/golden/**` 差分 0、投影ゴールデン 19 本無改変で全緑
9. `grep -rn "WorkflowExecution" modules/ --include='*.rs'` が **`WorkflowDefinition` 文脈を除き 0 件**
   （`WorkflowExecution` の部分一致で `WorkflowDefinition` は引っかからないので実質 grep 0 件）
10. `git log --follow` でファイル履歴が追えること（`git mv` を使った証跡）
11. 報告書: 改名対応表の実測、フィクスチャ確認の結果、迷った点

## 補記 — 旧名の由来と、将来の再分割トリガー（オーナー考察 2026-08-29）

旧名 `WorkflowExecution` の意図は、おそらく「`WorkflowDefinition` とその実行」という対で
あり、「Intent を実行したときのランタイム文脈」を表したかったもの。ここで
`WorkflowDefinition` は「計画」ではなく**全 intent 共通のプロセス定義（カタログ）** —
stage-graph + scope-grid + scopes — である点に注意（オーナー指摘 2026-08-29 で言い直し）。
**静的な計画（この intent 向けに解決済みの EXECUTE/SKIP 列・scope・request）は `Started`
イベントが運び、実行時文脈（cursor・checkbox・park 等）ともども `Intent` 集約が持つ** —
「Intent = 静的な計画 + 実行時文脈」が正しい整理。recompose（計画の作り直し）も
`Recomposed` イベントとして Intent に起き、定義側は不変（`definition_revision` でピン）。静的な Intent +
実行時の WorkflowExecution という分割の絵に近いが、**静的側の Intent は一度もモデルとして
彫られなかった**（台帳行と IntentId だけが存在）— ID があるのにエンティティが無い混乱の根。

この分割が働きを持つのは「**1 つの Intent に複数の実行がありうる**」要件（再実行・リプラン
別走行など）が現れたときで、そのとき初めて `ExecutionId ≠ IntentId` となり両モデルが別々の
同一性を持つ。現システムは 1 intent = 1 実行・静的情報は `Started` イベントが運ぶため、
分割は代金だけで配当が無い。**`Intent` への統一は現要件では正しい形**であり、上記要件が
現れた時点が `Intent`（静的）/ `WorkflowExecution`（実行ごと）へ分け直すトリガーである。

---

# 改訂 2（2026-08-29・オーナー裁定）— 到達点の変更: Intent 構造体 + IntentExecution 集約

**本改訂は冒頭の「改名の一族」表を上書きする。** 補記に記した「将来の再分割トリガー」は
将来ではなく**現在の意味論**と裁定された — **「Intent を元に IntentExecution は何回も起きる」
（1 intent : n 実行）**。upstream の挙動とも符合する（resume メニューの start fresh・
`--single` の合成 ID 実行は同一 intent の別実行に相当）。

## 到達点

| 型 | 役割 |
|---|---|
| **`Intent`**（新設・構造体） | 静的な intent — `id: IntentId`・`request`（依頼文）・`scope`・解決済み `stages`・`definition_id`/`revision`・`scan`。**Always Valid の不変構造体**（コンストラクタで検査、以後変異なし）。集約ではない |
| **`IntentExecution`**（集約 — 旧 `WorkflowExecution` の実体） | 実行時文脈 — cursor・checkbox・overlay・approved・autonomy_mode・parked_at・revision_count・seq_nr、**+ 所有する `intent: Intent`** |
| **`IntentExecutionId`**（新設） | 実行自身の識別子。`IntentId` と同じ UUIDv7 正準形・Always Valid。Entity + Id 法則どおり |
| `IntentExecutionEvent`（旧 WorkflowExecutionEvent） | 変種名は不変。**`Started` の payload は `intent`（Intent 構造体を丸ごと運ぶ）+ 従来の実行属性で Intent に畳まれないもの** |
| `IntentExecutionSnapshot` / `IntentExecutionSnapshotBuilder` | 訂正済み方針（スナップショット・私有）のまま名前だけ追随 |
| `IntentExecutionRepository` / `Impl` / `InMemoryIntentExecutionRepository` | `find_by_id(&IntentExecutionId)` へ |
| `RehydratedIntentExecution` | 同上追随 |
| `type_name` | `"IntentExecution"` |

- **集約は 1 つのまま**（`Intent` は実行が所有する値であり、整合性境界は増えない）。
- genesis: **`IntentExecution::start(id: IntentExecutionId, intent: Intent, occurred_at) →
  (IntentExecution, Started)`**（オーナーの理想形 `IntentExecution::new(id, intent)` を家風の
  genesis 動詞 `start` に写した形。id は呼出側がミント — upstream がツール層で uuid を
  ミントするのと同じ）。旧 `start` の 7 引数は `Intent` に畳まれる。
- 計画の構造クエリ（slug → 位置・フェーズ参照など、状態に依らないもの）は `Intent` へ、
  状態依存の判断（ゲート可否・実効プラン = `intent.stages ⊕ overlay`・next_in_scope）は
  `IntentExecution` に残す。
- `IntentId` は不変（`Intent` 構造体の識別子として Entity + Id 法則を満たす）。
- ユースケース `CommitVerdictUseCase::execute` の識別子引数は `&IntentExecutionId` へ。

## 本改訂で決めない（申し送り — U6/U7 の設計点）

- **「この intent の現在の実行」の解決**（intent → 最新 execution の対応）。読み方の候補は
  リードモデル投影 or 台帳拡張だが、`intents.json` の行形式は upstream 互換面なので独断で
  フィールドを足さない。
- **「同一 intent の生きた実行は同時に 1 つ」の不変条件**。集約単体では張れない
  （集約横断）。置き場所は U7 以降で裁定。

## 受入基準の差し替え

- 基準 9 の grep 対象: `WorkflowExecution` が `WorkflowDefinition` 文脈を除き 0 件（変わらず）。
- 追加: `Intent` 構造体が変異メソッドを持たないこと（`&mut self` grep 0 件）、
  `IntentExecutionId` に IntentId と同等の形式検査テストがあること、
  genesis の新形（`start(id, intent, at)`）のテスト、`Started` payload に intent が載る
  ラウンドトリップテスト。
- 外形不変（ゴールデン・監査語彙・投影 19 本）は従来どおり**絶対**。

---

# 改訂 3（2026-08-29・オーナー裁定）— 集約は Intent を埋め込まず ID で参照する

改訂 2 の `IntentExecution` が `intent: Intent` を所有する形は**誤り**（集約設計の基本規則:
他の集約・エンティティは ID で参照し、オブジェクトを埋め込まない）。正しい形:

```rust
IntentExecution { id: IntentExecutionId, intent_id: IntentId, cursor, checkbox, overlay, ... }
```

## 帰結（改訂 2 を次のとおり修正）

1. **集約の保持状態**: `id` / `intent_id` / cursor / 実行時ベクトル（checkbox・overlay・
   approved・revision counts）/ autonomy_mode / parked_at / seq_nr **のみ**。
   `stages`・`scope`・`request`・`scan`・`definition_id`/`revision` は**保持しない**（Intent 側）。
   base の plan_action・conditional も保持しない — 実効プランは
   `intent.stages[i].plan_action ⊕ overlay[i]` で導出する。
2. **計画が要るコマンド・クエリは `&Intent` を引数で受ける**（家風 —
   `next_decision(&self, &WorkflowDefinition, ...)` と同じパラメータ渡し）。受け取り時に
   `intent.id() == self.intent_id` と `intent.stages.len() == self.checkbox.len()` をガードし、
   不一致は Err で拒否（取り違え防御）。
3. **`Started` イベントは intent の材料を丸ごと運んだまま**（改訂 2 のとおり）。理由:
   投影核の入力はイベントだけ（cqrs-boundaries 規則 3）で、状態ファイル描画に scope・stages・
   依頼文が必要。イベントは「開始時点の intent の事実記録」、集約が適用時に**保持する**のは
   `intent_id` と実行時状態だけ、という分離。genesis の `start(id, intent, at)` は intent から
   ベクトル長を採り、`intent_id` を控え、`Started { intent }` を返す。
4. **`CommitVerdictUseCase::execute` は `&Intent` も受け取る**（Controller が読んで渡す —
   I8 と同じ参照渡し。ユースケースは Intent の取得手段を持たない）。実 CLI で Intent を
   どこから読むか（台帳 or ジャーナルの Started）は U7 の設計点として申し送り。
5. スナップショット（`IntentExecutionSnapshot`）も同じ縮小に追随。

受入基準に追加: 集約状態に Intent 由来の静的フィールドが**残っていない**こと（snapshot の
フィールド一覧で確認）、`&Intent` ガード（id 不一致・長さ不一致の拒否）のテスト。

---

# 改訂 4（2026-08-29・メインセッション裁定）— 再生時の `&Intent` は A+ 方式

委任先が検出したギャップ（スナップショットから `stages` が消えると、`apply_event` の
slug → 位置解決に使うステージ列を再生経路が入手できない）への裁定。**A+ を採る**。

1. **Repository が再生用の `Intent` をジャーナル先頭の `Started`（seq_nr 1・genesis 専用）
   から復元する。** スナップショットの有無に関わらず先頭 1 件を読む（ローカル SQLite の
   追加読取 1 回は許容）。`find_by_id(&IntentExecutionId)` の署名は不変。
2. `apply_event` は `&Intent` を引数で受ける（規則 `aggregate-references.md` の
   パラメータ渡しを再生経路にも適用）。
3. **復元した `Intent` は `RehydratedIntentExecution` に載せて返す。** ユースケースは
   再構成の戻り値から `&Intent` を得て集約コマンドへ渡す。
4. **改訂 3 の 4（「execute は `&Intent` も受け取る — Controller が読んで渡す」）は
   本改訂で差し替える**: `CommitVerdictUseCase::execute` は `&Intent` を**受け取らない**。
   出所はストリーム 1 つに畳まれ、U7 の申し送り「Intent をどこから読むか」は
   「新規実行の作成時・CLI 表示用」に狭まる。
5. 集約コマンド側の `intent.id() == self.intent_id` ガード（規則の取り違え防御）は維持する。

**根拠**: 1 intent : n 実行では、各実行が従うべき計画は「その実行が開始した時点の intent」で
あり、その永続記録は当該実行のストリームの `Started` に他ならない。Intent は不変
（recompose は実行側の overlay）なので Started の写しが古くなることはない。
D 案（イベントを index 運びへ変更）は、slug がイベントを自己記述の歴史にしている現状の
利点（監査投影も slug を使う）を壊すため不採用。C 案は改訂 3 の受入基準に抵触し不採用。
B 案は出所が二重になり不採用。

**追認**: `Intent` に `depth` / `test_strategy` を含める（現 `Started` payload の材料一式は
すべて Intent の静的構成である）。

---

# 改訂 5（2026-08-29・オーナー原則確認による A+ の撤回）— Repository は集約単位

オーナー確認: **Repository は自分の集約だけを I/O する** — `IntentRepository` は `Intent` のみ、
`IntentExecutionRepository` は `IntentExecution` のみ。この原則に照らし、改訂 4（A+）の
「`IntentExecutionRepository` が復元した `Intent` を `RehydratedIntentExecution` に載せて返す」は
**実行のリポジトリに Intent を扱わせる違反**であり撤回する。

## 訂正後の形（委任先の B 案の形）

1. `IntentExecutionRepository::find_by_id(&IntentExecutionId, intent: &Intent)` — 再生材料として
   `&Intent` を**パラメータで受ける**（I/O ではない）。再生中に `intent.id()` と自ストリームの
   intent_id（Started / スナップショット由来）を照合し、不一致は `Corrupt` 系で拒否。
2. `RehydratedIntentExecution` は Intent を**載せない**（実行 + 版のみ）。
3. `CommitVerdictUseCase::execute(&IntentExecutionId, intent: &Intent, transition, at)` —
   改訂 3 の 4 を復活（Controller が読んで渡す I8 型。ユースケースは取得手段を持たない）。
4. `Started` が intent 材料を運ぶのは従来どおり（歴史 + 投影核の入力）。リポジトリが**内部で**
   自ストリームの Started を復号するのは自集約の I/O であり違反ではない — 違反は Intent を
   **外へ返す**こと。
5. **`IntentRepository` は U7（intent-create 実装時）で新設** — `Intent` の I/O はそこだけ。
   B12 ではポート定義も不要（テストは `Intent` を直接構築）。申し送りに追加。

## 意味論の注記（記録）

現在の裁定では `Intent`（解決済み stages を含む）は**不変**なので、「E の Started の写し」と
「IntentRepository が返す現在の Intent」は常に一致し、どちらを使っても同じである。将来
「実行ごとに計画を再解決する」（同一 intent でも実行により stages が異なる）要件が現れたら、
stages は Intent ではなく**実行の開始材料**へ移す再設計が要る — そのときの再裁定事項として
記録する。

---

# 改訂 6（2026-08-29・オーナー確定）— find_by_id は自集約の ID だけを取る（A 案確定）

オーナー確定: **`IntentExecutionRepository::find_by_id(&IntentExecutionId)` — 必要なのはその
集約の id だけ**。改訂 5 の 1（ポートが `&Intent` を受ける）は上書き。委任先が導出した
**A 案が正**:

- ポート署名は `find_by_id(&IntentExecutionId)` のまま。
- 再生材料の `Intent` は **Impl が自ストリーム先頭の `Started`（seq_nr 1）から内部復元**する。
  自集約のストリームを読むのはリポジトリの本業であり、責務境界の違反ではない。
- 復元した Intent を**外へ返さない**（改訂 5 の 2 は維持 — `RehydratedIntentExecution` は
  実行 + 版のみ）。
- `CommitVerdictUseCase::execute(&IntentExecutionId, intent: &Intent, ...)` は維持
  （改訂 5 の 3 — Controller が読んで渡す）。

**原則の一般形**（gateway-taxonomy.md に登録）: Repository の署名は自集約の ID だけを取り、
他の集約・エンティティを引数にも戻り値にも出さない。再生に他エンティティの材料が要る場合、
それは自ストリームの誕生イベントに記録されているはずであり、そこから内部復元する。

## 委任先の判断 3 点への追認

1. `Intent` が `StartRequest` を丸ごと保持 — **追認**（4 値に不変条件なし、serde 導出で
   検査迂回の余地なし）。
2. `depth` / `test_strategy` を含める — 追認済み（改訂 4）。
3. `StartError` → `IntentError` 統合 — **追認**。計画の解決が `Intent` 構築側へ移ることで
   `IntentExecution::start` は Always Valid な `Intent` を受けて失敗しなくなり（`Result` が
   消える = E1 の勝ち）、`StartError` の持ち主が消える。`no-backward-compatibility` どおり
   別名を残さず削除。

---

# 改訂 7（2026-08-29・オーナー裁定）— WorkflowDefinition を集約規則へ適合させる

`WorkflowDefinition` は集約（オーナー裁定）だが、現状はイベント語彙が無く、ファクトリ
`new` が素の Self だけを返す — `aggregate-commands.md`（ファクトリは (集約, イベント) の対が
必須。無ければリポジトリで永続化できない）に非適合。**B12 で修正する**。

1. **`WorkflowDefinitionEvent` を新設**（workflow_definition モジュール）。genesis 変種
   `Defined { id: WorkflowDefinitionId, revision: DefinitionRevision }` — 定義が確立された
   事実の記録。内容フル（stage graph 等）はイベントに焼かない（実ファイル = この集約の
   リードモデルが内容そのもの。将来の `ScopeComposed` 等の差分イベントが変更内容を運ぶ）。
2. **genesis ファクトリは対を返す**: `WorkflowDefinition::define(...) ->
   (WorkflowDefinition, WorkflowDefinitionEvent)`（動詞は factory-naming のドメイン語優先で
   調整可。現 `new` の引数を引き継ぐ）。
3. **Repository の読取経路は再構成であり genesis ではない**: 現 `new` の役割（3 入力の
   束ね直し）は `from_artifacts(...)` 等の再構成コンストラクタとし、**イベントを生成しない**
   （規則の再構成条項）。`WorkflowDefinitionRepositoryImpl` はこちらを呼ぶ。genesis の
   `define` は将来の変異取込（後続 intent — audit-1.md 承認済み方針）が呼ぶ入口として
   テストで形を固定しておく。
4. テスト: genesis が対を返す形・`Defined` の材料・再構成がイベントを生成しないこと。
   doc は `aggregate-commands.md` を参照。
5. 外形不変（ゴールデン・投影 19 本）は従来どおり絶対。ジャーナル・永続化の接続は
   **やらない**（イベントを store する先は後続 intent — 型と形だけ規則適合させる）。

---

# 改訂 8（2026-08-30・オーナー裁定）— Intent は集約。IntentEvent と対返しファクトリが必要

先の裁定 ②（`Intent::new` はイベント不要・現行どおり）は**オーナー自身のその後の原則裁定に
より上書き**された — 「IntentRepository は必ず Intent を I/O する」「集約のファクトリは
(インスタンス, イベント) の対を返す。無ければリポジトリで永続化できない」。`Intent` は
**集約**である（静的で変異が現状無いだけ — WorkflowDefinition と同じ類型）。

1. **`IntentEvent` を新設** — genesis 変種 `Created`（材料 = intent の全属性。生成の事実の記録）。
2. **genesis ファクトリは対を返す**: `Intent::create(...) -> (Intent, IntentEvent)`。
   動詞 `create` は upstream の intent-create そのもの（factory-naming の `create` 行 —
   ドメイン語優先）。現 `new` の検査（Always Valid）を引き継ぐ。
3. **再構成経路を分離**（無イベント）: `Started` の材料から組み直す経路
   （IntentExecutionRepositoryImpl の再生用復元・serde 復号）は再構成コンストラクタとし、
   イベントを生成しない。Always Valid 検査は genesis と再構成の両経路で同一。
4. `IntentExecution::start(id, intent, ...)` が受け取る `intent` は**生成済みの集約インスタンス**
   （呼出側が `Intent::create` の対の左を渡す）— `Started` が intent を丸ごと運ぶ現行形は
   維持（BR2.2 自己完結。イベントに載る写しは歴史であり aggregate-references の違反ではない）。
5. **ジャーナル接続はしない**（WorkflowDefinition と同じ扱い — `IntentCreated` を store する
   `IntentRepository` は U7 の intent-create 実装時。今回は型と形の規則適合まで）。
6. テスト: 対を返す形・`Created` の材料・再構成が無イベント。

記録の是正（メインセッション実施）: ubiquitous-language.md 等の「集約ではない不変構造体」
記述を「集約（静的・作成時に `Created` を吐く・変異は現状なし）」へ訂正。監査記録 ② に
上書き注記。

---

# 改訂 9（2026-08-30・オーナー裁定）— ドメインから永続化知識を全撤去

オーナー裁定: 「`IntentMaterial` は削除せよ。ドメインに永続化知識を含めるな。集約はどんな
永続化知識からも中立。全部撤回せよ」。正典化済み:
`coding-rules/domain-persistence-neutrality.md`（必読）。

## 撤去対象（実測: serde 使用 28 ファイル + 2 件）

1. **`IntentMaterial` を削除**（改訂 8 実装中の serde 復号中間表現）。
2. **`core-command-domain` から serde を全撤去** — 全 28 ファイルの derive / `#[serde(...)]`
   属性・`Cargo.toml` の serde 依存。
3. **`event_manifest.rs` を domain から撤去**（ジャーナル列の値 = 永続化語彙）。書く側は
   command interface-adapter、読む側は RMU が**各自**定数を持ち、一致は既存の横断適合テスト
   （journal_protocol_conformance 等）で固定。
4. **`event-store-adapter-rs` 依存を domain から撤去** — `IntentId` / `IntentExecutionId` の
   `AggregateId` trait 実装は、アダプタ側の**ラッパ型**へ移す（境界で変換）。

## 行き先（アダプタが永続化モデルを所有）

- **command interface-adapter** に永続化 DTO モジュールを新設: イベント（全変種 + payload）・
  スナップショット・Intent とその部品（StageEntry / StageDisplay / WorkspaceScan /
  StartRequest / 各 enum / 各 Id は文字列で運び `parse` で戻す）。
  - 書き: domain の公開アクセサ → DTO → serde。
  - 読み: serde → DTO → domain の**検査付き再構成コンストラクタ**（Always Valid の担保は
    アダプタの変換関数経由で維持 — 検査を迂回する構築口を作らない）。
- **RMU は自前の復号 DTO を持つ**（cqrs-boundaries「側ごと専用化」）— domain のイベント型へ
  変換して投影核に渡す（投影核のシグネチャは不変）。
- **ワイヤ形式はバイト不変** — DTO のフィールド名・形は現行 serde 出力と 1 対 1。既存の
  ITF 準拠・crash・adapter・RMU テストが証明。ゴールデン外形不変は従来どおり絶対。

## 補足

- `chrono` は残る（時刻の値は永続化知識ではない — 規則の対象外）。
- スナップショット型（`IntentExecutionSnapshot`）は serde が消える結果、アダプタが読める
  **公開の memento API**（読取アクセサ + 検査付き構築）が必要になる。クレート内私有の裁定は
  「serde の裏口として不可視に使われる」前提だったため、**アダプタという正当な消費者への
  公開はこの改訂で認める**（visibility は必要最小に）。
- domain-design decisions.md の「serde がドメインに入るトレードオフ受容」は上書き
  （メインセッションが失効注記を入れる）。

---

# 改訂 10（2026-08-30・オーナー裁定）— CommitVerdictUseCase はリポジトリを保持し execute 内部で使う

オーナー裁定: 「ユースケースはリポジトリの参照を保持し、execute 内部で利用する。リポジトリを
外で使うな」。改訂 5 の 3 / 改訂 6 で維持していた「`execute` が `&Intent` を受け取り Controller が
読んで渡す」は **I8（読み取り専用ユースケース `Next` 専用のパターン）の誤適用**であり撤回。

## 確定形

```rust
pub struct CommitVerdictUseCase<E: IntentExecutionRepository, I: IntentRepository> {
    execution_repository: E,
    intent_repository: I,
}
// execute から &Intent 引数を除去
pub async fn execute(&mut self, id: &IntentExecutionId, transition: ReportedTransition,
                     occurred_at: DateTime<Utc>) -> Result<(), CommitError>
```

内部フロー: ① `execution_repository.find_by_id(id)` で実行を再構成 → ② 実行の `intent_id()` を
読む → ③ `intent_repository.find_by_id(intent_id)` で Intent を取得 → ④ `&intent` を集約
コマンドへ（取り違えガードは従来どおり集約側で発火 — ここでは構成上一致する）→ ⑤ `store`。
`Conflict` 再試行は attempt 全体（①〜⑤）をやり直す（Intent は不変なので再取得は無害）。

## IntentRepository ポートの新設（前倒し）

改訂 5 の 5「U7 で新設」を上書きし、**ポート定義を B12 で新設**する（ユースケースが注入を
要求するため）。

- ユースケース層に `IntentRepository` ポート — 当面 `find_by_id(&IntentId) -> Result<Intent, _>`
  のみ（`store(&IntentEvent, &Intent, ...)` は intent-create を実装する U7 で追加）。
- Repository は自分の集約（Intent）だけを I/O（gateway-taxonomy 既裁定）。
- テストはユースケース内 fake。**実物実装（読み先 = intent 自身のジャーナル）は U7** —
  現時点で Intent の完全な材料は各実行の `Started` にしか永続化されていないため、実物の
  読み先の設計（intent ジャーナルの導入）ごと U7 の課題として申し送りを維持。
  interface-adapter に InMemory 実装（結線テスト用）だけ置くのは可。

## 注記

- I8 パターン（Repository 非注入・Controller が `&` で渡す）は**読み取り専用ユースケース
  （`Next`）にのみ適用**される — use-case-rules §4 の射程はそのまま。書込ユースケースへの
  流用を禁じる 1 行を §4 に足す（メインセッションが実施）。
- 結線テスト（wiring test）もシグネチャ変更に追随。

## 改訂 10 追補（2026-08-30・オーナー裁定）— execute 引数の一般則

「execute の引数に集約を渡すのをやめろ。集約 ID や値オブジェクトを渡すのは OK」— 改訂 10 を
一般則へ格上げ（use-case-rules §2b として正典化）。読み取り専用ユースケース（U6 Next）も
対象であり、I8 の「Controller が集約を `&` で渡す」機構は失効 — 読取専用の型保証は
**find 系動詞しか持たない読取専用ポートの注入**へ置き換える（§4 の目的は維持、手段の変更）。
改訂 10 の CommitVerdictUseCase 確定形（id + ReportedTransition + DateTime）はこの一般則に
最初から適合している。
