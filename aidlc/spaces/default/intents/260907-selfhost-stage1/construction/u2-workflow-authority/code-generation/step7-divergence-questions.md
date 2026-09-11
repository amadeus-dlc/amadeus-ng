# Step 7 フック接続で見つかった本家との差の裁定

2026-09-10。`u2_runtime_graph` と `u2_review_guards` の 2 スライスで、上流仕様（本家 2.7.1、
固定コミット `a277af218f0df7f325d3b8be7b6d90fce2c5bd40`）と現行コードの差が 2 件見つかった。
どちらも読み替えずに人間の裁定を求める（`memory/project.md` の Mandated による）。

実装は止めていない。この 2 件はいずれも進行中の他スライスの範囲外である。

**受領記録について**: `aidlc-log.ts decision` は `--checkpoint` に `summary-confirmation` と
`plan-approval` の 2 つしか受け付けないため、この裁定質問には受領を記録していない
（`Unknown --checkpoint "step7-divergence"` で拒否された）。本ファイルと会話が記録である。

---

## Q1: per-unit 受領証を配線するか（review-freeze の凍結範囲）

### 差の所在

- 本家の review-freeze は **Unit ごとの受領証地図**を持ち、ステージ水準の書込みでは Unit 側だけを
  見る。したがって **ゼロ Unit の per-unit ステージでは凍結しない**。
- 本 build の `ReviewPolicy::per_unit` は既存 doc が「未配線」と明記しており、`ReviewAttempt` は
  ステージ 1 つに 1 つである。per-unit ステージでも同じステージの試行を見るため、**ゼロ Unit でも
  凍結する**。
- 今回の依頼は「ゼロ Unit でも requirements-analysis / code-generation に適用」であり、担当は
  本 build の受領証の持ち方に沿って実装した。兄弟 Unit の分岐は依頼どおり対象外である。

差は「本 build のほうが厳しい」向きである。保護が過剰なだけで穴ではないが、切替条件 2
（状態・監査が upstream 互換）の判定材料になる。

根拠: `review-guards-verification.md` の「確認できなかった範囲・写していない分岐」、
本家 `hooks/aidlc-review-freeze.ts:250-375`、現コード
`modules/core/command/domain/src/orchestration/review_closures.rs`、`review_attempt.rs`。

### Q1: この差をどう扱うか

- A. 現状を維持する — ゼロ Unit でも凍結する厳しい側のままにし、差を既知として記録する。配線は行わない。
- B. `ReviewPolicy::per_unit` を配線する — Unit ごとの受領証地図を持たせ、本家と同じ凍結範囲へ揃える。実装量は大きい。
- C. 切替条件 2 の判定前に、本家フックの実走行を採取して差の有無を実測してから決める。
- X. Other (please specify)

[Answer]: B. `ReviewPolicy::per_unit` を配線する

経緯: まず C（実走行を採取して決める）を選択し、`u2_upstream_capture` が採取した。
その実測を見て利用者が B を選択した（いずれも 2026-09-10、構造化質問で明示選択）。

### 採取結果（決定的）

固定コミット `a277af21` の `dist/claude` 全 277 ファイルが固定マニフェスト `282b17c5…` と
バイト一致することを先に確認したうえで、同一手順・同一入力を流した。

ゼロ Unit の per-unit ステージ（`bugfix` scope の `code-generation`、stage 水準の終端受領証あり）へ
`produces` を書いたとき:

| 側 | 結果 |
| --- | --- |
| 本家 review-freeze | exit 0、`REVIEW_FREEZE_BLOCKED` なし（**凍結しない**） |
| 本 build `aidlc hook review-freeze` | exit 2、`REVIEW_FREEZE_BLOCKED` 1 行（**凍結する**） |

`code-generation-plan.md` と `code-summary.md` の 2 宛先で同じ差。日誌 `memory.md` は両者とも通し、
受領証なしの陰性対照も両者とも通したので、差は「終端受領証があるゼロ Unit の per-unit ステージの
produces」に限局している。

**射程が裁定を決めた**: `bugfix` scope は `units-generation` が SKIP なので、本家の
`usesStageLevelPerUnitArtifacts` により per-unit ステージの成果物はそもそも stage 水準のパスへ置かれる。
つまりこの差は例外的な構成ではなく `bugfix` / `express` / `poc` / `refactor` / `security-patch` の
**通常経路**であり、本 intent の完了条件である bugfix 相当の実地スモークがまさにこの経路に当たる。

未採取: Q1 の Unit 有りの枝。本 build は `aidlc-bolt start` が未配線で
（「the start subcommand is not wired in this build. Only `set-autonomy` is available.」）、
本 build を Unit 有りの状態へ持って行く CLI 経路が存在しないため。

---

## Q2: `MEMORY_EMPTY` の重複抑止の鍵をどう扱うか

### 差の所在

- 本家の runtime compile は `(slug, completed_at)` を鍵に `MEMORY_EMPTY` を 1 件だけ記録する。
  再跳躍して承認し直した位置には改めて記録する（`tools/aidlc-runtime.ts:786-804`）。
- 本集約は**承認時刻を持たない**。担当は「承認へ倒れたこと」を新しい承認の印として扱い、
  同じ観測可能な振る舞いを得る設計にした。承認以外の進捗（TaskUpdate による同期など）では
  印を落とさない。
- 観測上は等価だという主張であり、**上流の実走行との突き合わせは行っていない**。

根拠: `runtime-graph-verification.md`、本家 `tools/aidlc-runtime.ts:786-804`、
現コード `modules/core/command/domain/src/orchestration/stage_slot.rs`、`stage_slots.rs`。

### Q2: この設計を受け入れるか

- A. 受け入れる — 観測等価の主張を採り、実走行での突き合わせは切替条件 2 の判定時にまとめて行う。
- B. 先に実走行を採取する — 本家フックを固定コミットで走らせ、再承認時の記録の有無をバイトで確かめてから確定する。
- C. 集約に承認時刻を持たせる — 本家と同じ鍵にして差そのものを無くす。ドメインの変更を伴う。
- X. Other (please specify)

[Answer]: A. 観測等価を受け入れる

経緯: まず B（先に実走行を採取する）を選択し、`u2_upstream_capture` が採取を完了した。
その実測を見て利用者が A を選択した（いずれも 2026-09-10、構造化質問で明示選択）。

### 採取の結論（測れた範囲で主張は正しい）

駆動できたすべての履歴で件数が一致した。初回 compile で 1 件、再発火で増えない、
TaskUpdate 同期で増えない、再跳躍して承認し直すと改めて記録する、の 4 点である。

さらに決定的だったのは、**本 build 自身が残した時刻表へ本家の鍵
（`既存行の timestamp >= completed_at`）を当てても同じ件数になる**ことである。本 build の
`completed_at` は 03:37:06 → 11 → 17 と更新され、`MEMORY_EMPTY` は 08 / 12 / 18 に立った。
本家の鍵で計算しても 3 件になる。

**未測定（推測で埋めていない）**: 秒未満の窓。本家は再承認の `completed_at` が既存行と同じ秒に
収まると抑止することを実測した。本 build は承認時刻を持たないので増えるはずだが、1 周に 5〜6 秒
かかるため同じ秒の履歴を作れず測れていない。実運用で起きにくい条件と判断して A を採った。

経路は片方ずつしか通せなかった。本 build は初期化段の `advance` が未配線で `state-init` を
再承認へ戻せず、本家はゲート段で要約確認の質問記録を要求するため一周できない。そこで本家は
`state-init`、本 build は `requirements-analysis` で採っている。

以下は採取前の記録である。

### 採取結果（13 段中 11 段は一致、残り 2 段は比較不能）

生データは [upstream-capture-logs/q2-memory-empty.json](upstream-capture-logs/q2-memory-empty.json)。

| 段 | 内容 | 一致 |
| --- | --- | --- |
| 01–04 | 初期化、空日誌の設置、1 回目の compile（1 件記録）、2 回目の compile（増えない） | 一致 |
| 05–06 | TaskUpdate 同期と、その後の compile | 一致 |
| 07–09 | 同じ秒での再跳躍・再承認・compile | 一致 |
| 10 | 時刻を空けた再跳躍 | 一致 |
| 11 | 時刻を空けた再承認 | 一致（ただし下記） |
| 12 | 再承認後の compile。本家 2 件、本 build 1 件 | **不一致** |
| 13 | 日誌が埋まった位置の compile | **不一致** |

**12・13 の不一致を「観測等価の反証」と読んではならない。** 段 11 で本 build は
`Cannot commit advance for "state-init": the advance transition is not wired in this build.` を返し、
**再承認自体が成立していない**。したがって 12・13 は「再承認した本家」と「再承認できなかった本 build」を
比べており、`MEMORY_EMPTY` の鍵の設計を検証したものではない。

**確定していること**: TaskUpdate 同期で記録が増えないこと（段 05–06）と、同じ秒での再承認の扱い
（段 07–09）は本家と一致する。観測等価の主張のうち、この範囲は裏付けが取れた。

**未検証のまま残ること**: 時刻を空けた再承認で本家が改めて記録する振る舞い（段 12）に本 build が
追随するか。`advance` 遷移が未配線である限り駆動できない。

### 派生して見つかった未配線（調査の結果、課題ではなかった）

段 11 の拒否文言から `advance` と `complete-workflow` の 2 遷移が未配線であることが分かり、
親は当初「実地スモークの完了条件に直結する」と評価した。**この評価は誤りだったので取り消す。**

`modules/core/command/use-case/src/orchestration/commit_error.rs:47-49` が理由を明記している。

> `advance` / `complete-workflow` の 2 段が該当する — 非ゲート完了のパイプラインは b42 で
> 撤去した（#85 = A）。**初期化ステージだけが in-scope の縮退計画でだけ到達する。**

つまり意図的な撤去であり、到達するのは初期化ステージだけが in-scope の縮退計画に限られる。
採取スクリプトが `state-init` だけの縮退計画を組んだため、この経路を踏んだ。

実地スモークは bugfix scope の実ステージ列を通るのでゲート付き完了の経路を使い、撤去された
非ゲート完了のパイプラインには到達しない。**したがって完了条件を脅かさない。**

この一件は、拒否文言を見ただけで欠落と判断してはならない例として記録する。撤去の意思決定
（b42 / #85 = A）が実コードのコメントに残っていた。

---

## 参考: 裁定ではなく後続スライスで閉じるもの

次の 3 点は差ではなく未移植であり、裁定ではなく実装で閉じる。担当割当て済み・予定済みである。

1. **シェル経由の書込みが凍結されない** — 本家 `shellWriteTargets`（約 400 行）が未移植。
   現状は `Bash` を書込み先 0 件として通し、10 分に 1 度 drop へ「未検査で通した」と残している。
   `u2_shell_write_targets` 担当へ委譲済み。
2. **reviewer-scope の越境拒否と `REVIEWER_SCOPE_BLOCKED` の保存** — 未移植。差し向け記録が在る
   呼出しは通し、レビュー専用エージェントの呼出しに限り drop へ「強制が未配線」と残している。
3. **本家フックの実走行採取** — `tests/golden/selfhost-stage1/` に review-freeze / reviewer-scope の
   採取が無い。突き合わせは固定コミットのソース読解と本 build のプロセス実測に限る。
