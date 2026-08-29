# B10 開発者報告 1 — U1 ゴールデン追加採取と RMU 投影の完成

Conversation language: 日本語
担当: 委任先（Opus）/ ブランチ: `bolt/b10-u1-golden-recapture` / 報告日: 2026-08-29
検証はすべて `CARGO_TARGET_DIR=$PWD/target-delegate` で実行。push はしていない。

---

## 0. 要約

**採取は完了、投影は 2 点を残して完了**。ブリーフの 4 点のうち 3 点は実バイトを採れた。
残る 1 点（`AUTONOMY_MODE_SET` 成功経路）は**ピンでは到達不能**であることを全数走査で
確定し、捏造せず `cases-missing.json` に根拠を残した。

その過程で、ブリーフが「採取さえ済めば実装できる」と見込んでいた**骨格生成が、採取とは
別の理由で詰まっている**ことが分かった。バイトは採れたが、**ジャーナルに無い材料が 4 つ
ある**（うち 1 つは環境の絶対パス）。ここは読み替えず止め、裁定を求めて連絡済みである（§4）。

- コミット 3 本 / 採取ケース 22 → **25**、監査ブロック検収 42 → **62** 本
- 投影ゴールデン検収 10 → **13 ケース**（`audit.md` + `state.diff` の両面バイト一致）
- テスト 737 → **741 件**（全緑）、カバレッジ **98.4805%**（`origin/main` の 98.4801% と同水準）
- 受入基準 **1〜8, 10 は PASS**、**9 は条件付き**（§6）

---

## 1. 採取できた点

| # | ブリーフの項目 | 結果 | 実バイトの置き場 |
|---|---|---|---|
| 1 | 状態ファイルの骨格 | **採取済み** | `cli/intent-create/classic-scope/state-full.md`（102 行全文） |
| 2 | 非ゲート `StageCompleted` の単独経路 | **採取済み** | `cli/report/completed-ungated` |
| 3 | `AUTONOMY_MODE_SET` 成功経路 | **到達不能**（§2） | `cli/cases-missing.json` |
| 4 | 途中で見つかる他の未採取バイト | **2 件採取** | `cli/jump/execute-backward`, `cli/jump/execute-forward-across-phases` |

### 1-1 骨格 — B8 の前提が 1 つ間違っていた

B8 §8-1 は「ゴールデンは差分（`state.diff`）しか持たない」としていたが、**これは誤り**である。
`cli/intent-create/classic-scope` は状態ファイルが**存在しないところから**生まれる遷移なので、
その `state.diff` のハンクは全文そのもの（`+` 行 101 + 文脈行 1 = 102 行）だった。骨格の実バイトは
B8 の時点で既に手元にあった。

それでも明示性のため `state-full.md` として全文を別に採った。差分は「前」があってはじめて読める
観測であり、骨格を**ゼロから起こす**側の検収には全文のほうが素直に使えるためである。

upstream 側の正本は `aidlc-utility.ts` の状態ファイル template literal である。
`knowledge/aidlc-shared/state-template.md` は**LLM 向けの契約文書でツールは読まない** —
両者は食い違っており、それが §2 の実害を生んでいる。

### 1-2 非ゲート `StageCompleted` — 到達手順の探索が本体だった

出荷グラフで非ゲートなのは initialization の 3 ステージだけで、その 3 本は genesis で完了済みに
なる。`report --result completed` は `pending` のステージを拒否するので、素直には打てない。

到達手順は**後方ジャンプ**だった。`aidlc-jump.ts execute --target workspace-scaffold
--direction backward` で initialization ステージを `[-]` へ戻すと、`aidlc-orchestrate.ts report
--result completed` が `advance` だけを走らせる（ゲートを開かない）。

確定した実バイト:

```
**Event**: STAGE_COMPLETED
**Stage**: workspace-scaffold
**Details**: Stage Workspace Scaffold completed
```

`**Details**:` は `Stage <表示名> completed` で、ゲート経由の `Stage <表示名> approved by gate`
とは**文言が割れる**。書き手が違う（`aidlc-state.ts` の `handleAdvance` と `handleApprove`）ので、
片方に寄せてはならない。B8 が `GateApproved` との同型から置いた暫定は、結果として実バイトと
一致していた（推測が当たっていたが、当たっていたことを実測で示せたのが本 Bolt の成果である）。

### 1-3 途中で見つかった 2 件

**後方ジャンプ**（`cli/jump/execute-backward`）— 上の到達手順そのものが未採取ケースだった。
既存ゴールデンは前方ジャンプ 3 件のみで、`**Direction**: BACKWARD` とフェーズ境界の 3 本は
これが初出である。

**フェーズ境界をまたぐ前方ジャンプ**（`cli/jump/execute-forward-across-phases`）— 既存の前方
2 件はどちらも inception 内で完結するため、**前方**の境界 3 本は 1 度も採れていなかった。
後方だけ実装すると前方は上流ソースの読解に依拠することになるので、そちらも採った。これで
ジャンプの境界は前方・後方とも実行バイトで裏取りされている。

この 2 件が確定させた実測挙動:

- ジャンプの境界 3 本は**ゲート経由の 3 本と同型ではない**。ジャンプ側だけが `**Details**:` を
  持ち（`Phase boundary crossed via <方向> jump` / `Traceability verification on jump`）、
  `**Stages completed**:` は計画上のフェーズ内件数ではなく**チェックボックスの数え直し**である
  （後方 0 / 前方 1 — 直前の書き換え後に数えた値でしか説明が付かない）。
- 前方の `STAGE_SKIPPED` は「間のステージを文書順」→「**最後に出発点そのもの**」の順に並ぶ。
- ジャンプ後の `- **Last Completed Stage**:` は到達点より手前を逆順に辿った最初の `[x]`。
  1 つも無ければ upstream の既定値 `state-init`。
- `- **Next Action**:` は書き手で綴りが割れる。genesis は **slug**（`Execute practices-discovery`）、
  `advance` は**表示名**（`Execute Workspace Detection`）。

---

## 2. 採取できなかった点 — `AUTONOMY_MODE_SET` 成功経路

ブリーフは「upstream のどの動詞 / どの遷移がこの行を作るかを探索」せよとした。探索した結果、
**ピン `3c3146cf` の配布シェル 262 ファイルのどこにも、`- **Construction Autonomy Mode**:` 行を
状態ファイルへ書き込む経路が無い**ことが確定した。

1. 状態ファイルを起こす唯一のテンプレート（`aidlc-utility.ts` の template literal）に当該行が無い
   — `state-full.md` の全文がその実測である。
2. `setField` は行が無ければ**黙って no-op**、`setFieldStrict` は **throw** する。どちらも挿入は
   しない。`set-autonomy` が踏むのは後者で、これが終了コード 1 の出どころである。
3. 行を挿入できる唯一の関数 `setOrInsertField` の呼出先は `Merge-Held` / `Skeleton Stance` /
   `Construction Iteration` / `Practices Affirmed Timestamp` / `Parked` / `Parked At Stage` /
   `Active Unit` / `Unit State` / `Unit Pause Reason` / `Unit Next Action` の 10 種のみ。
4. 汎用の `aidlc-state.ts set <field>=<value>` も `setField` 経由なので挿入しない。

当該行を規定しているのは §1-1 の契約文書だけである。**テンプレートと契約文書が食い違っている
upstream 側の欠落**であり、逸脱台帳の対象になる。行を手で足せば `set-autonomy` は通るが、それは
upstream の挙動ではなく採取者の捏造なので採らなかった。

帰結として `AUTONOMY_MODE_SET` の監査行のフィールドキー（`**Mode**:`）は**実行出力としては
採れない**。値そのものはピンのソース（`aidlc-bolt.ts` の
`emitAudit(pd, "AUTONOMY_MODE_SET", { Mode: flags.mode })`）から読めており、実装はその値のまま
だが、「実行バイトでの裏取りはピン更新待ち」である旨を `key::MODE` の doc・`cases-missing.json`・
ゴールデン README の 3 箇所に書いた。B8 の「暫定」という位置づけは変わらないが、**なぜ暫定の
ままなのか**が推測から実証に変わっている。

---

## 3. 投影の完成度

### 3-1 実装した（両面バイト一致で検収済み）

| 対象 | 内容 |
|---|---|
| `StageCompleted` の `**Details**:` | 暫定 → 実バイトで裏取り。doc を「未採取」から実測の引用へ改めた |
| ジャンプのフェーズ境界 3 本 | 新規実装。`**Details**:` 付き、`**Stages completed**:` は数え直し |
| `## Phase Progress` の付け替え | 新規実装。前方は出発フェーズ `Verified` + 飛び越えを `Skipped`、後方は到達点より後ろでスコープ内ステージを持つものを `Pending`、どちらも到達フェーズを `Active` |
| `- **Lifecycle Phase**:` | 新規実装。フェーズをまたぐ遷移が初めて差分を見せた行 |
| ジャンプ時の `- **Completed**:` / `- **Last Completed Stage**:` | 新規実装。後方ジャンプは `[x]` を戻すので完了数が減る（4 → 0） |

**イベントは 1 バイトも拡張していない**。フェーズ境界は `Jumped` の `source` / `target` と計画から
導ける（両方のフェーズを計画が知っている）ので、`GateApproved` のように `PhaseBoundary` を足す
必要はなかった。導出であって推測ではないので、材料が足りているうちはイベントを太らせない
（`resolved_plan.rs` の「正本は 1 つでよい」と同じ理由）。

### 3-2 実装していない（§4 の裁定待ち）

`ProjectionError::ScaffoldTemplateUnavailable` は**撤去していない**。ブリーフは「撤去し、genesis の
骨格生成を実バイトで実装」としていたが、詰まっているのはバイトではなく材料だった（§4）。
`- **Stages to Execute**:` / `- **Stages to Skip**:` の 2 行も触っていない。前者だけなら導けるが、
2 行で 1 組の計画表なので片方だけ書くと読み手に矛盾した表を見せることになる。

エラー型の doc は「テンプレートの実バイトが無い」という**もう正しくない理由**から、実際の 4 つの
材料不足へ書き換えた。撤去の可否そのものは裁定事項なので型は残してある。

---

## 4. 止めて報告した点 — 骨格生成に材料が 4 つ足りない

ブリーフの「畳み理由が `Started` の材料から導けない場合は止めて報告」に該当する。骨格 102 行を
全数照合した結果、詰まるのは畳み理由だけではなく、**4 行**だった。`Started` が運ぶのは
definition_id / definition_revision / scope / request / depth / test_strategy / stages
（`StageEntry`: slug, phase, plan_action, conditional, display{number, name, lead_agent}）/ scan の
8 つだけである。

| 行 | なぜ導けないか |
|---|---|
| `- **Project Root**:` | ワークツリーの絶対パス。**ジャーナルに存在しない**。`WorkspaceScan` は project_type / languages / frameworks / build_system の 4 値のみ。投影核が環境を読まないのは NFR3 の要請なので、ここは設計上の穴である |
| `- **Project**:` | 人間の記述そのもの。`request` は `/aidlc ` を前置した後の形しか持たない。前置を剥がす導出は引数が空のとき割れる（upstream は `[Project description]` と `/aidlc <scope>` を別々に書く） |
| `- **Review Override**:` | 対応するフィールドが無い。常に空と決め打つと `--review` 指定時に割れる |
| `- **Stages to Skip**:` の畳み理由 | upstream の規則は「slug が `reverse-engineering` かつ greenfield かつ**素のグリッドが EXECUTE**」。素のグリッド値と調整後の値の区別が要る。`PlanAction` は 2 値、`conditional` は出荷グラフ 33 ノード中 **22 ノード**で真（1.2 / 1.3 / 1.5 / 1.6 も classic で SKIP）なので、どちらでも代用できない |

提示した選択肢（推奨順、詳細は委任者へ送った連絡）:

- **A（推奨）**: 骨格生成は投影の仕事ではないと裁定し、エラー型は残す。`Project Root` が環境値で
  ある以上ジャーナルだけでは描けないことが構造的に確定している。骨格は intent-create の
  ユースケース（コマンド側）が書き、RMU は既存本文への差分適用に徹する。4 点すべてが解消する。
- **B**: `Started` を 4 値拡張する。逐語再現は完全になるが、環境パスをドメインイベントへ載せる
  ことになり ADR-008 と NFR3 の趣旨に反する疑いがある。
- **C**: 畳み理由だけ slug ハードコードで近似する。`infra` スコープ + greenfield のときだけ静かに
  割れる（11 スコープ中 1 つ）。0a 逐語契約に照らして私は採らない。
- **D**: 部分実装。ゴールデン両面一致が取れないので採らない。

---

## 5. 独自解釈

1. **`state-full.md` を既存ケースのディレクトリへ足した**。「既存ゴールデンのバイトは 1 バイトも
   変更禁止（追加のみ）」を「既存**ファイル**のバイトは動かさない、新規ファイルの追加は可」と
   読んだ。既存 22 ケースの観測ファイル（`argv` / `stdin` / `exit` / `stdout` / `stderr` /
   `state.diff` / `audit.md`）は 1 バイトも動いていない（§6 基準 8 の実測）。
2. **新ケースを列の末尾に足した**。`cli/` は 1 本の作業を頭から進めながら採るので、途中に足すと
   後続ケースの観測が全部動く。末尾なら先行ケースは採り直しても同一である。
3. **前方の境界ケースを追加採取した**（ブリーフの明示項目ではない）。後方だけ実装すると前方は
   上流ソースの読解に依拠することになり、「捏造しないが唯一の掟」に照らして弱いと判断した。
4. **`cases-missing.json` の既存 1 件の文面を書き換えた**（バイト変更）。`set-autonomy/gated` の
   `reason` / `evidence` / `follow_up` である。観測バイトではなく採取者の記録であり、内容が
   「テンプレートに行が無い」から「書き込む経路が 1 つも無い」へ**強化**されたので、古い記述を
   残すほうが害だと判断した。
5. **`cargo test` のフィクスチャ 2 箇所に `- **Lifecycle Phase**:` 行を足した**
   （`journal_protocol_conformance.rs` の合成状態ファイルと `projection.rs` の `SKELETON`）。
   実物の状態ファイルには元からある行で、合成側が最小骨格として省いていただけである。
6. **`audit_block_golden_test.rs` の期待ブロック数を 42 → 62 へ上げた**。ゴールデンが減ったのに
   緑のまま、を防ぐ番人なので、増えたぶんを反映しないと番人にならない。
7. **カバレッジ相対ゲートを一度 FAIL させ、防御的死にコードを削って直した**。最初に
   `--base main` で計測して PASS したが、ローカルの `main` が PR #21 時点まで古く、比較先が
   間違っていた。正しい `origin/main`（B9 のマージ後）で計り直すと head 98.4645% < base
   98.4801% − 0.01 で **FAIL** した。原因は私が足した 2 行の到達不能な防御分岐である。
   テストで踏みにいくのではなく**分岐そのものを消した**:
   - `PhaseId::ALL` の `position` が返す `Option` を、網羅 `match` の `phase_order` へ置換した。
     閉集合なので `None` は起きず、`match` ならフェーズを増やしたときコンパイルエラーで
     ここを直すよう強制できる（順序が `PhaseId::ALL` と一致することは単体テストが見張る）。
   - `last_completion_before` の「到達点の索引を引く → `take(索引)`」を
     `take_while(到達点の手前まで)` の前向き走査へ置換した。索引の `Option` が消える。
   結果 head 98.4805% >= base 98.4801% で PASS。**ローカルの `main` は古いことがあるので、
   相対ゲートは `origin/main` を基準に計ること**（次の担当者への申し送り）。

---

## 6. 受入基準

| # | 基準 | 判定 | 実測 |
|---|---|---|---|
| 1 | fmt（workspace + `tools/lint`） | **PASS** | `cargo fmt --all --check` 両方とも差分なし |
| 2 | clippy | **PASS** | `cargo clippy --workspace --all-targets -- -D warnings` 警告 0 |
| 3 | `cargo lint` | **PASS** | exit 0 |
| 4 | `cargo test --workspace` | **PASS** | **741 件**全緑（+4 = 投影ゴールデン検収 3 本 + `phase_order` の順序ガード 1 本） |
| 5 | quint-gate | **PASS** | 不変条件 run 3 種・witness 12 本すべて緑 |
| 6 | coverage 相対 | **PASS** | 絶対 **98.4805%** >= 90%、相対 98.4805% >= `origin/main` 98.4801% − 0.01。**一度 FAIL させてから直した** — §5-7 を参照 |
| 7 | unwrap 0 | **PASS** | プロダクトコードに `unwrap()` / `expect()` なし（clippy が機械強制） |
| 8 | 新採取に provenance が揃い、再実行で `captured_at` 以外の差分が出ない | **PASS** | 最終構成（25 ケース）で再採取を連続実行し、**296 ファイル**すべてが `captured_at` 以外バイト一致。新 3 ケースはいずれも `case.json`（`commit` / `captured_at` / `command`）と族単位の `provenance.json` を持つ |
| 9 | `ScaffoldTemplateUnavailable` の grep 0 件（採取成功時）／`cases-missing.json` に理由（不能時） | **条件付き** | §4 の裁定待ちのため型は残存。**採取は成功しているが実装が材料不足で止まっている**という、基準が想定していなかった第 3 の状態である。`cases-missing.json` には `set-autonomy` の理由を記録済み |
| 10 | 投影ゴールデン検収が B8 の 10 ケース + 新ケースで全両面一致 | **PASS** | **13 ケース**（10 + 新 3）すべて `audit.md` + `state.diff` の両面バイト一致。監査ブロック検収は 42 → **62 本** |

---

## 7. 申し送り

1. **§4 の裁定が最優先**。骨格生成と `Stages to Skip` はどちらもここで止まっている。裁定が A なら
   コード変更はほぼ doc とエラー名だけで済む。B なら `Started` の payload 変更なので契約
   （`contract-summary.md` C5 の `projects_to`）まで波及する。
2. **`GateApproved` の境界 3 本の `**Stages completed**:` が未検収**である。現在の実装は
   `plan.in_scope_count_of(from)` を書くが、ジャンプ側の実測は数え直しだった。ゲート経由で
   フェーズ境界をまたぐケースはゴールデンに無く、genesis の `Stages completed: 3` だけが両者
   一致するので判別が付かない。upstream の `handleAdvance` は数え直し
   （`String(completedCount)`）を書いているので、**実装のほうが怪しい**。追加採取するなら
   inception 最終ステージ（delivery-planning）を承認するケースになる。
3. **`cli/jump/execute-forward-to-conditional` が投影検収に接続されていない**（B8 から未接続）。
   本 Bolt では触っていない。
4. `docs/` / `formal/` / `coding-rules` には触れていない（所有外）。ゴールデン README
   （`tests/golden/upstream-3c3146cf/README.md`）は所有内なので更新済み。
5. **相対カバレッジは `--base origin/main` で計ること**。ローカルの `main` は PR #21 時点まで
   古く、`--base main` は 96.67% という無関係な値と比べて誤って PASS する（§5-7）。
6. 検証用の `target-delegate/` が未追跡で残っている（B8 の申し送り 4 と同じ）。
