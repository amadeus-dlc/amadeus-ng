# Step 8 棚卸しで見つかった差の裁定

2026-09-10。`u2_step8_audit` の棚卸し（[step8-migration-verification.md](step8-migration-verification.md)）と、親が主セッションで実行した classic scope の probe（[step8-logs/native-classic-probe/out/](step8-logs/native-classic-probe/out/)、本家固定コミット `a277af21` の配布シェルだけを置いた作業ツリーで `next --scope classic` → `intent-create` → `next` → `continue` → `next --stage contract-design` を本 build と本家採取で突き合わせ）に基づく。

## probe で確定した本家との差（契約 C1 / C2 の「既定はバイト一致」で是正が決まるもの。裁定不要）

| # | 面 | 本家 2.7.1（採取） | 本 build（probe） |
| --- | --- | --- | --- |
| D1 | 作業が 1 件も無い（`intents/` 不在）ワークスペースでの `next --scope classic` | `{"kind":"print","message":"Run \`bun .claude/tools/aidlc-utility.ts intent-create --scope classic\` to start the workflow (25 of 33 stages, 22 approval gates, 5 stages repeat per unit of work in Construction), then re-run \`next\` to continue.","narration":"Setting up a classic workflow for this: …"}` | `{"kind":"error","message":"aidlc-orchestrate: cannot open the workflow definition repository: io: NotFound at <ROOT>/aidlc/spaces/default/intents/."}` |
| D2 | Greenfield の classic で `intent-create` | `2.1 (reverse-engineering — greenfield)` を SKIP、25 stages、`practices-discovery` へ routing、Active Agent `aidlc-pipeline-deploy-agent` | `Project Type: Greenfield` と監査に書きながら reverse-engineering を EXECUTE、26 stages、`reverse-engineering` へ routing、Active Agent `aidlc-developer-agent` |
| D3 | `WORKFLOW_STARTED` / `WORKSPACE_SCAFFOLDED` / `WORKSPACE_INITIALISED` の `Request` 欄 | `/aidlc Build a small ordering service`（本家 `aidlc-utility.ts:5550,5625,5683,5971` の `` `/aidlc ${flags.arguments || scope}` ``） | `Build a small ordering service` |
| D4 | `load-steering` の `bundle` | `sha256:cc98…`（`sha256:` 接頭辞付き） | `29657b55…`（接頭辞なし） |
| D6 | 承認待ちの工程への再報告（`report --result awaiting-approval` の再提示） | `spawnState(pd, ["gate-start", slug])` で根拠を再検証してから `… gate evidence revalidated.` を出す（`aidlc-orchestrate.ts:8040-8068`） | 集約が `AlreadyAwaiting` の no-op を返し、再検証せず同じ文言だけを出す（`intent_execution.rs:3300-3304`、`runtime.rs:918-921`） |

`Source Baseline` は配布シェルだけの作業ツリーで両者とも空ハッシュ `e3b0c4…` で一致した（前 2 回の probe で見えた差は probe 側の配置ミスであり、本 build の差ではない）。

## Q1: print directive が名指すコマンドの綴りと、`next --stage` の意味（R3）

[Question]: `next` が返す print directive のコマンド綴りを本家 2.7.1 に揃えますか？

本 build は `engine_command.rs:87-104` で 2.7.1 観測に合わせ `bun .claude/tools/aidlc-utility.ts …` を出す一方、同 `:105-108,120-155,185-199` と `wording.rs:692-695` は削除済みの「逸脱台帳 #1」を根拠にマルチコール形（`aidlc-jump resolve`、`aidlc-utility intent-create` 等）を出す。probe の実出力:

- `next --stage contract-design`: 本家 `Run \`bun .claude/tools/aidlc-jump.ts execute --target contract-design --direction forward --scope classic\` to perform the jump, then re-run \`next\` to continue from the jump target.` / 本 build `Run \`aidlc-jump resolve --stage contract-design\`.`（綴りだけでなく、本家は `next --stage` の中で方向・scope を解決して **execute** を名指すのに対し、本 build は **resolve** を名指す）
- `next --scope classic`（作業なし）: 本家 `Run \`bun .claude/tools/aidlc-utility.ts intent-create --scope classic\` …` / 本 build は D1 の error

セルフホストでは配布手順の `bun .claude/tools/<tool>.ts` 入口を U4 が Rust へ接続する（契約 C6 / C8）ため、本家逐語を出しても指揮者はそのまま実行できる。

- A. 本家逐語へ全面的に揃える。`next --stage` は本家どおり方向と scope を解決して `execute` を名指す（推奨）
- B. 綴りだけ本家（`bun .claude/tools/<tool>.ts`）へ揃え、`next --stage` が `resolve` を名指す意味は据え置く
- C. マルチコール形を据え置き、承認済みの観測差として記録する（切替条件 2 の判定材料に残す）
- X. Other (please specify)

[Answer]: A. 本家逐語へ全面的に揃える (推奨)

## Q2: 旧コーパスと規則 README の扱い（R5）

[Question]: 旧コーパス `tests/golden/upstream-3c3146cf/`（git 追跡 320 ファイル）と `tests/golden/supplemental-3c3146cf/`（3 ファイル）、および `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/README.md:5,25` の「仕様の正本はゴールデン `tests/golden/upstream-3c3146cf/`（ピン 3c3146cf = v2.6.40）」の記述をどうしますか？

どの検査も旧コーパスを読まない。`memory/project.md` の Mandated「旧 2.6.40 の値を現行互換の根拠として残さない」に照らした扱いを決める。README は規則の正本（人間所有）なので独断で書き換えない。

- A. 旧コーパス 2 本を `git rm` で削除し、README の 2 行を `tests/golden/upstream-a277af21/`（ピン a277af21 = v2.7.1）へ書き換える（推奨）
- B. 旧コーパスは履歴として残し、README の 2 行だけ a277af21 へ書き換える
- C. 両方とも据え置き、別件として記録する
- X. Other (please specify)

[Answer]: A. 削除し README を a277af21 へ (推奨)

## Q3: コーパス内の旧ピン説明文（R6）

[Question]: `tests/golden/upstream-a277af21/cli/set-autonomy/state-field-absent/case.json:6` の description「ピン 3c3146cf の intent-create が起こす状態ファイルには … 行が無いため …」（出所 `scripts/goldens/capture-cli.ts:548`）を直しますか？

内容は 2.7.1 でも真だが説明が旧ピンを名指す。直すには `capture-cli.ts` と case.json の description を修正し、`corpus-manifest.json` を再封印する必要がある。再封印は R1（`reviewer-scope/cases.json` の追加で `verify-corpus.ts` が exit 1、CI が落ちる）のためにどのみち 1 回行う。

- A. description を a277af21 の説明へ修正し、R1 の再封印と同時に封印し直す（推奨）
- B. 据え置き（説明文だけの差として記録する）
- X. Other (please specify)

[Answer]: A. 修正して同時に再封印 (推奨)

## Q4: 未駆動ケースの扱い（R8）

[Question]: 棚卸しで「未駆動」と分類した 2.7.1 コーパスのケースを、U2 の中でどこまでテストで駆動しますか？

上の D1〜D4・D6 の是正は契約で決まるので U2 で行う。加えて probe で前提を再現できた先頭 4 ケース（`next/no-active-intent`、`next/start` 全文、`continue/load-steering` 全文、`next/stage-jump-print`）は Q1 の裁定後にテストへ固定できる。残る未駆動は `next/after-approval`、`jump/execute-forward-to-conditional`、`jump/resolve-forward`、`practices-promote/affirm`、`set-autonomy/state-field-absent`、`report/approved-across-phases` の CLI 面、stop フックの classic 用 stdin 2 件、`write-audit-log` 5 ケースと `record-human-turn` の監査バイト、`continue/multi-part` の native 配送で、いずれも承認・jump・skip を挟む連鎖の再現が要る。

- A. D1〜D4・D6 の是正と先頭 4 ケースのテストを U2 で行い、残る未駆動ケースは一覧のまま切替条件 2 の判定材料として記録する（推奨）
- B. 残る未駆動ケースもすべて U2 の中で TDD で追加する（U2 の完了が後ろへずれる）
- C. D1〜D4・D6 の是正だけ U2 で行い、テストの追加は別 intent へ回す
- X. Other (please specify)

[Answer]: A. 是正＋先頭 4 ケースを U2 で (推奨)

（4 件とも 2026-09-10 の構造化質問で利用者が明示選択。原文はいずれも推奨案の選択肢ラベル）
