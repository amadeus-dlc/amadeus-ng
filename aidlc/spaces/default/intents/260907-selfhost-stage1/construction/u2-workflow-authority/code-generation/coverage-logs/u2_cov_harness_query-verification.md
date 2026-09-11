# カバレッジ向上の記録 — 担当 `u2_cov_harness_query`

担当 crate: `harness-infrastructure`、`harness-claude`、`core-query-use-case`、
`core-query-interface-adapter`、`core-infrastructure`。対象は
[targets-harness-query.md](targets-harness-query.md) の 40 ファイル・未カバー 517 行
（workspace 計測 = step 9 の per-file JSON）。共通 brief は [brief-common.md](brief-common.md)。
生ログは [u2_cov_harness_query/](u2_cov_harness_query/) に置いた。

用語: 「未カバー行」は llvm-cov（LLVM の行カバレッジ計測）でテスト実行時に 1 度も通らなかった
行をいう。「契約テスト」は実装の計算を写すのではなく、入力に対する観測可能な振る舞いを
主張するテストをいう。

## 結果の要約

| 指標 | 着手前 | 着地後 |
| --- | ---: | ---: |
| 対象 40 ファイルの未カバー行（workspace 計測の集合を基準） | 517 | **71**（86.3% 減） |
| 対象 40 ファイルの未カバー行（所有 crate だけの計測） | 1,679 | 81 |

完了条件 1（85% 以上減らす）を満たす。残り 71 行の内訳は「残った未カバー行」に示す。
プロダクトコードは変更していない（テスト追加のみ）。

### crate ごとの行カバレッジ（所有 crate のテストだけを走らせた計測）

計測コマンドは `cargo llvm-cov --no-report -p <5 crate> --tests` →
`cargo llvm-cov report --json --ignore-filename-regex '(^|/)modules/app/aidlc/src/main\.rs$'`
（`PROPTEST_RNG_SEED=20260823`）。他担当と `target/` を共有すると `llvm-cov clean` で
互いの計測データを消し合うため、`CARGO_TARGET_DIR` を scratchpad 配下の専用ディレクトリに
分離して計測した。

| crate | 着手前（[01](u2_cov_harness_query/01-baseline-coverage.log)） | 着地後（[05](u2_cov_harness_query/05-after-coverage.log)） |
| --- | ---: | ---: |
| `modules/harness/infrastructure/` | 1701/2625 (64.80%) | 3048/3104 (98.20%) |
| `modules/harness/claude/` | 2185/2830 (77.21%) | 2921/3041 (96.05%) |
| `modules/core/query/use-case/` | 2313/2864 (80.76%) | 2505/2864 (87.47%) |
| `modules/core/query/interface-adapter/` | 1250/1482 (84.35%) | 1315/1482 (88.73%) |
| `modules/core/infrastructure/` | 1964/2031 (96.70%) | 1978/2042 (96.87%) |

注: 所有 crate だけの計測なので、app 側の統合テストが駆動していた行は「着手前」では未カバーに
数えられている（workspace 計測の 517 行より多い 1,679 行になるのはそのため）。分母が増えて
いるのは、`src` 内の `#[cfg(test)]` モジュールの行も llvm-cov が数えるためである。

### 品質ゲート

| 検査 | 結果 | ログ |
| --- | --- | --- |
| `cargo test -p <5 crate>` | 524 件成功・0 失敗 | [06](u2_cov_harness_query/06-cargo-test-owned-crates.log) |
| `cargo fmt -p <5 crate> --check` | 成功 | [03](u2_cov_harness_query/03-fmt-clippy.log) |
| `cargo clippy -p <5 crate> --all-targets -- -D warnings` | 成功 | 同上 |
| `cargo lint` | 成功 | [04](u2_cov_harness_query/04-cargo-lint.log) |

`cargo fmt --all --check` は他担当の編集中ファイル
（`modules/core/read-model-updater/` 配下）で差分を報告した。私の所有 crate ではない
ので触れていない。なお着地直前に 1 度 `cargo fmt --all` を実行してしまった（所有 crate に
限定すべきだった）。整形のみの変更であり、その後の他担当の編集で上書きされうる。

## 追加したテスト（ファイル・件数・検証した契約）

合計 95 件。`src` 内の `#[cfg(test)]` モジュールへ追加したものと `tests/` の新規ファイルがある。

### `harness-infrastructure`（55 件）— シェル文の解析の拒否・境界ケース

本家フック `aidlc-shell-write-targets` / `aidlc-reviewer-scope` の移植物に対し、
[shell-write-targets-verification.md](../shell-write-targets-verification.md) と
[reviewer-scope-verification.md](../reviewer-scope-verification.md) が固定した字句規則の
境界を契約として固定した。

| ファイル | 件数 | 検証した契約 |
| --- | ---: | --- |
| `src/shell_write_targets.rs` | 11 | 列の件数・添字・filter が発見順を保つ。張り付きスイッチだけで被演算子が無い `cp`/`mv` は宛先を生まない。`$OUT` のような未確定の宛先はディレクトリ宛でも子候補を生まない。逃がした・引用中の `>` はリダイレクトでない。末尾で途切れたリダイレクト・閉じない引用は宛先を生まない。宛先語は `&&` `;` で終わり、`\ ` は語に残る。`copy-item -Destination:` を拾う。`find` の先頭スイッチ（`-H/-L/-P`、`-D`、`-O2`）を起点の前に読み飛ばす。`$PWD` 単独と `${PWD}/x` を作業ディレクトリ起点で解決する。相対の cwd では根を付けずに字句解決する（`.`・`..` の縮約）。 |
| `src/shell_mutation.rs` | 8 | `env -S` の入れ子を 8 段まで辿り 9 段目で打ち切る（値の逃がしを段ごとに二重化して構築）。`command -p/-pp/--` は読み、`-v/-V` は名前問合せで実行対象なし、他は読めない。`builtin -x` は読めない。`busybox`/`toybox` のアプレット欠落は読めない。`timeout` の未知スイッチは読めない。`env` のスイッチ表（`--`、`-u`、`--chdir`、張り付き値、旗の束、`--default-signal` 系）を逐語で辿り、未知は読めない。ラッパーのスイッチ表（`--`、値付き長短スイッチとその値欠落、任意値、数値短縮、旗の束、未知）を逐語で辿る。 |
| `src/shell_invocation.rs` | 8 | `time`/`command`/`exec`/`env`/`nice`/`nohup` の各スイッチ表と値欠落・未知スイッチの扱い。`-S` の値がシェル展開を含めば `Uninspectable`。前置の代入・リダイレクト・`if` を読み飛ばす。実行ファイルは葉名に縮める。 |
| `src/shell_substitutions.rs` | 8 | `$( )` と `` ` ` `` の本文抽出とマスク（長さと改行を保つ）。単一引用内は実行対象にしない。逃がした開始記号は置換でない。閉じない置換は原文のまま。入れ子の括弧・引用・逃がしを尊重する。サロゲートペア（UTF-16 で 2 単位の文字）の後ろの本文も落とさない。 |
| `src/shell_words.rs` | 5 | `$'…'`/`$"…"` は引用を開く。末尾の `\` は文字として残る。行継続は畳み、空の引用語は残る。FCC の `len`/`at`/`filter`、`replace_range` の範囲丸め。 |
| `src/shell_text.rs` | 4 | 引用中の区切りだけを隠し引用符は残す。閉じない引用は終端まで隠す。複数行の引用は改行以外を全て隠す。heredoc 本文を行単位で空白化（`<<-` はタブを剥がして終端判定）。関数定義を空白化し、定義内の引用・逃がしは `}` を閉じない。閉じない定義は隠さない。 |
| `src/shell_segments.rs` | 3 | 区切りで分け空区間は落とす。`${…}`・引用・逃がしの中の `;` で切らない。`#` コメントは語頭のときだけで次の行まで区間を終える。 |
| `src/redirection_free_words.rs` | 2 | `2>&1` の記述子は区切りか終端で終わるときだけ記述子（`>&1x` はファイル名）。FCC の 4 操作。 |
| `src/reviewer_scope_segments.rs` | 1 | FCC の 4 操作と `is_empty`。 |
| `src/reviewer_scope_words.rs` | 2 | 空列の `is_empty`/`at`。FCC の 4 操作。 |
| `src/shell_parse_error.rs` | 2 | `Boundary` の `Display` は `offset=<n>` で `source` なし。`Pattern` は regex のエラー文言と `source` を透過する。 |
| `src/command_segments.rs` | 1 | `"` 内の `\` は `$` `` ` `` `"` `\` 改行の前でだけ逃がし（bash 準拠）。 |

### `harness-claude`（25 件）— JSON 封筒の不正入力の拒否

| ファイル | 件数 | 検証した契約 |
| --- | ---: | --- |
| `tests/envelope_rejections.rs`（新規） | 7 | `SubagentStopEnvelope`: 非 JSON・非 object は `Ok(None)`、`last_assistant_message` が文字列以外は `Err(MessageType)`（文言逐語）、`agent_id` は JS の truthy 判定で文字列化、message は UTF-16 先頭 200 単位。`SessionStartEnvelope`: 空→`startup`、不正 JSON→`malformed`、非 object→`unknown`、`source` は truthy 判定、`rebind_check` は `true` だけ。`SessionEndEnvelope`: 不正 JSON は `unknown`/anonymous、`reason` は truthy 判定。`HumanTurnEnvelope`: 文字列が運ぶ JSON を辿る、数値の JSON は綴りを保つ、配列は最初の空でない要素、真偽値・数値は空、キーの優先順。 |
| `src/delegated_lifecycle.rs` | 8 | 置換・heredoc の中の進行コマンドも検出。`$X/bin/bun` のような動的実行ファイルは拒否し、代入で 1 語に解決できれば判定。`eval` は引数を辿り動的なら `dynamic eval …`。`sh -c` は文字列を辿り、未代入変数・`$` を含む文字列は `dynamic shell command …`。`bun` のスイッチ表（`--`、値付き、旗、`run`）と `-e/--eval` の不透明化。`aidlc-utility.ts` の `--flag` の読み飛ばし（値消費と `=` 付き）。ディスパッチャの名詞・動詞分類（`space-create`、`intent create`、`switch` の引数要求、`--help`、読取り動詞）。読めないラッパーと 9 段の入れ子の拒否文言。 |
| `src/stop_transcript.rs` | 4 | `classification_text` は通常の逃がしと有限数を逐語で保ち、孤立サロゲート→`�`、無限大→`0`、途中で切れた逃がしを伸ばさない。`decode_line` は壊れた JSON を変換で有効化しない。`content` が文字列でも配列でもない user 行は人間の発言でない。非 JSON 行・`message` 無し行は読み飛ばす。 |
| `src/runtime_compile_envelope.rs` | 2 | `bunx`/`.tsx`/`statement` のような部分一致は語境界で退け、後ろの本物まで探索を進める。再帰ガードは `runtime` を語として要求する（`runtimes`/`aidlcruntime` は通す）。 |
| `src/dispatch_rules_envelope.rs` | 2 | `stages[]` の `role` が文字列でない要素は飛ばす。`items[]` の非テキスト要素は空行として数え、本家の `join("\n")` と同じく空行だけでも本文は空にならない。 |
| `src/engine_tool_call.rs` | 2 | `js_string` は ECMAScript の文字列化（`true`/数値/配列は `,` 連結/object は `[object Object]`）。語境界（`aidlc report` と `aidlc reporter` の区別）と `command` 欠落。 |

### `core-query-use-case`（9 件）— View の透過

| ファイル | 件数 | 検証した契約 |
| --- | ---: | --- |
| `tests/view_dto_roundtrip.rs`（新規） | 9 | `ArtifactAuditView`/`SessionAuditView`/`HookHealthView`/`ExecutionView`/`ReportResultView`/`PlanGenerationView`/`TestingContractView`/`AnswerResultView`/`IntentRecordView` が与えた列を逐語で返し、`Option` の不在を不在のまま伝える。`ExecutionView` の投影旗は既定が偽・不在で、ビルダで設定され等価性に含まれる。 |

### `core-query-interface-adapter`（4 件）— 壊れた保存物を成功に丸めない

| ファイル | 件数 | 検証した契約 |
| --- | ---: | --- |
| `tests/broken_read_model_rows.rs`（新規） | 4 | `read_plan_operation` の `as_of` が負なら `find_pending`/`find` とも `ReadModelReadError`（kind `Other`、所在付き）で、View を作らない。範囲内なら行が返り、無い鍵は `Ok(None)`。`intents.json` の不在は `NotFound`、非 JSON・非配列は `InvalidData`、同じ `uuid` の重複・`dirName` 欠落・型違いは `InvalidData`（いずれも所在付き）。 |

### `core-infrastructure`（2 件）

| ファイル | 件数 | 検証した契約 |
| --- | ---: | --- |
| `src/secret_file.rs` | 1 | 親の名前がファイルに取られていれば、鍵は不在ではなく `Unreadable` で止まり、親のファイルを壊さない。 |
| `tests/collections_test.rs` | 1 | `ObjectMembers::default()` は空で `new()` と等しい。 |

## TDD の運用について

対象は既存のプロダクトコードの未カバー行であり、テストを先に書いても最初から green になる
（red を作るにはプロダクトコードを壊す必要があり、brief で禁止されている）。各テストは
「その行が実行される振る舞い」を入力と期待値で主張する契約として書き、期待値が実装の
観測と食い違ったものは実行して食い違いを確認してから、本家の字句規則に照らして期待値側を
直した（例: `shell_text` は引用符自体を残す、`${PWD}` 単独は解決されない、`-Path:` は
`-t` の値を取るため `cp` の宛先になる）。

## dead code 候補（プロダクトコードは変更せず報告のみ）

| ファイル:行 | 内容 | 根拠 |
| --- | --- | --- |
| `modules/harness/infrastructure/src/shell_write_targets.rs:485`（`trimmed == BRACED_PWD` の分岐） | `${PWD}` 単独の綴りを cwd に解決する分岐 | 直前の `trim_matches` が `}` を剥がすため `trimmed` は `${PWD` になり、`BRACED_PWD`（`${PWD}`）と一致しえない。実測: `rm -rf ${PWD}` は宛先なし（`$` を含むため捨てられる）。本家の観測と同じかは未確認なので、修正の要否は人間の裁定を求める。 |
| `modules/harness/infrastructure/src/reviewer_scope_segments.rs:29,73` | `let Some(&c) = characters.get(index) else { break }` | 直前の `while index < characters.len()` が範囲内を保証する。 |
| `modules/harness/infrastructure/src/shell_text.rs:135` | 固定パターンの `expression(...)?` の失敗経路 | パターンはリテラル定数で、`compile_shell_pattern` は常に成功する。 |
| `modules/harness/claude/src/delegated_lifecycle.rs:32` | `segments.at(index)` の `None` 経路 | `0..segments.len()` の範囲内で `at` は必ず `Some`。 |
| `modules/harness/claude/src/dispatch_rules_envelope.rs:233,245,249,257` | `apply` の `items`/`stages` 不在・非 object の失敗経路 | slot は同じ `input` から `brief_slots` が導いたもので、`apply` 時点で `items`/`stages`/対象要素が消えることはない。 |
| `modules/harness/claude/src/runtime_compile_envelope.rs:229` | `starts_with_word_sequence` の `words.first()` が `None` | 呼出し元は常に 2 要素の配列を渡す。 |
| `modules/core/infrastructure/src/append_only.rs:51-54` | `bytes.get(written..)` の `None` 経路 | コード自身のコメントどおりループ不変条件で到達しない。 |
| `modules/core/infrastructure/src/secret_file.rs:187` | `file_name()` が `None` のときの `"secret"` 既定 | `file_name` が無いのは `..`/根のみで、その場合 `read()` が先に `Unreadable` で止まり `mint` に到達しない。 |

## 残った未カバー行（71 行）と理由

| ファイル | 行 | 理由 |
| --- | --- | --- |
| `harness/infrastructure/src/shell_mutation.rs` | 414-432（19） | `WrapperOptions::new` は `const fn` で、13 個の `const` 定数の初期化にだけ使われコンパイル時評価される。実行時に通らないため計測できない。dead code ではない。 |
| `harness/claude/src/dispatch_rules_envelope.rs` | 315,318,325,337,479,483,525,532,554,557（10） | 既存テスト補助関数の `panic!` 腕（想定外の形で落とすための分岐）。テストコード内で通らないのが正しい。 |
| `harness/claude/src/dispatch_rules_envelope.rs` | 233,245,249,257（4） | 上記 dead code 候補（`apply` の失敗経路）。 |
| `core/infrastructure/src/canon_json/parse.rs` | 222,239,251,290,304,346（6） | 既存テスト補助の `panic!` 腕。 |
| `core/infrastructure/src/canon_json/parse.rs` | 394,403,413（3） | proptest の `map_err(... TestCaseError::fail ...)` クロージャ本体で、失敗しない限り通らない。 |
| `core/infrastructure/src/canon_json/value/json_value.rs` | 132,165,183,197（4） | 既存テスト補助の `panic!` 腕。 |
| `core/infrastructure/src/append_only.rs` | 51-54（4）、58-61（4） | 前者は dead code 候補。後者は `File::write` が `Ok(0)` を返す状況で、通常ファイル・パイプでは再現できない（パイプが詰まると `WouldBlock` の `Err` になる）。 |
| `harness/infrastructure/src/shell_write_targets.rs` | 520-522（3） | `Component::Prefix` は Windows のドライブ接頭辞で、Unix では到達しない。 |
| `core/infrastructure/src/secret_file.rs` | 154,178,187（3） | 154 は `create_dir_all` の失敗（親がファイルのときは `read()` が先に失敗するため、競合でしか到達しない）。178 は `hard_link` の `AlreadyExists` 以外の失敗（同一ディレクトリ内なので再現手段がない）。187 は dead code 候補。 |
| `harness/infrastructure/src/reviewer_scope_segments.rs` | 29,73（2） | dead code 候補。 |
| `harness/claude/src/delegated_lifecycle.rs` | 26,32（2） | 26 は `if nested.is_some()` の閉じ括弧（領域境界の計測上の残り）。32 は dead code 候補。 |
| `harness/infrastructure/src/shell_text.rs` | 135（1） | dead code 候補。 |
| `harness/claude/src/runtime_compile_envelope.rs` | 229（1） | dead code 候補。 |
| `core/query/interface-adapter/src/continue_token_dto.rs` | 3 | 行番号を持たない未カバー領域（行の一部の区間）。所有 crate だけの計測では 204-206（`Display` 実装の一部）で、workspace では app 側が駆動している可能性がある。 |
| `core/query/interface-adapter/src/read_model_store.rs` | 2 | 同上（行番号を持たない区間、`find_many` の行エラー経路）。 |

## 作業上の注記

- 他担当と scratchpad・`target/` を共有していたため、初回の計測 JSON と補助スクリプトが
  上書きされた。以後は `scratchpad/hq/` と `CARGO_TARGET_DIR=<scratch>/hq/target` に分離した。
  [01-baseline-coverage.log](u2_cov_harness_query/01-baseline-coverage.log) は分離後に取り直したもの。
- スクリプトファイルの実行はガード（`aidlc-state-transition-guard`）が拒否したため、
  計測コマンドはインラインで実行した。
- `aidlc next` や `.claude/tools/*.ts` は実行していない。承認済み計画・`.claude/` は編集していない。
