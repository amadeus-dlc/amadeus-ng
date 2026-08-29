# B10 開発者報告 1 — U1 ゴールデン追加採取と RMU 投影の完成

Conversation language: 日本語
担当: 委任先（Opus）/ ブランチ: `bolt/b10-u1-golden-recapture` / 報告日: 2026-08-29
検証はすべて `CARGO_TARGET_DIR=$PWD/target-delegate` で実行。push はしていない。

---

## 0. 要約

**B10 は完了**。ブリーフの採取 4 点のうち 3 点は実バイトを採れた。残る 1 点
（`AUTONOMY_MODE_SET` 成功経路）は**ピンでは到達不能**であることを全数走査で確定し、
捏造せず `cases-missing.json` に根拠を残した。

その過程で、ブリーフが「採取さえ済めば実装できる」と見込んでいた**骨格生成が、採取とは
別の理由で詰まっている**ことが分かった。バイトは採れたが、**ジャーナルに無い材料が 4 つ
ある**（うち 1 つは環境の絶対パス）。読み替えず止めて裁定を求め、**オーナー裁定 A**
（骨格生成は投影の責務外 — 書くのは合成ルート、実装は U7）を受けて実装を確定した（§4）。

さらに最終ラウンドで、付帯確認から見つかった B8 由来の乖離 3 件を**実バイトで確定して
是正**した（§6-2）。うち 1 件は**私の見立てが誤り**で、採取した実バイトが否定した。

- コミット 9 本 / 採取ケース 22 → **28**、監査ブロック検収 42 → **70** 本
- 投影ゴールデン検収 10 → **19 ケース**（`audit.md` + `state.diff` の両面バイト一致）
- テスト 737 → **744 件**（全緑）、カバレッジ **98.5165%**（`origin/main` の 98.4801% から +0.036pt）
- 受入基準 **1〜10 すべて PASS** — **最終コミット状態で測り直した値**（§6）。
  最終ラウンド直後は測り直しを怠って provenance を陳腐化させ、委任者の独立検証で差し戻された
  （§5-10）

---

## 1. 採取できた点

| # | ブリーフの項目 | 結果 | 実バイトの置き場 |
|---|---|---|---|
| 1 | 状態ファイルの骨格 | **採取済み** | `cli/intent-create/classic-scope/state-full.md`（102 行全文） |
| 2 | 非ゲート `StageCompleted` の単独経路 | **採取済み** | `cli/report/completed-ungated` |
| 3 | `AUTONOMY_MODE_SET` 成功経路 | **到達不能**（§2） | `cli/cases-missing.json` |
| 4 | 途中で見つかる他の未採取バイト | **5 件採取** | `cli/jump/execute-backward`, `cli/jump/execute-forward-across-phases`, `cli/report/approved-across-phases`, `cli/recompose/skip-two-appends-in-graph-order`, `cli/recompose/add-restores-conditional` |

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

### 3-2 裁定 A に従って**書かない**と確定した部分

`ProjectionError::ScaffoldTemplateUnavailable` は**撤去せず改名**した
（`ProjectionError::ScaffoldMissing`、`Display` は `scaffold missing`）。意味論が
「テンプレートの実バイトが無い（採取待ち）」から「**骨格が無いのは投影の前提違反**」へ
変わったためで、名前が古い理由を指したままだと次の読み手が「採取すれば直る」と誤読する。

doc に書いたこと:

- 投影の責務は**既存本文への差分適用に徹する**ことであり、骨格を起こすことは含まれない。
  骨格は intent-create の時点で**合成ルート**が書く（環境と両側を知ってよい唯一の場所。実装は U7）。
- これは導出の工夫不足ではなく**構造から従う**。`- **Project Root**:` は環境の値でジャーナルに
  存在せず、投影が書けるようになる道は「環境を読む」か「環境パスをイベントへ載せる」の 2 つしか
  ない。前者は投影核の定義を壊し、後者は ADR-008 と NFR3 の趣旨に反する。**書けないのではなく、
  書く場所がここではない**。
- **NFR3 の適用範囲**: 冪等な再構成が保証するのは**差分適用**である。骨格はその保証の対象では
  なく環境成果物であり、全損したら再生成ではなく upstream 同様 archive & recreate の運用に載る。
- 骨格の実バイトは `cli/intent-create/classic-scope/state-full.md`（102 行全文）にあり、U7 が
  骨格を書くときの正本になる。

`- **Stages to Execute**:` / `- **Stages to Skip**:` の 2 行も触っていない。どちらも骨格の行で
あり、裁定 A により自動的にスコープ外である。イベント拡張は**していない** — `Started` は現状の
ままである。

---

## 4. 止めて報告した点と裁定 — 骨格生成に材料が 4 つ足りない

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

提示した選択肢のうち、**オーナー裁定は A**（骨格生成は投影の責務外。書くのは合成ルート、
実装は U7）。決め手は `Project Root` が環境値であることで、これは「導出の工夫が足りない」の
ではなく「骨格生成はそもそも投影ではない」ことの証明である。B（環境パスをイベントへ載せる）は
ADR-008 / NFR3 の趣旨に反するため不採、C（slug ハードコードで近似 — `infra` + greenfield で
静かに割れる）と D（部分実装）も不採。実装は §3-2 のとおり確定した。

裁定により (A)(B)(D) の 3 行はすべて骨格行としてスコープ外になり、残る畳み理由も骨格行なので
同様である。**イベント拡張は不要になった**。

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
7. **注釈の区切りに `BOUNDARY_ARROW` を誤用した**。Skip 行の注釈は em dash（` — `）だが、
   フェーズ境界用の矢印定数（` → `）を使ってしまい、`2.1 (reverse-engineering — greenfield)` から
   slug を取り出せなかった。単体テストが捕まえたので専用定数へ分けた。**この 1 本を書いて
   いなければ、ゴールデン検収だけでは通っていた可能性がある**（当該ケースでは結果的に項目が
   保存されず、たまたま期待どおりになりうる）。
8. **単体テストの骨格フィクスチャが自己矛盾していた**。`- **Stages to Execute**: 0.1, 2.1` と
   書きながらチェックボックス行は `second` も `— EXECUTE` としており、追記方式では露見しないが
   組み直し方式では 2.2 が現れる。フィクスチャ側を実態に合わせて是正した。
10. **最終ラウンド後の再測定を怠り、provenance の陳腐化を委任者の独立検証で指摘された**。
   再現性検証の最後に `git checkout -- tests/golden/` で採取物をコミット前の状態へ戻したが、
   これは**再生成された族 `provenance.json` まで巻き戻していた**。結果、`cli/provenance.json` が
   `case_count: 25` のままディスクの 28 ケースと食い違い、
   `golden_corpus_read::both_families_carry_their_provenance` が赤になった。私が「744 件全緑」を
   測ったのはこの巻き戻しの**前**であり、**コミット状態の測定ではなかった**。
   教訓は 2 つある: (a) 採取物を生成するスクリプトを走らせたあとに `git checkout` で戻すと、
   意図した「観測バイトの復元」だけでなく**集計メタデータの巻き戻し**も起きる。戻すのではなく
   再生成した状態をそのままコミットするのが正しい。(b) **受入基準は最終コミット状態で測り直す**。
   途中で測った値は、そのあと 1 バイトでも動けば根拠にならない。今回は全 10 基準を測り直した。
11. **カバレッジ相対ゲートを一度 FAIL させ、防御的死にコードを削って直した**。最初に
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

**下表はすべて最終コミット状態（作業ツリーがクリーンな状態）で測り直した値である。**
最終ラウンド直後に測り直しを怠って差し戻された経緯は §5-10 に記録した。

| # | 基準 | 判定 | 実測 |
|---|---|---|---|
| 1 | fmt（workspace + `tools/lint`） | **PASS** | `cargo fmt --all --check` 両方とも差分なし |
| 2 | clippy | **PASS** | `cargo clippy --workspace --all-targets -- -D warnings` 警告 0 |
| 3 | `cargo lint` | **PASS** | exit 0 |
| 4 | `cargo test --workspace` | **PASS** | **744 件**全緑（+7 = 投影ゴールデン検収 6 本 + 順序ガード / トークン解析ガード） |
| 5 | quint-gate | **PASS** | 不変条件 run 3 種・witness 12 本すべて緑 |
| 6 | coverage 相対 | **PASS** | 絶対 **98.5165%** >= 90%、相対 98.5165% >= `origin/main` 98.4801% − 0.01。**一度 FAIL させてから直した** — §5-7 を参照 |
| 7 | unwrap 0 | **PASS** | プロダクトコードに `unwrap()` / `expect()` なし（clippy が機械強制） |
| 8 | 新採取に provenance が揃い、再実行で `captured_at` 以外の差分が出ない | **PASS** | 最終構成（28 ケース）で再採取を連続実行し、**320 ファイル**すべてが `captured_at` 以外バイト一致。族 provenance とディスクの整合も機械確認済み（cli 28/28・欠落 2/2、hooks 14/14・欠落 1/1）。**一度この整合を壊して差し戻された** — §5-10 |
| 9 | `ScaffoldTemplateUnavailable` の grep 0 件（採取成功時）／`cases-missing.json` に理由（不能時） | **PASS** | 旧名の grep は `modules/` / `scripts/` / `tests/` で **0 件**（`ScaffoldMissing` へ改名）。撤去ではなく改名なのは**裁定 A がそう定めたため**である（§3-2）。`cases-missing.json` には `set-autonomy` の理由を記録済み |
| 10 | 投影ゴールデン検収が B8 の 10 ケース + 新ケースで全両面一致 | **PASS** | **19 ケース**（10 + 新 6 + 既存の接続漏れ分）すべて `audit.md` + `state.diff` の両面バイト一致。監査ブロック検収は 42 → **70 本** |

---

## 6-2. 裁定の付帯確認 — 計画 2 行を差分適用側が書き換えていないか

裁定時に「`recompose` / `jump` のゴールデン `state.diff` が
`- **Stages to Execute**: ` / `- **Stages to Skip**: ` を書き換えていないこと」の明示検証を
求められた。`state.diff` を持つ **cli 全 25 ケース**を機械的に走査した結果は次のとおりである。

| ケース | 2 行との関わり |
|---|---|
| `jump/execute-forward`, `execute-forward-to-conditional`, `execute-backward`, `execute-forward-across-phases` | **触れていない**（ハンクに現れもしない） |
| 上記以外の 20 ケースのうち 18 件 | **触れていない** |
| `practices-promote/affirm` | ハンクに**文脈行として写っているだけ**（変更行 0） |
| `intent-create/classic-scope` | 変更行 2 — ただしこれは骨格が生まれる遷移であり、裁定 A で合成ルートの仕事になった。差分適用ではない |
| `recompose/skip-one` | **変更行 4**（`4.5 (incident-response)` の移動）— 差分適用側で 2 行を書き換える唯一のケース |

したがって**ジャンプは 4 ケースとも触れていない**。書き換えているのは `recompose` だけである。
ただし**畳み理由 (D) はここで再浮上しない**。投影は既存の行から項目を出し入れするだけで、
`— greenfield` を**構築しない** — `2.1 (reverse-engineering — greenfield)` は文字列のまま
素通りする。したがって畳み理由を作れないという §4 の制約は差分適用側には効かない。

一方、この付帯確認で **B8 由来の潜在的な乖離を 2 つ**見つけ、§7-2 の 1 件と合わせて
**計 3 件を最終ラウンドで実バイト採取して是正した**。以下は採取後の確定内容である。

### (i) 並び順 — **私の見立てが誤りだった**

当初「upstream はグラフ順に組み直すが投影は末尾へ追記するので割れる」と書いた。
`cli/recompose/skip-two-appends-in-graph-order` を採取した結果、**upstream も追記していた**。

```
前: … 2.1 (reverse-engineering — greenfield), 4.5 (incident-response)
後: … 2.1 (reverse-engineering — greenfield), 4.5 (incident-response), 4.3 (deployment-execution), 4.7 (feedback-optimization)
```

4.5 の**後ろ**に 4.3 が来る。番号順に並べ替えていない。upstream 自身のコメントが理由を
書いている — 項目が持つ注釈（`2.1 (reverse-engineering — greenfield)`）を bare-slug の
組み直しが壊すため、**まだ skip の項目は逐語・その位置のまま**保つのが仕様である。
B8 の追記は正しかった。

ただし採取は**別の乖離**を露わにした。**`Stages to Execute` は逆に毎回 graph 順へ組み直される**
（`add-restores-conditional` で再投入した `2.1` が末尾ではなく `0.3` と `2.2` の**間**へ入る）。
2 行は同じ規則で作られていない。投影は両方とも追記していたので、Execute 側が割れていた。

### (ii) `reverse-engineering` の再投入 — 見立てどおり割れていた

`cli/recompose/add-restores-conditional` の実バイトは、`2.1 (reverse-engineering — greenfield)`
が**注釈ごと** Skip 行から消えることを示した。投影は `2.1 (reverse-engineering)` を除去キーに
していたので一致せず、`remove_from_list` の**無言 no-op** で両方の行に載っていた。

是正は「不在なら拒否へ寄せる」ではなく**経路ごと削除**した。上の (i) を踏まえて 2 行を
upstream と同じ「組み直し」で書くようにしたところ、項目の出し入れをする助数詞が不要になり、
`remove_from_list` と `append_to_list` がまるごと dead code になったためである。無言ドリフトを
厳格化するより、**無言ドリフトが起こりうる関数を消す**ほうが強い。

### (iii) `**Stages completed**:` — 見立てどおり実装が誤っていた（§7-2 の件）

`cli/report/approved-across-phases`（inception 最終 = delivery-planning の承認）の実バイトは
**2**。計画上の inception 内スコープ件数は 8 なので、`plan.in_scope_count_of` では説明が付かず、
**倒したあとのチェックボックスの数え直し**であることが確定した。genesis だけは計画由来のまま
である（まだ 1 つも倒れていない時点で描くため）ので、値を呼出側が渡す形へ変えた。

同じケースで、ゲート経由の境界 3 本が**ジャンプ側と違い `**Details**:` を持たない**ことも
確定した。さらに `## Phase Progress` の `Verified` / `Active` の付け替えがゲート経路で
**未実装**だったことが判明し、実装した（フェーズをまたぐ承認が初めてこの差分を見せた）。

## 7. 申し送り

1. **U7（合成ルート）が骨格を書く**。正本は `cli/intent-create/classic-scope/state-full.md` の
   102 行である。`- **Project Root**:` / `- **Project**:` / `- **Review Override**:` /
   `- **Stages to Skip**:` の畳み理由の 4 行は、合成ルートなら（環境と素のグリッド値の両方を
   知っているので）すべて書ける。`Started` の payload は変更していないので契約
   （`contract-summary.md` C5）への波及は無い。
2. **`GateApproved` の境界 3 本の `**Stages completed**:` は是正済み**（最終ラウンド）。
   `cli/report/approved-across-phases` を採取して数え直しであることを確定し、実装を直した。
   同ケースが `## Phase Progress` のゲート経路での未実装も露わにしたので、そちらも実装済み。
3. **§6-2 の乖離 3 件は是正済み**（最終ラウンド）。残る既知の未確定は次の 1 点だけである —
   recompose の `- **Completed**:` を upstream は書き換えるが（`completed && eff == EXECUTE` の
   数え直し）、採取した 2 ケースではどちらも値が動かないため実装の要否を実バイトで判別できない。
   投影は書いていない。ピン更新か、完了済みステージを skip する新ケースを採れば決着する。
4. **`cli/jump/execute-forward-to-conditional` が投影検収に接続されていない**（B8 から未接続）。
   本 Bolt では触っていない。
5. `docs/` / `formal/` / `coding-rules` には触れていない（所有外）。ゴールデン README
   （`tests/golden/upstream-3c3146cf/README.md`）は所有内なので更新済み。
6. **相対カバレッジは `--base origin/main` で計ること**。ローカルの `main` は PR #21 時点まで
   古く、`--base main` は 96.67% という無関係な値と比べて誤って PASS する（§5-7）。
7. 検証用の `target-delegate/` は PR 収束時に委任者が削除済み（測定はもともと追跡対象のみのクリーン状態で実施 — 未追跡の検証ディレクトリは測定に影響しない）。
