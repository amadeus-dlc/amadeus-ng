# 集約のコマンドは必ずドメインイベントを戻り値で返す — decide / apply の分離

**裁定日**: 2026-08-29（オーナー「集約は &mut self なメソッドで状態遷移するときにイベントを
戻り値で返すべき。この考え方が徹底されていない」— B3 以来の暗黙の家風を明文化し、
CQS 規則の字面との矛盾を閉じる）
**関連**: [command-query-separation.md](command-query-separation.md)（本規則が集約について
これを精密化する）、[aggregate-references.md](aggregate-references.md)、
[error-handling.md](error-handling.md)
**適用例**: `IntentExecution` の全 11 コマンド（`complete_stage` / `open_gate` / `approve_gate` /
`reject_gate` / `revise_stage` / `skip_stage` / `jump` / `park` / `unpark` / `recompose` /
`switch_autonomy` — いずれも `Result<IntentExecutionEvent, CommandError>`）
**機械強制**: レビュー基準 + `cargo lint` ルール候補（集約 impl の pub `&mut self` メソッドが
`Result<Xxx…Event, _>` 以外を返したら違反。`apply_event` は除外。赤例テスト必須）

## 原則

イベントソーシングの集約では、**状態遷移（`&mut self` のコマンド）は必ず単一のドメイン
イベントを戻り値で返す**。

```rust
// decide — 判断し、遷移し、起きた事実を 1 イベントで返す（1 コマンド 1 イベント・絶対）
pub fn approve_gate(&mut self, ..) -> Result<IntentExecutionEvent, CommandError>

// apply — イベントを畳み込む fold。イベントを消費する側なので戻り値は無い
pub fn apply_event(&mut self, ..) -> Result<(), ApplyError>

// genesis (ファクトリ) — 集約インスタンスと誕生イベントの**両方**を対で返す (必須)
pub fn start(..) -> (IntentExecution, IntentExecutionEvent)
```

**ファクトリメソッドも対で返すことは必須である**（オーナー裁定 2026-08-29）。理由は
コマンドと同じで、しかもより直接的である — Repository の永続化は
`store(&event, &aggregate, expected_version)` の形で**イベント（ジャーナルへ追記する分）と
適用後の集約（スナップショットに書く分）を同一トランザクションで受け取る**。ファクトリが
集約だけを返すと誕生をジャーナルに書けず、イベントだけを返すとスナップショットに書く実体が
無い。どちらが欠けても永続化が組めない。

なお**再構成はファクトリではない** — `from_snapshot`（復号の検査点）と `apply_event`（fold）は
歴史を読み戻す経路であり、イベントを**作ってはならない**（作ればリプレイのたびに歴史が増える）。

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
- 再構成経路（`from_snapshot` / `apply_event`）がイベントを生成する（リプレイのたびに
  歴史が増える）
- 1 コマンドから複数イベントを返す・`Vec<Event>` を返す（1 コマンド 1 イベント違反）
- コマンドがイベントを返さず、別のクエリで「さっき何が起きたか」を答えさせる
  （変更と観測の分断 — 別トランザクション間で食い違う）
