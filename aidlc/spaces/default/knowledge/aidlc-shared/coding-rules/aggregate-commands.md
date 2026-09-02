# 集約のコマンドは必ずドメインイベントを戻り値で返す — decide / apply の分離

**裁定日**: 2026-08-29（オーナー「集約は &mut self なメソッドで状態遷移するときにイベントを
戻り値で返すべき。この考え方が徹底されていない」— B3 以来の暗黙の家風を明文化し、
CQS 規則の字面との矛盾を閉じる）
**関連**: [command-query-separation.md](command-query-separation.md)（本規則が集約について
これを精密化する）、[aggregate-references.md](aggregate-references.md)、
[error-handling.md](error-handling.md)
**適用例**: `IntentExecution` の全 11 コマンド（`complete_stage` / `open_gate` / `approve_gate` /
`reject_gate` / `revise_stage` / `skip_stage` / `jump` / `park` / `unpark` / `recompose` /
`switch_autonomy` — いずれも `Result<IntentExecutionEvent, CommandError>`）、
`WorkflowDefinition::redefine`、`CompiledDefinition` の 3 コマンド（`recompile` /
`register_scope` / `apply_plugin_selection` — b36、2026-09-02。媒体がスナップショットなので
`replay` は無く、genesis `compile` と `From<Compiled>` が構築口）
**機械強制**: レビュー基準 + `cargo lint` ルール候補（集約 impl の pub `&mut self` メソッドが
`Result<Xxx…Event, _>` 以外を返したら違反。`apply_event` は除外。赤例テスト必須）

## 原則

イベントソーシングの集約では、**状態遷移（`&mut self` のコマンド）は必ず単一のドメイン
イベントを戻り値で返す**。

```rust
// decide — 判断し、遷移し、起きた事実を 1 イベントで返す（1 コマンド 1 イベント・絶対）
pub fn approve_gate(&mut self, ..) -> Result<IntentExecutionEvent, CommandError>

// apply — イベントを畳み込む fold。失敗を返さない（壊れた歴史はクラッシュ — 2026-08-30 裁定）
pub fn apply_event(&mut self, ..)

// genesis (ファクトリ) — 集約インスタンスと誕生イベントの**両方**を対で返す (必須)
pub fn start(..) -> (IntentExecution, IntentExecutionEvent)
```

**ファクトリメソッドも対で返すことは必須である**（オーナー裁定 2026-08-29）。理由は
コマンドと同じで、しかもより直接的である — Repository の永続化は
`store(&event, &aggregate, expected_version)` の形で**イベント（ジャーナルへ追記する分）と
適用後の集約（スナップショットに書く分）を同一トランザクションで受け取る**。ファクトリが
集約だけを返すと誕生をジャーナルに書けず、イベントだけを返すとスナップショットに書く実体が
無い。どちらが欠けても永続化が組めない。

なお**再構成はファクトリではない** — 歴史を読み戻す経路であり、イベントを**作ってはならない**
（作ればリプレイのたびに歴史が増える）。

## 再構成の形（オーナー裁定 2026-08-30 — event-store-adapter-rs サンプル準拠）

集約に作ってよい構築 API は **genesis（`new` / `create` / `start` — 対を返す）と
`replay`・`apply_event` だけ**である。本家 v3 の `user-account-sqlite` サンプルが正典。

- **イベントは内容（値）を運ぶ** — 集約インスタンスを埋め込まない
  （`UserAccountEvent::Created { name }` の形）。集約を埋め込むと「イベントを復号するには
  集約が要り、集約はイベントからしか作れない」という循環が生じ、イベントからのリプレイが
  成立しない。genesis イベントから集約を導出する変換（`From<Created> for Intent`）が
  リプレイのスナップショット種を与える。
- **genesis イベントは集約 id と genesis の材料を運ぶ**（追記 2026-09-02、b39）— `Created` が
  `id` と依頼・計画を、`Defined` が `id` と内容を運ぶように、`Started` も `id` と解決済み計画
  （各ステージの slug / phase / plan_action）を運ぶ。理由: 集約の歴史は**自ストリームだけで**
  再生できなければならない。`Started` が `intent_id` しか運ばず genesis が `&Intent` を要した
  旧形は、再生に他集約の状態を要する ES の基本違反だった（RMU が集約を `replay` で起こそうと
  して発覚）。`From<(Started, DateTime<Utc>)>` が唯一の genesis 状態導出で、`start` はそれを通る。
- **再構成は失敗を返さない** — `replay` / `apply_event` は `Result` を返さない。歴史を読む
  だけの経路にエラー型は要らず、壊れた歴史（通番の飛び・未知ステージ・不変条件違反）は
  回復せず**クラッシュが正**である（`expect` / `panic` 容認 — 万一発生したらアプリケーション
  は落ちてよい、というオーナー裁定。`# Panics` を明記し、allow には理由を書く）。
- **memento 双子型を作らない** — 集約と構造同一（ID 型の包み紙差だけ等）の「写し」型は複製で
  しかない。スナップショット行は書込規約の一部として書かれ続けるが、**状態の正本はイベント列**
  であり、読取は「版の正本（envelope の version）+ 存在検査」にだけ使いジャーナル全再生で
  状態を導出する。
- **経緯（誤適用の記録）**: `Intent::from_material`（造語 + genesis と同一署名の双子）→
  `restore` / `rehydrate`（保存値からの検証付き再構成 = 状態ストア発想の第 3 の構築口）→
  `IntentSnapshot` / `IntentExecutionSnapshot`（構造同一の memento 双子）は、いずれも
  2026-08-30 に**この裁定で撤去**された。リプレイの理解を欠いた設計はこの 3 段を辿りがちで
  ある — 迷ったら本家サンプルの API 面（genesis / replay / apply_event のみ）に戻ること。

- **decide と apply は分離**し、リプレイと通常実行は同一経路（apply）を通る。
- 戻したイベントは呼出側（ユースケース）が `store(&event, &aggregate, ..)` へ渡す —
  **イベントは書込パイプラインの産物**であり、これが永続化の唯一の材料である。
- 拒否はガード付き `Err`（無言 no-op にしない）。

## CQS 規則との関係（矛盾の解消）

[command-query-separation.md](command-query-separation.md) の「Command は戻り値なし または
`Result<(), E>`」は**集約のコマンドには適用しない** — 本規則が優先する。イベントの戻しは
状態を読むクエリではなく**書込自体の産物**（受領証）であり、CQS が禁じたい「変更と観測の
結合」に当たらない（裁定 7 の線引き: 配管であって読取チャネルではない）。したがって
集約コマンドのイベント返しに個別のオーナー許可は**不要**（正式な形である）。

ユースケース層はこの限りではない — ユースケースの Command は `Result<(), E>` を返し、
結果はリードモデル経由で読む（裁定 7。`CommitVerdictUseCase::execute` が実例）。

## 射程

- 対象は**集約**（ES の decide を持つ型）。値オブジェクト・Domain Primitive の可変メソッド
  （例: `BoltRefs::append_slug -> Result<(), _>` — upstream 契約の重複拒否）は対象外。
- `WorkflowDefinition` は**集約**である（「読取専用集約」「読取モデル集約」の呼称は廃止 — 2026-08-29 オーナー裁定: 集約に統一）。現スコープでは本システムから変異させないためコマンド未実装だが、**変異が要件化した時点で本規則がそのまま適用される**（状態遷移はイベントを吐く。実ファイル stage-graph.json 等はこの集約のリードモデル）。

## 禁止パターン

- 集約の `&mut self` コマンドが `()` / `Result<(), E>` を返す（イベントの闇落ち —
  永続化する材料が消える）
- **ファクトリが集約だけを返す**（誕生イベントが無く、ジャーナルに最初の 1 行を書けない）、
  または**イベントだけを返す**（スナップショットに書く集約実体が無い）
- 再構成経路（`replay` / `apply_event`）がイベントを生成する（リプレイのたびに歴史が増える）
- 再構成経路が `Result` を返す（歴史読みにエラー型は無い — 壊れた歴史はクラッシュ、
  2026-08-30 裁定）
- genesis / `replay` / `apply_event` 以外の構築口（`from_material` / `restore` / `rehydrate` /
  `from_snapshot` と memento 双子型）を作る（2026-08-30 裁定）
- 1 コマンドから複数イベントを返す・`Vec<Event>` を返す（1 コマンド 1 イベント違反）
- コマンドがイベントを返さず、別のクエリで「さっき何が起きたか」を答えさせる
  （変更と観測の分断 — 別トランザクション間で食い違う）
