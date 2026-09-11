# 切替条件 2 の判定へ持ち越す所見

切替条件 2 は「状態・監査が upstream 互換で機能する（ワークスペース配置、監査イベント語彙、
状態ファイルの形式、逐語文言）」である。U2 の実装中に見つかったが、裁定 Q1 / Q2 のどちらにも
含まれない所見をここへ集める。**U2 で直さず、切替条件 2 の判定時に改めて扱う。**

利用者の裁定: 「別件として記録する」（2026-09-10、構造化質問で明示選択）。

---

## F1: review-freeze の判定材料が本 build 固有の成果物に依存している

発見者: `u2_upstream_capture`（2026-09-10、実走行による採取）。

### 事実

本 build が組んで凍結が起きる記録から `.aidlc-execution`（**本 build 固有の集約スナップショット**）
だけを外すと、`requirements-analysis` でも `code-generation` でも `aidlc hook review-freeze` が
exit 0 で素通しする。

本家は同じ操作で挙動が変わらない。判定材料が `aidlc-state.md` と監査シャードだけだからである。

受領証の監査行そのものは両者で同形・同指紋だった（`Artifact Fingerprint` まで一致）。
**差は監査の中身ではなく、フックが何を読んで決めるかにある。**

### なぜ切替条件 2 に効くか

切替条件 2 は上流互換を要求する。監査と状態ファイルは互換でも、フックの判定が本 build 固有の
成果物を要求するなら、上流形式の記録だけを渡されたときに保護が働かない。ホストを本 build へ
切り替えたあと、上流が書いた記録・他ハーネスが書いた記録・スナップショットを失った記録に対して
凍結が素通しする経路が残る。

### U2 で直さない理由

per-unit 受領証の配線（裁定 Q1 = B）と同じ領域を触るため、同時に動かすと衝突する。
判定材料の設計変更は影響範囲が広く、U2 の完了を遠のかせる。

`u2_per_unit_policy` 担当には「この依存を**増やす方向へ寄せない**」とだけ伝えてある
（既存の依存を消す作業は今回の範囲外）。

### 判定時に確かめること

1. 上流形式の記録だけ（`aidlc-state.md` + 監査シャード）で本 build の review-freeze が
   本家と同じ判定に達するか。
2. 達しない場合、判定材料を上流形式へ寄せるか、`.aidlc-execution` を上流互換の一部として
   位置づけるか。後者を採るなら、他ハーネスと上流が書いた記録をどう扱うかを決める必要がある。
3. 同じ依存が review-freeze 以外のフックにもあるか（未調査）。

---

## F2: 採取値の鮮度（判定前に採り直すこと）

`u2_upstream_capture` の本 build 側の観測はすべて `target/debug/aidlc`
（2026-09-10 10:28、sha256 `4bb7ebad…`）に対するものである。以後の他担当の変更を含まない。

**切替条件 2 を判定する直前に採り直すこと。** 採取スクリプトは
[upstream-capture-logs/](upstream-capture-logs/) にあり、そのまま走る。

## F3: 比較基準の取り出し方（手順として固定する）

`vendor/aidlc-workflows` の HEAD は `801c5700` で**固定コミット `a277af21` ではない**。
対象 6 ファイルのうち `aidlc-lib.ts` と `aidlc-state.ts` の 2 本が固定コミットと異なる
（`.kimi-code` のハーネス登録の追加のみ）。残り 4 本はバイト一致。

したがって本家と比べるときは vendor の HEAD を直接使わず、`git archive a277af21 dist/claude` で
固定コミットの実体を取り出し、リポジトリ既存の `verifySource` で 277 ファイル・マニフェスト
`282b17c5…` の一致を確認してから使う。`tests/golden/upstream-a277af21/source.json` の固定ピンと
同じ値である。

## 未採取のまま残るもの

1. **秒未満の窓**（Q2 の残り）。本家は再承認の `completed_at` が既存行と同じ秒に収まると抑止する。
   本 build は承認時刻を持たないので増えるはずだが、1 周に 5〜6 秒かかるため同じ秒の履歴を作れず
   未測定。
2. **Q1 の Unit 有りの側の本 build**。`aidlc-bolt start` が未配線で状態を作れない。
3. **本家のゲート段の一周**（`aidlc-log question` 相当まで駆動していない）。

---

## U2 着地時に追加した所見（2026-09-10 午後）

Step 8 の棚卸し（[step8-migration-verification.md](step8-migration-verification.md)）とフック 3 スライスの着地確認（[reviewer-scope-verification.md](reviewer-scope-verification.md)、[stage-rules-verification.md](stage-rules-verification.md)、[fold-usage-verification.md](fold-usage-verification.md)）で見つかり、利用者の裁定（[step8-decisions-questions.md](step8-decisions-questions.md)、[hook-closeout-decisions-questions.md](hook-closeout-decisions-questions.md)）で「U2 では直さず記録する」と決まったもの。契約で是正が決まった差（D1〜D6、監査値の伏せ字、状態欄のタブ、唯一記録への後退、規則 BOM、Stop の flush-all、差し向け記録の逐語強制）はここに含めない。

### F4: 2.7.1 コーパスの未駆動ケース（裁定 Q4 = A）

classic scope の採取列のうち、承認・jump・skip を挟む連鎖の再現が要るため U2 ではテストへ固定しなかったもの。`next/after-approval`、`jump/execute-forward-to-conditional`、`jump/resolve-forward`（前提が異なる）、`practices-promote/affirm`（stdout / state / audit）、`set-autonomy/state-field-absent`（stdout / audit）、`report/approved-across-phases` の CLI 面、`hooks/stop-forwarding-loop/{block-pending-directive,reentrant-ignored}` の classic 用 stdin、`hooks/write-audit-log/*` 5 ケースと `hooks/record-human-turn/active-workflow` の監査バイト、`supplemental/cases.json` の `cli/continue/multi-part` の native 配送。同型は bugfix scope の 2.7.1 観測（`stage1/cases.json`、`selfhost-stage1/*.json`）で全文一致を検査している。`cli_golden_test.rs` に残る `next/start` のキー集合比較、report 5 ケースの slug 置換、`awaiting-approval-repeat` のセンサー再発火（裁定 A で対象外）も同じ扱い。判定前にこれらを駆動するかを決める。

### F5: `deliver-stage-rules` の `tool_name` が `null` / 非文字列のとき（裁定 Q2 = A）

本家は未処理例外（`TypeError` の握り漏れ）で exit 1、本 build は素通し（exit 0、出力なし）。例外の握り漏れは契約ではないと判断し、本 build の挙動を保った。実走行 64・72。

### F6: stderr の丸括弧内のエラー原因文言（裁定 Q3 = A）

規則ファイルが読めないときの `Cannot load required stage rule "<path>" (<原因>). …` の丸括弧内が、本家は Node の文言（`ENOENT: no such file or directory, open '<abs>'`）、本 build は Rust の文言（`No such file or directory (os error 2)`）。固定部分は一致。実走行 89 件中 2 件と 512KiB 境界 3 件がこの差だけで不一致。OS エラーの表現は本家でもプラットフォーム依存であり、固定部分の一致を契約とした。

### F7: 完了監査の利用量欄（裁定 Q4 = A）

利用量追跡が有効なとき、本家は `STAGE_COMPLETED` / `WORKFLOW_COMPLETED` へトークン数・キャッシュ数・Cost USD・モデル/agent 別内訳を足す（`aidlc-state.ts` の `stageUsageAuditFields` / `workflowUsageAuditFields`）。本 build の RMU は描かない。U1 の採取 75 観測はすべて追跡無効で欄のある観測が無く、`fold-usage.json` は監査を含まない。欄は利用量があるときだけ付く任意項目。判定前に固定コミットで有効時の `approve` / `complete-workflow` を採取して実装するかを決める。fold-usage の producer 側（台帳）は本家採取 40 観測でバイト一致済み。

### F8: fold-usage の採取に無い経路の既知差

sub-agent の畳み込み順を辞書順に固定（本家は `readdirSync` 順）、cursor 鍵の `path.join` 正規化を再現していない、旧形 `<slug>-<id8>` の registry 照合は未再現、`JSON.parse` と serde / canon の極端入力差。いずれも本家採取 40 観測では踏まない。

### F9: `deliver-stage-rules` の未駆動・対象外の分岐

(1) 本家はグラフとペルソナをフックのスクリプト相対で読み、本 build は `<project>/.claude/` 固定（配布形では一致する見込み。U4 の hook 登録と `--project-dir` の方針に依存）。(5) `AIDLC_RULES_DIR`（Codex ハーネスの規則ディレクトリ指定）は未実装（他ハーネスは対象外）。(7) `run_in_background` の inflight 更新（`markSubagentInflight`）は未実装（棚卸しどおり通常スモークに追加しない）。

### F10: reviewer-scope の未実測の分岐

監査シャードが無い状態（本家は行を書かず拒否だけ）、監査ロック競合、team unit ownership の claimed-checkout（未移植・到達不能）、ゼロ Unit の非適用分岐、`aidlc-bolt start` 未配線のため Unit 有りの状態を本 build の CLI で作れないこと。`GuardReviewerScopeUseCase` に単体テストが無い（子プロセス面 12 件と 32 ケースの実走行で固定）。
