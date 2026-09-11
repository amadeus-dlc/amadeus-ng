# per-unit 受領証方針の配線（review-freeze の凍結範囲）

裁定 Q1 = B「`ReviewPolicy::per_unit` を配線する」の実装記録である。
本家 2.7.1（固定コミット `a277af218f0df7f325d3b8be7b6d90fce2c5bd40`）の
`hooks/aidlc-review-freeze.ts` に合わせて、本 build の凍結範囲を揃えた。

裁定の全文は [step7-divergence-questions.md](step7-divergence-questions.md) の Q1 節、
裁定前の採取は [upstream-capture-logs/q1-review-freeze-v2.json](upstream-capture-logs/q1-review-freeze-v2.json)
にある。

## 結論

受入の中心である「ゼロ Unit の per-unit ステージで凍結しない」を満たした。
配線前に本家と食い違っていた条件 B の 2 宛先は、配線後の同一手順・同一入力の走行で
**両側とも exit 0・`REVIEW_FREEZE_BLOCKED` 0 行**になり、差が消えた。

同じ走行で、依頼が「維持する」と指示した Unit 宛先の凍結は**本家より厳しい側に残る**。
これは推測ではなく実測である（後述「配線後も残る差」）。読み替えずに提示する。

## 正本として実測した範囲

行番号はいずれも**固定コミット `a277af21` の実体**のものである（後述の注意を参照）。

- `hooks/aidlc-review-freeze.ts:147-193` — `judgeFreeze` の `for_each === "unit-of-work"` 枝。
  Unit 名の取れない宛先に対して `unitVerdicts` / `unitPending` **だけ**を見る。
  `stageVerdict` は同関数の**非** per-unit 枝（`:212-214`）でしか読まれない。
- `tools/aidlc-lib.ts:7961-7998` — `producesArtifactUnit`。接尾辞 `/<slug>/<filename>` で照合し、
  per-unit ステージでは直前の `/construction/<unit>` から 1 階層の Unit 名を取り出す。
  取れなければ `null`（ステージ水準または曖昧）。
- `tools/aidlc-lib.ts:9748-9753` — `freshReviewReceipts` の `perUnit` 判定。
  `for_each === "unit-of-work"` **かつ** `usesStageLevelPerUnitArtifacts` が偽のときだけ
  受領証地図が Unit 鍵になる。
- `tools/aidlc-lib.ts:21910-21915` — `usesStageLevelPerUnitArtifacts` の定義。
  `effectivePlanAction("units-generation", …) !== "EXECUTE"` の 1 行である。つまり
  `units-generation` を読み飛ばす scope（`bugfix` / `express` / `poc` / `refactor` /
  `security-patch`）では受領証はステージ鍵に落ちる。

この 2 つが噛み合った結果が、裁定の対象になった観測である。**ステージ鍵の受領証は
per-unit ステージの `judgeFreeze` からは一切参照されない**ので、これらの scope では
per-unit ステージの `produces` は凍結されない。

**注意（実測して分かったこと）**: リポジトリ内の `vendor/aidlc-workflows/dist/claude` は
本家そのものではなく**フォーク**である（`aidlc-version.ts` の `AIDLC_VERSION` は
`2.7.1-j5ik2o.1`。固定コミットは `2.7.1`）。`scripts/goldens/upstream-source.ts` の
`verifySource` へ渡すとマニフェストが不一致で止まる（実測: `8874c93d…` ≠ `282b17c5…`）。
16 ファイル以上が異なり `aidlc-lib.ts` も含む。**`aidlc-review-freeze.ts` はバイト一致**
だったが、行番号の引用は固定コミットの実体から取り直した。採取もそちらで走らせている。

## 実装した変更

いずれも既存の `review_closures.rs` / `review_attempt.rs`（受領記録と鮮度判断）を
作り直していない。判断の材料はそのまま使い、**受領証の射程**だけを新しく持たせた。

| ファイル | 変更 |
| --- | --- |
| `modules/core/command/domain/src/workflow_definition/review_policy.rs` | `ReviewPolicy::receipt_covers(&ArtifactTarget) -> bool` を追加。`per_unit` の doc から「未配線」を外した |
| `modules/core/command/domain/src/orchestration/write_targets.rs` | `first_declared_by(node)` を `first_frozen_by(node, policy)` へ置換 |
| `modules/core/command/domain/src/orchestration/intent_execution.rs` | `judge_review_freeze` が `first_frozen_by` を呼ぶ。doc を現行の振る舞いへ差し替え |
| `modules/app/aidlc/tests/review_guards_contract.rs` | ゼロ Unit の凍結を主張していた検査を、本家と同じ観測を主張する検査へ置換 |

### `ReviewPolicy::receipt_covers`

`per_unit` を**射程の判断**として使う。反復軸（`for_each`）を持たないステージの受領証は
宣言成果物のすべてを覆う。per-unit ステージの受領証は Unit ごとなので、Unit 名の取れない
書込み（ゼロ Unit の実行がステージ直下へ置く成果物）は覆わない。宣言外は常に覆わない。

判断を `ReviewPolicy` が持つのは、`per_unit` がこの型の材料だからである
（`coding-rules/tell-dont-ask.md`）。呼出側が `per_unit()` を取り出して自前で分岐すると、
射程という業務判断が値オブジェクトの外へ漏れる。

### `WriteTargets::first_frozen_by`

覆わない宛先で**切り上げない**。本家は宛先を 1 つずつ判定して最初に拒否へ倒れたものを
拒否行に載せるので、per-unit ステージではステージ水準の宛先を読み飛ばし、後ろに並んだ
Unit 宛先で凍結しなければならない。旧 `first_declared_by` は「最初の宣言成果物」で
切り上げるため、`[ステージ水準, Unit]` の順に並んだ 1 回の呼出しを丸ごと素通ししていた。
この取りこぼしは配線と同時に検査で塞いだ。

## Red の再現

`red` はいずれも**コンパイルに成功したうえでの失敗**である。

### 1. ドメイン（判定関数）

```
cargo test -p core-command-domain --lib
```

生ログ: [per-unit-policy-logs/red-01-domain.txt](per-unit-policy-logs/red-01-domain.txt)

```
test result: FAILED. 827 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.63s

---- orchestration::intent_execution::tests::a_stage_level_write_on_a_per_unit_stage_is_not_frozen_by_the_stage_receipt stdout ----
assertion `left == right` failed: /w/x/construction/code-generation/code-generation-plan.md は Unit を名指さない — per-unit ステージの受領証は Unit ごとである
  left: Blocked(ReviewFreezeBlock { target: WriteTarget("/w/x/construction/code-generation/code-generation-plan.md"), stage: StageSlug("code-generation"), unit: None })
 right: Allowed

---- orchestration::intent_execution::tests::a_unit_target_behind_a_stage_level_target_is_still_frozen stdout ----
assertion `left == right` failed
  left: "/w/x/construction/code-generation/traceability.json"
 right: "/w/x/construction/u2-workflow-authority/code-generation/code-generation-plan.md"
```

### 2. プロセス面（フック実行）

```
cargo test -p aidlc --test review_guards_contract
```

生ログ: [per-unit-policy-logs/red-02-integration.txt](per-unit-policy-logs/red-02-integration.txt)

```
test the_freeze_releases_a_zero_unit_stage_level_write_but_holds_the_unit_write ... FAILED
test result: FAILED. 14 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 240.16s

assertion `left == right` failed: construction/code-generation/traceability.json:
  Output { status: ExitStatus(unix_wait_status(512)), stdout: "",
  stderr: "review-freeze: \"…/construction/code-generation/traceability.json\" is this stage's
  output document for stage \"code-generation\", and its latest review is final. …" }
```

`unix_wait_status(512)` は exit 2 である（512 = 2 << 8）。

## Green の一覧

| 検査 | 結果 | 生ログ |
| --- | --- | --- |
| `cargo test -p core-command-domain --lib` | 831 passed / 0 failed | [green-01-domain.txt](per-unit-policy-logs/green-01-domain.txt) |
| `cargo test -p aidlc --test review_guards_contract` | 23 passed / 0 failed | [green-02-integration.txt](per-unit-policy-logs/green-02-integration.txt) |
| `cargo test --workspace` | （下記に追記） | [green-03-workspace.txt](per-unit-policy-logs/green-03-workspace.txt) |
| `cargo lint` | 所見 1 件（他担当のファイル。私の変更に対する所見は 0） | [green-04-cargo-lint.txt](per-unit-policy-logs/green-04-cargo-lint.txt) |
| `cargo clippy --workspace --all-targets -- -D warnings` | 所見 3 件（すべて他担当の検査ファイル） | [green-06-clippy.txt](per-unit-policy-logs/green-06-clippy.txt) |
| `bash scripts/quint-gate.sh` | `[PASS] quint gate: all steps green`（exit 0） | [green-07-quint-gate.txt](per-unit-policy-logs/green-07-quint-gate.txt) |
| `bun scripts/aidlc-sync.ts --check` | exit 1（他担当の `.claude/` 変更。私は `.claude/` を触っていない） | [green-05-aidlc-sync.txt](per-unit-policy-logs/green-05-aidlc-sync.txt) |

追加した検査は 4 件である。

1. `a_stage_level_write_on_a_per_unit_stage_is_not_frozen_by_the_stage_receipt`
   — ゼロ Unit のステージ水準の 2 宛先を通し、同じ走行で反復軸を持たないステージの凍結が
   変わらないことも併せて確かめる。
2. `a_unit_target_behind_a_stage_level_target_is_still_frozen`
   — `[ステージ水準, Unit]` の順に並んだ 1 回の呼出しで、後ろの Unit 宛先を拒否行に載せる。
3. `a_stage_level_receipt_covers_every_declared_target`（`review_policy.rs`）
4. `a_per_unit_receipt_covers_only_a_unit_scoped_target`（`review_policy.rs`）

3・4 は red 先行ではない。1・2 を green にした実装（`receipt_covers`）に対して、
緑を保ったまま refactor 段で足した特性検査である。事実として記す。

（`green-01-domain.txt` の件数が本文中の red 側 827+2 と合わないのは、同じ木で
`u2_reviewer_scope` が検査を足しているためである。私の追加は 4 件である。）

## 配線後に本家との差が消えたことの実行証拠

`u2_upstream_capture` の採取スクリプトを**そのまま**再利用した。手順・入力・正規化は
裁定前の走行と同一である。本家側は固定コミットの実バイト（277 ファイルのマニフェスト
`282b17c5…` を走行の先頭で照合済み）。

```
bun aidlc/.../upstream-capture-logs/capture-q1-review-freeze.ts \
    <固定コミットの dist/claude> \
    target/debug/aidlc \
    aidlc/.../per-unit-policy-logs/q1-review-freeze-after-wiring.json
```

生データ: [per-unit-policy-logs/q1-review-freeze-after-wiring.json](per-unit-policy-logs/q1-review-freeze-after-wiring.json)

条件 B（`bugfix` scope の per-unit ステージ `code-generation`、Unit を 1 つも作らず
ステージ水準の終端受領証を立てた状態）で、記録を本 build が組んだ走行:

| 宛先 | 配線前 本家 / 本 build | 配線後 本家 / 本 build |
| --- | --- | --- |
| `code-generation-plan.md` | exit 0 / **exit 2** | exit 0 / **exit 0** |
| `code-summary.md` | exit 0 / **exit 2** | exit 0 / **exit 0** |
| `memory.md`（日誌・宣言外） | exit 0 / exit 0 | exit 0 / exit 0 |

`REVIEW_FREEZE_BLOCKED` の行数も両側 0 で一致する。陽性対照（条件 A、反復軸を持たない
`requirements-analysis`）は配線後も両側 exit 2・1 行で一致し、陰性対照（条件 D、受領証なし）は
両側 exit 0 のままである。**凍結そのものを壊していないことを同じ走行が示している。**

## 配線後も残る差（実測。読み替えていない）

裁定前の走行では組めなかった条件が、今回は 2 つ成立した。どちらも新しい実測である。

### 1. ステージ水準の受領証しか無いときの Unit 宛先

依頼は「Unit 有りの場合の凍結は維持する」と指示している。その指示どおりに実装したが、
**本家は同じ状態で凍結しない**。射程を測るために採取スクリプトへ探査を 2 つ足した派生版
（[per-unit-policy-logs/capture-per-unit-policy.ts](per-unit-policy-logs/capture-per-unit-policy.ts)、
条件 B に Unit 宛先の探査を追加しただけ）で実測した。

生データ: [per-unit-policy-logs/per-unit-policy-after.json](per-unit-policy-logs/per-unit-policy-after.json)

条件 B、記録を本 build が組んだ走行:

| 宛先 | 本家 | 本 build |
| --- | --- | --- |
| `construction/u1-x/code-generation/code-generation-plan.md` | exit 0、0 行 | **exit 2、1 行** |
| `construction/u2-y/code-generation/code-generation-plan.md` | exit 0、0 行 | **exit 2、1 行** |

原因は明確である。本家は Unit 宛先の凍結に**その Unit の受領証**を要求する
（`judgeFreeze` の `unitVerdicts.has(targetUnit)`）。本 build に Unit 鍵の受領証は無く、
`ReviewAttempt` はステージ 1 つに 1 つなので、ステージの試行を Unit 受領証の代わりに使っている。

向きは「本 build のほうが厳しい」である。穴ではなく保護の過剰だが、**切替条件 2
（状態・監査が upstream 互換）の判定材料**になる。依頼が明示した受入条件どおりの状態なので
実装は止めていない。裁定が要るなら人間へ差し戻す。

### 2. Unit 鍵の受領証が在るときの本家の振る舞い（本 build では駆動不能）

条件 C（`feature` scope で Bolt `u1` / `u2` を起こし、`u1` にだけ終端受領証を立てる）が
今回は本家側で成立した。裁定前の走行では成立しなかった条件である。

| 宛先 | 本家 |
| --- | --- |
| `construction/u1/code-generation/code-generation-plan.md`（自 Unit） | exit 2、1 行 |
| `construction/code-generation/code-generation-plan.md`（ステージ水準） | exit 2、1 行 |
| `construction/u2/code-generation/code-generation-plan.md`（兄弟 Unit） | exit 0、0 行 |

つまり本家は、Unit 鍵の受領証が 1 つでも在るとステージ水準の曖昧な宛先を**閉じる側**へ倒す
（`judgeFreeze` の「Ambiguous per-unit path … fails closed」）。本 build はこの状態へ到達できない
（`aidlc log review --unit` が `REVIEW_UNIT_NOT_WIRED` で拒否し、`aidlc-bolt start` も未配線）ので、
この分岐は**写していない**。兄弟 Unit の分岐が依頼の対象外であることと同じ理由である。

**同じ走行の本 build 側の列（exit 0）は差の証拠ではない。** 条件 C の記録は本家のツールが
組んでおり、本 build のイベントストアには実行が 1 件も無い。本 build のフックは実行カーソルを
読めずに fail-open するので exit 0 になる。**本 build の列が意味を持つのは
`built_by = rust` の走行だけ**である。

## 既存の検査を置換した理由

前提が偽になった検査が 2 か所ある。どちらも削除ではなく、受入条件を弱めない置換にした。

1. `intent_execution.rs` の `the_block_names_the_first_declared_target_and_its_unit`
   — 後半が「ゼロ Unit の実行はステージ直下へ書く — 同じ試行の受領証で凍結する」を主張していた。
   これが裁定で覆った主張そのものである。前半（Unit 宛先が拒否行に Unit 名を載せる）はそのまま残し、
   後半は新設の `a_stage_level_write_on_a_per_unit_stage_is_not_frozen_by_the_stage_receipt` が
   逆向きの主張として引き受けた。**主張の総量は減っていない。**
2. `review_guards_contract.rs` の `the_freeze_applies_to_code_generation_in_a_zero_unit_run`
   — 同じ主張のプロセス面である。`the_freeze_releases_a_zero_unit_stage_level_write_but_holds_the_unit_write`
   へ置換した。置換後は主張が**増えている**: 受領証前に通ること（据置き）、受領証後にステージ水準の
   2 宛先が通ること（新設）、通した書込みが `REVIEW_FREEZE_BLOCKED` を残さないこと（新設）、
   Unit 宛先が拒否されること（据置き）、拒否行が `**Stage**` と `**Unit**` を載せること（`**Unit**` は新設）。

## 確認できなかった範囲

- **Unit 鍵の受領証を本 build で作ること。** `aidlc log review --unit` は
  `REVIEW_UNIT_NOT_WIRED` で拒否し、`aidlc-bolt start` も未配線である。したがって
  「Unit A の受領証が Unit B の書込みを凍結しない」（兄弟 Unit の分岐）と
  「Unit 受領証が在るときステージ水準を閉じる」（fails closed 分岐）は、
  本 build の判定関数では表現していない。依頼の対象外である。
- **`usesStageLevelPerUnitArtifacts` の移植。** 本家はこの述語で受領証の鍵を切り替えるが、
  `judgeFreeze` 自身は述語を見ずに `for_each` だけで分岐する。本 build は `--unit` の記録が
  無いため受領証の鍵が常に 1 つで、述語を持つ必要が現時点で無い。`--unit` を配線するときに
  併せて要る。
- **`stagePending` / `unitPending`（recovery・suspension）の分岐。** 本家 `judgeFreeze` の
  回復待ち分岐は本 build に無い。今回の配線で新たに欠けたのではなく、以前から未移植である。
- **release バイナリでの走行。** 採取は裁定前と同じく `target/debug/aidlc` で行った。
  実地スモークの完了条件が要求する `target/release/aidlc` での確認は本スライスの範囲外である。

## 残課題

1. 上記「配線後も残る差 1」— ステージ水準の受領証で Unit 宛先を凍結する点。依頼の受入条件
   どおりだが本家より厳しい。切替条件 2 の判定前に人間の裁定が要る。
2. `--unit` 受領証の配線（Unit 鍵の受領証地図、兄弟 Unit の分岐、fails closed 分岐）。
   本スライスの対象外。
3. 本スライス**以外**の未解消（いずれも他担当の作業中ファイル。私は触っていない）。
   - `cargo fmt --all --check`: `wording.rs`、`dispatch_rules_contract.rs`、
     `review_guards_contract.rs` の reviewer-scope 検査、`reviewer_scope_*.rs`、
     `dispatch_rules_envelope.rs`、`stage_rule_bundle.rs` ほか。私の 4 ファイルは差分 0 である。
   - `cargo clippy --workspace --all-targets -- -D warnings`: 所見 3 件・exit 101。
     `dispatch_rules_contract.rs:144` と `:214` の `serde_json::to_string`（`clippy.toml` の
     `disallowed-methods`）、`fold_usage_contract.rs:197` の `panic`。いずれも他担当の作業中の
     検査ファイルである。生ログ [green-06-clippy.txt](per-unit-policy-logs/green-06-clippy.txt)。
     （作業の途中では `core-command-domain` にも `redundant closure` 4 件があったが、
     `u2_reviewer_scope` が解消したので最終計測には現れない。）
   - `cargo lint`: 1 件（`reviewer_scope_error.rs:30` の `one-public-type`。1118 ファイル走査）。
     私の変更に対する所見は 0 件である。生ログ [green-04-cargo-lint.txt](per-unit-policy-logs/green-04-cargo-lint.txt)。
   - `bun scripts/aidlc-sync.ts --check`: exit 1（`コピー 0、削除 0、保持設定の確認 0` のまま
     `plan.changed` が真 = 記録済み目録との差）。原因は作業中の `.claude/` 配下の変更であり、
     本スライスは `.claude/` を 1 ファイルも触っていない。生ログ
     [green-05-aidlc-sync.txt](per-unit-policy-logs/green-05-aidlc-sync.txt)。

## 生ログ

すべて [per-unit-policy-logs/](per-unit-policy-logs/) に置いた。

| ファイル | 内容 |
| --- | --- |
| `red-01-domain.txt` | ドメインの red |
| `red-02-integration.txt` | プロセス面の red |
| `green-01-domain.txt` | ドメインの green |
| `green-02-integration.txt` | プロセス面の green |
| `green-03-workspace.txt` | workspace 全体の通し実行 |
| `q1-review-freeze-after-wiring.json` | 採取スクリプトそのままの再走行（配線後） |
| `capture-after-wiring-stdout.txt` | 同、標準出力 |
| `capture-per-unit-policy.ts` | 探査を 2 つ足した派生スクリプト |
| `per-unit-policy-after.json` | 派生スクリプトの走行 |
| `capture-extended-stdout.txt` | 同、標準出力 |
| `green-04-cargo-lint.txt` | `cargo lint` |
| `green-05-aidlc-sync.txt` | 配布同期の確認 |
| `green-06-clippy.txt` | workspace 全 target の Clippy |
| `green-07-quint-gate.txt` | Quint ゲート |
