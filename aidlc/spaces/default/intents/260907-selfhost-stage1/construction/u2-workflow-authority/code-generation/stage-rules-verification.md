# 規則受渡し（`aidlc hook deliver-stage-rules`）の着地確認記録

担当 `u2_stage_rules_closeout`（2026-09-10）。対象は承認済み実装計画 Step 7 の「規則受渡し
（deliver-stage-rules）」— 指揮者（conductor）が部下（subagent）へ渡す brief（依頼文）の末尾へ、その
ステージの規則束（rule bundle）を逐語で付ける PreToolUse フックである。前担当 `u2_stage_rules` が
実装を進めて記録を書く前に停止したので、本記録は (1) 現在の実装を本家 2.7.1（固定コミット
`a277af218f0df7f325d3b8be7b6d90fce2c5bd40`）の**実走行採取**と突き合わせ、(2) Red / Green の証跡を
揃え、(3) 写していない分岐を列挙する。突き合わせで見つかった差のうち、明白な写し漏れ 1 件
（先頭 BOM の扱い）だけを TDD（失敗するテストを先に走らせてから最小実装する進め方）で直した。

用語: 「brief」はエージェントへ渡す依頼文、「束（bundle）」はその末尾へ足す規則ブロック、
「ダイジェスト」は束の内容から計算する sha256、「BOM」は UTF-8 ファイル先頭に置かれることのある
バイト順マーク（EF BB BF、文字としては U+FEFF）である。

## 1. 実装の要約 — 何を判断し、何を判断しないか

本 build の入口は `modules/app/aidlc/src/runtime.rs` の `run_hook`（`"deliver-stage-rules"` →
`dispatch_rules::run`。行 1048 / 1084。本担当は編集していない）で、判断は次の 3 つだけである。

| 判断 | 置き場 | 本家の対応行 |
| --- | --- | --- |
| **どの stage の束か** — brief 中の明示パス（`stages/<phase>/<slug>.md`）→ 状態ファイルの `Current Stage` → brief が語として名乗る唯一の slug、の順 | `dispatch_rules.rs` `resolve_stage` / `explicit_stage_path` / `mentions_slug`（122–227 行） | `hooks/aidlc-deliver-stage-rules.ts:85–106` |
| **どのファイルを読むか** — その stage の `rules_in_context` を `/memory/` 目印で active space の memory 層へ解決し、同じ綴りは 1 度だけ、中身のある規則だけを読み順に並べる。読めなければ部分的には書かず拒否 | `dispatch_rules.rs` `read_rules` / `read_utf8`（253–295 行） | `tools/aidlc-steering.ts:57–108`（`rulesContentEntries` / `readRuleBundle`） |
| **出しても安全か** — stdout 全体（末尾改行込み）が 512KiB を超えるなら拒否（exit 2）、`AIDLC_DISPATCH_RULES_PRELOAD_FALLBACK=1` なら助言（exit 3） | `dispatch_rules.rs` `run`（42–75 行）、`wording.rs` の逐語 2 本 | `hooks/aidlc-deliver-stage-rules.ts:329–354` |

判断を持たない部品（描画・封筒）は `harness-claude` に置いた。

| ファイル | 役割 | 本家の対応 |
| --- | --- | --- |
| `modules/harness/claude/src/rule_file.rs` | `RuleFile` — 配送する綴りと本文の 1 件 | `RuleContent = { path, text }` |
| `modules/harness/claude/src/stage_rule_bundle.rs` | `StageRuleBundle` — 束の描画（見出し・枠文・BEGIN/END マーカー）とダイジェスト、同一ブロックの検出 | `bundleBlock` / `hasExactBundle`（108–134 行） |
| `modules/harness/claude/src/dispatch_rules_envelope.rs` | `DispatchRulesEnvelope` — PreToolUse 封筒の読み（4 工具・除外エージェント・本文 4 フィールドと `items`・`subagent` の `stages[]`）と書き戻し | 47–48、157–258 行 |
| `modules/app/aidlc/src/wording.rs` | `dispatch_rule_unreadable` / `dispatch_rules_oversize` / `dispatch_rules_oversize_advisory` の逐語 3 本 | `aidlc-steering.ts:101–102`、hook 338–352 行 |
| `modules/app/aidlc/tests/dispatch_rules_contract.rs` | 公開プロセス面の契約テスト 15 件（本 build のバイナリを子プロセスで起動） | — |
| `modules/harness/claude/src/lib.rs`、`modules/harness/claude/Cargo.toml` | 3 型の公開と、封筒テストが使う `tempfile` の dev-dependency（前担当の変更） | — |

判断しないもの: 規則が「中身を持つか」の述語は RMU（読取りモデル更新器）の
`SteeringSource::text_is_substantive`（`modules/core/read-model-updater/src/orchestration/steering_source.rs:153`）を
呼ぶ。本家が `isSubstantiveRuleText` を `tools/aidlc-steering.ts` から 1 本だけ輸出し、`next` /
`continue` の load-steering と dispatch の両方に使わせているのと同じ配置である。フックは
ワークスペースへ何も書かない（テスト `a_dispatch_never_writes_anything_into_the_workspace`）。

## 2. Red の再現 — 前担当のログの検証

前担当が残した 3 本と、本担当が足した 1 本。いずれも**コンパイルに成功したうえで**、意図した
振る舞いの不一致で失敗している（「テストが見つからない」「依存不足」「構文不正」ではない）。

| ログ | コマンド | 結果 | red-first（テストが実装より先）の順序は守られていたか |
| --- | --- | --- | --- |
| [red-1-stage-rule-bundle.log](stage-rules-logs/red-1-stage-rule-bundle.log) | `cargo test -p harness-claude --lib stage_rule_bundle` | 0 passed / 6 failed | **整合する**。`the_block_is_byte_identical_to_the_upstream_capture` が `left: ""` で失敗し、`an_exact_block_anywhere…` が `attempt to subtract with overflow`（空文字の `len() - 1`）で落ちている — `block()` が空文字を返す仮実装（stub）に対してテストを先に走らせた形である。ただしテスト本文と仮実装のどちらを先に書いたかはログからは分からない |
| [red-2-dispatch-rules-envelope.log](stage-rules-logs/red-2-dispatch-rules-envelope.log) | `cargo test -p harness-claude --lib dispatch_rules_envelope` | 4 passed / 12 failed | **整合する**。通っている 4 件は「brief を持たない」否定形のケースで、brief 0 件を返す仮実装でも通る。残り 12 件が `left: []` で失敗している |
| [red-3-dispatch-rules-contract.log](stage-rules-logs/red-3-dispatch-rules-contract.log) | `cargo test -p aidlc --test dispatch_rules_contract` | 9 passed / 5 failed | **5 件は本物の Red**（明示パス・区切り記号の形・唯一の slug・重複抑止・`subagent` の各 stage）。**9 件は Red 採取時点で既に通っており、この 9 件については red-first の順序を確認できない**。また `a_subagent_dispatch_augments_each_stage_entry…` の失敗理由は `phases/inception.md` の不在（fixture 側の欠落。現在の `Workspace::new` は他フェーズの規則も置く）であり、実装の欠落だけによる Red ではない |
| [red-4-bom-strip.log](stage-rules-logs/red-4-bom-strip.log)（本担当） | `cargo test -p aidlc --test dispatch_rules_contract a_leading_utf8_bom` | 0 passed / 1 failed | **守った**。§4.4 |

前担当の Red のログには**タイムスタンプが無い**（ファイルの更新時刻は 12:37 / 12:51 / 13:13）。
Green のログは前担当の停止時点で無く、本担当が最終状態で採った（§3）。

## 3. Green の一覧（最終状態 — BOM 修正と `serde_json` 置換の後）

| ログ | コマンド | 結果 |
| --- | --- | --- |
| [green-1-stage-rule-bundle.log](stage-rules-logs/green-1-stage-rule-bundle.log) | `cargo test -p harness-claude --lib stage_rule_bundle` | 6 passed / 0 failed |
| [green-2-dispatch-rules-envelope.log](stage-rules-logs/green-2-dispatch-rules-envelope.log) | `cargo test -p harness-claude --lib dispatch_rules_envelope` | 16 passed / 0 failed |
| [green-3-dispatch-rules-contract.log](stage-rules-logs/green-3-dispatch-rules-contract.log) | `cargo test -p aidlc --test dispatch_rules_contract` | 15 passed / 0 failed |
| [green-4-bom-strip.log](stage-rules-logs/green-4-bom-strip.log) | `cargo test -p aidlc --test dispatch_rules_contract a_leading_utf8_bom` | 1 passed / 0 failed |

契約テスト 15 件の内訳: 本家採取バイトとの一致 1、空テンプレートの除外 1、stage 解決 5（明示
パス > 現在 stage > 唯一の slug、未知パスの後退、区切り記号 6 形、境界文字）、重複抑止 1、無関係入力
10 形 1、`subagent` の `stages[]` 1、規則が読めない 2（不在・非 UTF-8）、先頭 BOM 1、512KiB 境界
（ちょうど・1 超・助言）1、無書込 1。

実行時間は同時刻の他担当のビルドに左右される（同じ 15 件が 1.34 秒〜94.9 秒）。解析の遅さではない。

## 4. 本家実走行との突き合わせ

### 4.1 方法

- 本家は本リポジトリの `.claude/hooks/` ではなく、**固定コミットの実バイト**を
  `git -C vendor/aidlc-workflows archive a277af21… dist/claude` で一時領域へ展開して走らせた
  （[closeout-workspaces.sh](stage-rules-logs/closeout-workspaces.sh)。展開物の sha256 は
  `CLOSEOUT_ROOT/pinned.sha256` に記録: hook `1da072d0…`、`aidlc-lib.ts` `3b0ff3dd…`、
  `stage-graph.json` `547f10c1…` — 後者は `tests/golden/upstream-a277af21/data/` および本リポジトリの
  `.claude/tools/data/` とバイト一致）。なお本リポジトリの `.claude/tools/aidlc-lib.ts` は固定コミットと
  **異なる**（`62c7fa1e…`）ので、前担当が本リポジトリの hook を走らせた採取とは lib の版が違う。
- project dir は一時ワークスペース 14 個（`ws` = 本リポジトリ相当。memory 層は本リポジトリの写しで
  バイト同一、`Current Stage: code-generation`。`stateless` = 固定コミットの `dist/claude` そのもの。
  ほか `norules` / `norules-with-harness` / `fixture` / `bom` / `bom2` / `ctrl` / `tabstage` /
  `nospace` / `nostage` / `dangling` / `boundary` / `duplicate`）。本リポジトリには何も書いていない。
- 両側へ**同じ stdin・同じ環境（PATH / HOME / TMPDIR と case の env だけ。この会話の `CLAUDE_*` は
  持ち込まない）・同じ project dir** を与え、exit / stdout / stderr を**バイト**で比べた
  （[closeout-differential.ts](stage-rules-logs/closeout-differential.ts)）。前担当の
  `differential-driver.ts` は exit と stdout だけを照合し stderr は表示のみだったので、ここが違う。
- corpus は前担当の 2 本（[differential-corpus.json](stage-rules-logs/differential-corpus.json) 38 件、
  [differential-corpus-2.json](stage-rules-logs/differential-corpus-2.json) 25 件）をそのまま再利用し
  （本リポジトリを指す `cwd` / `AIDLC_PROJECT_DIR` は一時ワークスペースへ写像）、本担当が
  [closeout-corpus-3.json](stage-rules-logs/closeout-corpus-3.json) 26 件を足した。
- 本 build は `cargo build -p aidlc` の `target/debug/aidlc`（BOM 修正後に再ビルド）。

### 4.2 結果

| 突き合わせ | 件数 | 完全一致（exit・stdout・stderr） | stderr の丸括弧内だけ相違 | 相違 | 記録 |
| --- | --- | --- | --- | --- | --- |
| corpus（前担当 63 + 本担当 26） | 89 | **80** | 2 | 7 | [closeout-differential.tsv](stage-rules-logs/closeout-differential.tsv)、[closeout-differential-detail.json](stage-rules-logs/closeout-differential-detail.json) |
| 重複抑止（`hasExactBundle`）7 形 | 7 | **7** | 0 | 0 | [closeout-duplicate.tsv](stage-rules-logs/closeout-duplicate.tsv) |
| 512KiB 境界・拒否 | 8 | **5** | 3 | 0 | [closeout-boundary.tsv](stage-rules-logs/closeout-boundary.tsv) |

BOM 修正前は corpus 88 件（case 90 追加前）で 78 一致 / 2 stderr のみ / **8** 相違だった
（[closeout-differential-before-bom-fix.tsv](stage-rules-logs/closeout-differential-before-bom-fix.tsv)）。
差の 1 件（67）が修正で消え、追加した 90（BOM 2 つ）も一致した。

コーパス別: 前担当 corpus-1（01–38）38/38 一致、corpus-2（40–64）23 一致・2 相違（52・64 —
前担当の結果と同じ）、本担当 corpus-3（65–90）19 一致・2 stderr のみ・5 相違。

一致した中で、本家の逐語・分岐の写しを実走行で確かめられた観測（抜粋）: 明示パスの正規表現の
全形（区切り `/` `\` 混在、大文字フェーズ、`.mdx` / `.draft.md` / `.md_v2` の拒否、先頭・`/` 直後、
最初の一致が勝つ）、`Current Stage` が prose の slug 言及に勝つこと、記録なしでの唯一 slug の束縛と
曖昧時の無束縛、`_` は境界で `-` と英数字は境界でないこと、大小無視、非 ASCII を含む brief、4 工具名の
大小無視、除外エージェント、ペルソナ不在、名前の形、4 つのエージェント名フィールドの優先と非文字列
での打切り、本文 4 フィールドの名前順優先（キー順ではない）、空文字の打切り（`message` へ落ちない）、
`items` の改行結合（非テキストだけでも `"\n"` が brief になる）、`items` への新規要素追加、キー順の
保持、重複キーの後勝ち、数値（`1e21` / `-0` / `1.0` / 20 桁整数 / `0.1` / `1e-7` / `-1.5e300`）と
非 BMP 文字・U+2028・制御文字の素通し、規則本文中の制御文字（0x01 / 0x1f / 0x7f / NUL / NBSP /
U+2028 / U+2029 / CR）の JSON エスケープ、`subagent` の `stages[]`（対象外要素の保持・2 束・2 件目の
規則不在で全体停止）、ちょうど 524288 バイトは通り 524289 で拒否・助言（exit 3）、同一ブロックの
末尾・途中・2 個並びの抑止と 1 バイト違い・別 stage マーカーの非抑止。

### 4.3 相違の内訳（読み替えていない — 裁定は親と人間）

| # | case | 入力 | 本家（固定コミット） | 本 build | 原因の所在 | 分類 |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | 52 `norules` | `.claude` も `aidlc` も無いディレクトリを project dir に、brief「work on intent-capture now」 | exit 2、stderr `Cannot load required stage rule "…/org.md" (ENOENT…)`。グラフとペルソナは**フックのスクリプト相対**（`tools/aidlc-runtime-paths.ts` `resolveHarnessRoot` — `moduleHarnessRoot` が project 相対より先）で読めるため stage が束縛され、規則の不在で止まる | exit 0、無出力。グラフは `<project>/.claude/tools/data/`（`layout.rs:270`）から読み、無ければ判断材料なしとして黙る（`dispatch_rules.rs:48`） | ハーネス根の解決元の違い。配布形（`.claude/` が project dir 直下に在り、フックがそこから起動する）では両者は同じ場所を指す。双子 case 66（`.claude` あり・`aidlc` なし）は両側 exit 2 で一致 | **所有外・環境**。`Layout` は共通部品 |
| 2 | 64 | stdin `null` | exit 1、stderr に `TypeError: null is not an object (evaluating 'parsed.tool_name')` のスタック（`JSON.parse("null")` は成功し、続く `parsed.tool_name` で未処理例外） | exit 0、無出力（`a_dispatch_that_is_not_a_delegated_aidlc_agent_passes_untouched` が exit 0 を固定） | 本家の未処理例外 | **本家の不具合**。写すべきかは裁定 |
| 3 | 72 | `{"tool_name":7,…}` | exit 1、`TypeError: toolName.toLowerCase is not a function` のスタック | exit 0、無出力 | 同上（`augmentDispatchRules:223`） | 同上 |
| 4 | 76 `tabstage` | 状態ファイルが `- **Current Stage**:<TAB>code-generation` | `Current Stage` を読めて `code-generation` の束（28024 B） | 読めず（`None`）唯一 slug へ後退し `intent-capture` の束（27861 B） | 本家 `getField` は `^- \*\*Field\*\*:[ \t]*(.*)$` で区切りに空白かタブを 0 個以上許す。本 build の `session_hooks::field`（`runtime/session_hooks.rs:97`）は `- **Field**: `（空白 1 個）の前方一致 | **所有外**（`session_hooks.rs`。同関数は fold-usage / review-guards / session-start など 5 ファイル 9 箇所から使われ、直せば全フックの読みが揃う） |
| 5 | 77 `nospace` | `- **Current Stage**:code-generation`（空白なし） | `code-generation` | `intent-capture` へ後退 | 同上 | 同上 |
| 6 | 79 `dangling` | `active-intent` が存在しない記録 `gone` を指し、記録 `rec-0001` だけが実在 | カーソルが実在記録を指さないので無視し、**唯一の記録**へ後退（`aidlc-lib.ts` `activeIntent` の lone-intent）→ `code-generation` | カーソルの綴りをそのまま record dir にし（`layout.rs:157–164` `Layout::shared`）、状態ファイル不在 → 唯一 slug → `intent-capture` | `Layout` の active-intent 解決 | **所有外**（`layout.rs`） |
| 7 | 80 | 環境 `AIDLC_RULES_DIR=<fixture の memory>` | fixture の規則束（876 B、`3072de…`） | project の memory 束（28042 B、`b503b8…`）— 環境変数を読まない | 本家 `rulesContentEntries` の Codex / fixture 向けシーム（`aidlc-steering.ts:62–67`） | **写していない**（§5） |
| 8 | 66・82（stderr のみ） | 規則ファイル不在 | `(ENOENT: no such file or directory, open '<絶対パス>')` | `(No such file or directory (os error 2))` | 丸括弧の中は Node の `errorMessage(error)` と Rust の `io::Error` の綴り。前後は逐語（`wording.rs` の doc に記載済み） | **既知の綴り差**。揃えるかは裁定 |
| 9 | boundary 3 件（stderr のみ） | 不在・非 UTF-8・サロゲートの UTF-8 符号化 | `(ENOENT…)` / `(Invalid byte sequence)` | `(No such file or directory (os error 2))` / `(invalid utf-8 sequence of 1 bytes from index 0)` | 同上 | 同上 |

いずれも exit と stdout（束のバイト）については、#8・#9 は一致、#1〜#7 は上記のとおり相違である。
本 build 側の実装をこれらに寄せてはいない。

### 4.4 修正した差分 — 先頭 BOM（写し漏れ、TDD で最小修正）

- **実測**: case 67（`bom` — fixture の `org.md` の先頭に EF BB BF）で、本家は BOM 無しと同じ束
  （876 B、ダイジェスト `3072de…`）を出し、本 build は U+FEFF を本文に含めた束（879 B、`892de2…`）を
  出した。本家は `new TextDecoder("utf-8", { fatal: true }).decode(bytes)` で読み
  （`aidlc-steering.ts:96`）、WHATWG Encoding の既定（`ignoreBOM: false`）は先頭の BOM を出力へ
  含めない。本 build は `String::from_utf8` のままだった。
- **Red**: `a_leading_utf8_bom_is_dropped_the_way_the_upstream_decoder_drops_it` を
  `dispatch_rules_contract.rs` に追加して実行。`892de2…` 対 `3072de…` の不一致で失敗
  （[red-4-bom-strip.log](stage-rules-logs/red-4-bom-strip.log)）。
- **Green**: `dispatch_rules.rs` `read_utf8` で先頭の U+FEFF を 1 つだけ落とす
  （[green-4-bom-strip.log](stage-rules-logs/green-4-bom-strip.log)）。
- **本家での裏取り**: case 90（`bom2` — BOM を 2 つ並べる）で本家も 2 つ目を本文として残す
  （879 B、`892de2…`）ことを実走行で確かめ、修正後の本 build と一致した。同じ性質をテストの後半で
  固定している。
- **Refactor**: 無し（doc コメントを足しただけ）。

同じ読み方をする兄弟がある。RMU の `SteeringSource`（`next` / `continue` の load-steering が読む
規則）は `fs::read_to_string`（`steering_source.rs:77,126`）で BOM を残す。本家は同じ
`readRuleBundle` を使うので同じ差が出ると推定されるが、**本記録では駆動していない**（所有外）。

### 4.5 テストの修正（同語反復の除去）

scoped clippy（§6）が `dispatch_rules_contract.rs` の `serde_json::to_string` 3 件（前担当 2・
本担当 1）を `clippy.toml` の disallowed method として検出した。期待値を実装と同じ直列化で組み立てる
形にはせず、**本家採取の stdout 行そのもの**（`upstream-fixture-block.txt` の逐語。末尾改行込みの
sha256 `13db371a…5fa5` は本 build の case 65 stdout と完全一致）を `UPSTREAM_STDOUT_LINE` として置き、
`a_dispatch_to_an_aidlc_agent_receives_the_bundle_upstream_produced` と BOM テストの期待値にした。
入力の組立て（`dispatch()`）だけ `canon_json::serialize` に替えた。15 件は置換後も成功している。

## 5. 写した / 写していない / 対象外 / 駆動できなかった分岐

| 分岐（本家の行） | 状態 | 理由・備考 |
| --- | --- | --- |
| `DISPATCH_TOOLS` 4 工具（47・223）、`EXEMPT_AGENTS`（48）、`isAidlcAgent` の形・ペルソナ実在・除外（55–62） | 写した | corpus 08–15・19、封筒テスト |
| `currentStage` — 状態ファイルの `Current Stage`（64–72） | 写した（読みの緩さに差） | §4.3 #4・#5 |
| `promptStage` の 3 段（85–106）、正規表現 2 本 | 写した | corpus 01–07・38・40–49・53–60・87–89 |
| `bundleBlock` / `hasExactBundle`（108–134） | 写した | fixture 一致、重複抑止 7/7 |
| `augmentText`（136–155）— 読めない → 拒否、空束・同一束 → 無変化 | 写した | corpus 24・28・65・82、duplicate |
| `promptText` / `withPrompt` の 4 フィールドと `items`（157–194） | 写した | corpus 20–27・71・81・83・84 |
| `augmentSingleDispatch` のエージェント名フィールド順（196–216） | 写した | corpus 16–19・85 |
| `subagent` の `stages[]`（229–258） | 写した | corpus 30–35・50・75・82 |
| `run` — 不正 JSON → 0、拒否 → 2、無変化 → 0、上限超 → 2 / 助言 3、出力（305–358） | 写した | corpus 61–63、boundary 8 件 |
| `rulesContentEntries` / `readRuleBundle` — `/memory/` 目印、相対綴り、重複除去、中身判定、UTF-8 厳密、先頭 BOM（`aidlc-steering.ts:57–108`） | 写した（BOM は本記録で修正） | corpus 65・67・68・90、boundary 3 件 |
| `recordAcceptedBackgroundDispatch` — `run_in_background: true` のとき `session_id` を検査し `markSubagentInflight` を書く（271–303・326・344・355） | **写していない** | 棚卸し行「`run_in_background` の inflight 更新は通常スモークに追加しない」。本 build は何も書かない（テストで固定）。corpus は `run_in_background: true` を含めていない（本家が書き込むため）。完了扱いにしない |
| `AIDLC_RULES_DIR`（`aidlc-steering.ts:62–67`） | **写していない** | Codex / fixture 向けの環境シーム。§4.3 #7 |
| project dir の決め方 — `AIDLC_PROJECT_DIR` → `CLAUDE_PROJECT_DIR` → `parsed.cwd` → `process.cwd()`（312–319） | **写していない**（build 全体の規約） | 本 build は作業ディレクトリか `--project-dir` で決める（`runtime.rs:167–171`）。Claude Code は `CLAUDE_PROJECT_DIR` を渡し、`settings.json` は `bun "$CLAUDE_PROJECT_DIR/.claude/hooks/…"` で登録している。native 登録時に作業ディレクトリが project dir と一致するか、`--project-dir "$CLAUDE_PROJECT_DIR"` を渡すかは U4 の登録の論点 |
| ハーネス根・グラフ・ペルソナの解決元 — スクリプト相対、`AIDLC_HARNESS_DIR` / `AIDLC_AGENTS_DIR` / `AIDLC_STAGE_GRAPH` / `AIDLC_RUNTIME_*`（`aidlc-runtime-paths.ts`、`aidlc-lib.ts` `agentsDir`） | **写していない** | 本 build は `<project>/.claude/` 固定。§4.3 #1 |
| セッション束縛の選択 — `AIDLC_SESSION_OVERRIDE` / 祖先プロセス（`resolveWorkflowSelection`）、`activeIntent` の lone-intent 後退 | 差あり・**所有外** | `Layout::resolve_for_session` は独自の束縛（`aidlc/.aidlc-sessions/<id>.binding.json`）を持つ。lone-intent は §4.3 #6 |
| 本家の未処理例外（`null` / 非文字列 `tool_name` / グラフが読めない → `loadStageGraphAll` が throw） | 差あり | 本 build は exit 0 で黙る。グラフ不在は駆動していない（配布形では常に在る） |
| 無効ステージ（`enabled: false`）の除外 | 同等（駆動不能） | 本家 `loadStageGraph` も `enabled !== false` で絞る。固定コミットのグラフに `enabled` キーが無いので corpus では区別できない |
| Kiro CLI / Kiro IDE / OpenCode / Codex / Copilot での消費（ヘッダ 5–11 行） | **対象外** | 他ハーネス |
| `settings.json` の登録を TS から `aidlc hook deliver-stage-rules` へ向ける | 対象外（U4） | 本記録では触っていない |

## 6. 検査の結果

| 検査 | コマンド | 結果 | 記録 |
| --- | --- | --- | --- |
| 整形 | `rustfmt --edition 2024 --config-path rustfmt.toml --check` 所有 6 ファイル | 差分なし（exit 0） | — |
| 静的検査（対象 crate 全体） | `cargo clippy -p aidlc -p harness-claude --all-targets --no-deps -- -D warnings` | **exit 101**。所見はすべて他担当の作業中ファイル `modules/app/aidlc/src/usage_ledger.rs` と `usage_ledger/*.rs`（fold-usage）。**本担当の 6 ファイルに所見なし**（lib の所見は全件列挙されるので、不在は意味を持つ） | [static-checks.log](stage-rules-logs/static-checks.log) |
| 静的検査（harness-claude 単独） | `cargo clippy -p harness-claude --all-targets --no-deps -- -D warnings` | 所見なし（exit 0） | [static-checks-scoped.log](stage-rules-logs/static-checks-scoped.log) |
| 静的検査（aidlc、テスト target まで到達させた scoped 実行） | 上に `-A dead_code -A unused_imports -A clippy::missing_const_for_fn -A clippy::manual_let_else -A clippy::collapsible_match` を付けたもの | 初回は `dispatch_rules_contract.rs` に `disallowed_methods` 3 件 → §4.5 で置換 → **exit 0**。5 つの allow は他担当の `usage_ledger/*` の所見を外すためだけで、本担当のファイルではこの 5 種の所見が元々出ていない | 同上 |
| 独自検査 | `cargo lint` | 所見 0（exit 0） | 同上 |
| 対象テスト | §3 | 6 + 16 + 15 = 37 件成功 | green-1〜4 |

閾値・期待バイト・正規化設定・カバレッジ床・clippy / lint の設定は変えていない。
`cargo test --workspace`、`cargo clippy --workspace`、カバレッジ（行 90.0% 床・相対ゲート）、Quint /
ITF は**本記録では実行していない**（承認済みテスト手順が B1 統合前の共通検査と位置づけ、親が最後に
1 回行う）。

## 7. 残課題（裁定または後続が要る）

1. **§4.3 #1（ハーネス根の解決元）**: 配布形では一致するが、`Layout` が `<project>/.claude/` 固定で
   あることと、native 登録時の作業ディレクトリ / `--project-dir` の扱い（§5 の project dir 行）は U4 と
   合わせて裁定が要る。
2. **§4.3 #2・#3（本家の未処理例外）**: `null` / 非文字列 `tool_name` に対し本 build は exit 0 で黙り、
   前担当のテストがそれを固定している。本家に合わせて exit 1 にするか、不具合として黙るかは裁定。
3. **§4.3 #4・#5（`session_hooks::field` の読みの緩さ）**: 本家 `getField` の `[ \t]*` を写すなら
   `runtime/session_hooks.rs:97` の 1 箇所で、5 ファイル 9 箇所の読みが同時に変わる。本担当の所有外
   なので手を付けていない。
4. **§4.3 #6（`Layout::shared` の lone-intent 後退）**: カーソルが実在記録を指さないときの後退は
   `layout.rs` の共通部品の論点。所有外。
5. **§4.3 #7（`AIDLC_RULES_DIR`）**: Codex / fixture 向けシームを写すか。
6. **stderr の丸括弧内の綴り差**（§4.3 #8・#9）: 揃えるなら Node の `errorMessage` の綴り
   （`ENOENT: no such file or directory, open '<abs>'` / `Invalid byte sequence`）を境界で変換する。
7. **`run_in_background` の inflight 更新**（`markSubagentInflight`）: 棚卸しどおり未実装。本家は
   状態ファイルがあるときに書く。裁定が要る。
8. **`SteeringSource` の先頭 BOM**（§4.4 末尾）: `next` / `continue` 側の兄弟。未駆動・所有外。
9. 差分駆動は一度きりで CI に載っていない。corpus・スクリプト・出力は
   [stage-rules-logs/](stage-rules-logs/) に残した（再現: `CLOSEOUT_ROOT=<dir> bash closeout-workspaces.sh` →
   `CLOSEOUT_ROOT=<dir> AIDLC_BIN=target/debug/aidlc bun closeout-differential.ts <out.tsv> <detail.json> differential-corpus.json differential-corpus-2.json closeout-corpus-3.json`）。
10. red-3 の 9 件については red-first の順序を確認できない（§2）。

## Sources

- 本家固定コミット `a277af218f0df7f325d3b8be7b6d90fce2c5bd40` の実バイト（`git -C vendor/aidlc-workflows show a277af21…:dist/claude/.claude/<path>`）:
  `hooks/aidlc-deliver-stage-rules.ts:47–62,64–106,108–134,136–194,196–258,260–303,305–358`、
  `tools/aidlc-steering.ts:20–116`、`tools/aidlc-lib.ts`（`getField` / `activeIntent` / `activeSpace` /
  `stateFilePath` / `resolveWorkflowSelection` / `loadStageGraph` / `agentsDir` / `deriveHarnessDir`）、
  `tools/aidlc-graph.ts`（`loadGraph` / `stageGraphPath` / `memoryDirFor`）、
  `tools/aidlc-runtime-paths.ts`（`resolveHarnessRoot` / `moduleHarnessRoot` / `explicitHarnessRoot`）。
- vendor の作業ツリー HEAD `801c5700` は固定コミットと 16 ファイル異なる（本フックと `aidlc-steering.ts`
  は同一、`aidlc-lib.ts` は本リポジトリの `.claude/tools/` 側が異なる）。
- 現コード: `modules/app/aidlc/src/runtime/dispatch_rules.rs`、
  `modules/harness/claude/src/{stage_rule_bundle,dispatch_rules_envelope,rule_file}.rs`、
  `modules/app/aidlc/src/wording.rs:462–498`、`modules/app/aidlc/src/runtime.rs:1033–1085,150–171`、
  `modules/app/aidlc/src/layout.rs:125–290`、`modules/app/aidlc/src/runtime/session_hooks.rs:97–103`、
  `modules/core/read-model-updater/src/orchestration/steering_source.rs:77,126,146–193`、
  `modules/app/aidlc/tests/dispatch_rules_contract.rs`。
- 前担当の採取と本担当の採取: [stage-rules-logs/](stage-rules-logs/)（一覧は §2〜§4、§6）。
- 前担当の棚卸し [remaining-step7-inventory.md](../remaining-step7-inventory.md) の「規則受渡し」行、
  手本 [shell-write-targets-verification.md](shell-write-targets-verification.md) /
  [continuation-rejection-verification.md](continuation-rejection-verification.md)、
  承認済み [実装計画](code-generation-plan.md) / [テスト手順](unit-test-instructions.md)（読むだけ）。
- 設計規則 [coding-rules/](../../../../../knowledge/aidlc-shared/coding-rules/) の
  `upstream-contracts.md`（境界で変換）/ `error-handling.md`（文言は出す側の `wording`）/
  `abstract-data-type.md` / `first-class-collections.md` / `no-backward-compatibility.md`。

## Assumptions & Open Questions

- §7 の 1〜8 は裁定が要る。本 build の実装をこれらへ寄せていない。
- 「配布形では #1 の差は現れない」は、フックが `<project>/.claude/hooks/` から起動され作業ディレクトリが
  project dir である、という Claude Code の実運用を前提にした推定であり、実地スモークで確かめていない。
- `SteeringSource` の BOM（§7 の 8）は本家との実走行比較を行っていない推定である。
