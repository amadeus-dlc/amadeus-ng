# シェル書込み先の解析（`shellWriteTargets`）の移植記録

担当 `u2_shell_write_targets`。対象は [review-guards-verification.md](review-guards-verification.md)
「残課題 1」— 本家 2.7.1（固定コミット `a277af218f0df7f325d3b8be7b6d90fce2c5bd40`、実体は
`vendor/aidlc-workflows/dist/claude/.claude/`）の `hooks/review-freeze-command.ts` にある
`shellWriteTargets` を Rust へ移し、review-freeze（レビュー受領証が立っている間の書込み凍結）へ
繋ぐことである。TDD（失敗するテストを先に走らせてから最小実装する進め方）で
red → green → refactor の順に進め、生の実行出力を [shell-write-targets-logs/](shell-write-targets-logs/) に残した。

用語: 「書込み先（write target）」は、そのツール呼出しが書き換えうるファイルのパスをいう。
「リダイレクト」はシェルが出力の送り先を変える綴り（`>`、`>>`、`2>`、`&>` など）である。
「記述子（file descriptor）」は開いている入出力の通し番号で、`2` が標準エラーを指す。

## 出発点（前スライスの実測）

本 build は `Bash` を「書込み先 0 件」として通していた。黙って通してはおらず、`Bash` の呼出しは
10 分に 1 度 `.aidlc-hooks-health/review-freeze.drops` へ「書込み先を検査せずに通した」と残していた。
その結果、**シェル経由の書込みは凍結されなかった**。本スライスでこの穴を塞いだ。

## 実装した変更

### 汎用機構（`harness-infrastructure`）

シェル字句の解析は相手方（Claude Code）の契約を知らない言語拡張なので、
[coding-rules/infrastructure-layer.md](../../../../knowledge/aidlc-shared/coding-rules/infrastructure-layer.md)
の判定基準に従い `harness-infrastructure` へ置いた。公開型は `ShellWriteTargets` 1 つで、
残りは crate 内部に閉じている。

| ファイル | 要点 |
| --- | --- |
| `modules/harness/infrastructure/src/redirection_free_words.rs` | 本家 `shellWords` に対応する語の分割。リダイレクト演算子と記述子番号を語にしない。 |
| `.../src/command_segments.rs` | 本家 `shellCommandSegments` に対応する実行位置の分割。`;` 改行 `\|` `&` だけで切る。 |
| `.../src/shell_options.rs` | 本家 `parseShellArgs` / `attachedPathOptionValues`。被演算子・スイッチ・スイッチの値へ分ける。 |
| `.../src/shell_mutation.rs` | 本家 `shellInvocation` / `invocationMayMutate`。実行ラッパーを剥がしてコマンド名・引数・読めなさ・データ駆動の 4 項目を返す。 |
| `.../src/shell_write_targets.rs` | 本家 `shellWriteTargets`。リダイレクト走査と、コマンドごとの操作対象の抽出。 |
| `.../src/lib.rs` | 上記 5 mod の登録と `ShellWriteTargets` の公開。 |
| `.../Cargo.toml` | 実在ディレクトリ判定のテストのため `tempfile` を dev-dependency に追加。 |

**既存のシェル解析部品は土台にしていない。** `harness-infrastructure` には
`ShellWords` / `split_shell_segments` / `ShellInvocation` が既にあるが、これらは
`aidlc-state-transition-guard` 系の別の上流関数の移植であり、**字句規則が違う**。

| 観点 | 既存 `ShellWords` / `split_shell_segments` | 本スライスの `RedirectionFreeWords` / `split_command_segments` |
| --- | --- | --- |
| `;` `\|` `&` `<` `>` | 語の一部として残す | 語の区切りにする |
| `(` `)` `{` `}` | 区間を切る | 区間を切らない（語は切る） |
| コメント `#` | 区間を打ち切る | 見ない |
| `$'...'` | ANSI-C 引用として扱う | `$` は普通の文字、`'` が引用を開く |
| 空の引用語 `""` | 語として残す | 語にしない |
| 行継続 `\`+改行 | 継続として畳む | 単なる逃がしとして改行を語に入れる |

同じ名前の別実装を並べると混ぜて使う事故が起きるため、両方の doc に相互参照と差分を明記した。
既存側は他担当の移植物なので変更していない。

### 封筒（`harness-claude`）

| ファイル | 要点 |
| --- | --- |
| `modules/harness/claude/src/write_tool_envelope.rs` | `Bash` のとき `ShellWriteTargets::parse(command, cwd)` の結果を書込み先にする。相対綴りの基点は封筒の `cwd`、無ければ呼出側が渡す作業ディレクトリ（本家 `parsed.cwd ?? projectDir`）。 |

`WriteToolEnvelope::parse` の署名を `parse(input, project_dir)` に変え、
「解析未移植」を名乗っていた `shell()` を**削除**した。
[no-backward-compatibility.md](../../../../knowledge/aidlc-shared/coding-rules/no-backward-compatibility.md)
に従い旧口を並立させず、呼出側を同時に直した。

### 実行入口（`aidlc`）

| ファイル | 要点 |
| --- | --- |
| `modules/app/aidlc/src/runtime/review_guards.rs` | `Bash` を早期に通していた分岐を削除し、他の書込み工具と同じ経路へ合流させた。作業ディレクトリを封筒へ渡す。 |
| `modules/app/aidlc/tests/review_guards_contract.rs` | 「未検査で通した」ことを実測していた 1 件を、**凍結されることを実測する 4 件**へ置き換えた。 |

## 既存テストの扱いと理由

削除したのは `a_shell_write_is_allowed_but_recorded_as_uninspected` 1 件である。
このテストは「シェル書込みは本 build では凍結できない」ことと、その代わりに drop へ
目印が残ることを固定していた。**移植によって前提そのものが偽になった**ので、受入条件を
弱めない形で次の 4 件に置き換えた。

| 新しいテスト | 何を固定するか |
| --- | --- |
| `a_shell_write_to_a_declared_artifact_is_refused_and_recorded` | 同じ入力（`printf x >> <宣言成果物>`）が exit 2 で拒否され、`REVIEW_FREEZE_BLOCKED` に `**Tool**: Bash` が残ること。加えて「未検査」の目印と drop が**もう残らない**こと。 |
| `every_shell_form_the_matcher_reads_reaches_the_freeze` | リダイレクト・`rm`・`sed -i`・`tee`・`cp`/`mv` の宛先・`touch`・`2>`・`sudo` 経由・`&&` 連結・封筒の `cwd` を基点にした相対綴り、の 10 形すべてが拒否に届くこと。 |
| `a_read_only_or_unrelated_shell_call_passes_untouched` | 読取り専用・宣言外・`2>&1`・`mkdir` は通り、監査に拒否行が出ないこと。 |
| `a_malformed_shell_envelope_is_allowed_without_a_record` | コマンド欠落・型違い・空文字は fail-open で通ること。 |

置換前の「未検査」観測は 1 件が消えて 4 件が増えており、拒否条件は増えている。

## Red の再現

各段は、テストを先に置いた状態（または実装を無処理へ戻した状態）で実行し、
**コンパイルに成功したうえで**意図した振る舞いの不一致で失敗することを確かめてから実装した。
5 は実装を書いた直後に `parse` を空返しへ戻して red を取り直したものであり、
テストを先に書いた 1〜4・6・7・8 とは順序が異なる。ここを曖昧にしないため明記する。

### 1. 語の分割（`RedirectionFreeWords::parse` を空返しにした状態）

```
cargo test -p harness-infrastructure --lib redirection_free_words
```

```
---- redirection_free_words::tests::quotes_and_escapes_are_unwrapped_into_single_words stdout ----
assertion `left == right` failed
  left: []
 right: ["echo", "a b", "c d", "e f"]

test result: FAILED. 0 passed; 9 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

生ログ: [red-1-redirection-free-words.log](shell-write-targets-logs/red-1-redirection-free-words.log)

### 2. 実行位置の分割（`split_command_segments` を空返しにした状態）

```
cargo test -p harness-infrastructure --lib command_segments
```

```
---- command_segments::tests::a_both_streams_redirection_never_splits_a_command stdout ----
assertion `left == right` failed
  left: []
 right: ["rm x &> log"]

test result: FAILED. 0 passed; 6 failed; 0 ignored; 0 measured; 9 filtered out; finished in 0.00s
```

生ログ: [red-2-command-segments.log](shell-write-targets-logs/red-2-command-segments.log)

### 3. 引数の読み（`ShellOptions::parse` を空返しにした状態）

```
cargo test -p harness-infrastructure --lib shell_options
```

```
---- shell_options::tests::a_short_cluster_names_every_letter_it_carries stdout ----
assertion failed: parsed.has("-r") && parsed.has("-f")

test result: FAILED. 0 passed; 8 failed; 0 ignored; 0 measured; 15 filtered out; finished in 0.00s
```

生ログ: [red-3-shell-options.log](shell-write-targets-logs/red-3-shell-options.log)

### 4. ラッパーの剥がし（`ShellMutation::read_all` を空返しにした状態）

```
cargo test -p harness-infrastructure --lib shell_mutation
```

```
---- shell_mutation::tests::a_name_query_and_a_loop_header_name_no_command stdout ----
assertion `left == right` failed
  left: []
 right: ["rm"]

test result: FAILED. 0 passed; 10 failed; 0 ignored; 0 measured; 23 filtered out; finished in 0.00s
```

生ログ: [red-4-shell-mutation.log](shell-write-targets-logs/red-4-shell-mutation.log)

### 5. 書込み先の抽出（`ShellWriteTargets::parse` を空返しへ戻した状態）

```
cargo test -p harness-infrastructure --lib shell_write_targets
```

```
---- shell_write_targets::tests::an_output_redirection_names_its_file stdout ----
assertion `left == right` failed
  left: []
 right: ["/r/a.md"]

---- shell_write_targets::tests::a_data_stream_or_an_unreadable_launcher_names_the_working_directory stdout ----
assertion `left == right` failed
  left: []
 right: ["/r"]

test result: FAILED. 2 passed; 15 failed; 0 ignored; 0 measured; 33 filtered out; finished in 0.00s
```

生ログ: [red-5-shell-write-targets.log](shell-write-targets-logs/red-5-shell-write-targets.log)

### 6. 封筒（`Bash` の枝を空の書込み先にした状態）

```
cargo test -p harness-claude --lib write_tool_envelope
```

```
---- write_tool_envelope::tests::a_shell_write_names_the_targets_its_command_would_change stdout ----
assertion `left == right` failed
  left: []
 right: ["/w/x/inception/requirements-analysis/requirements.md"]

test result: FAILED. 7 passed; 2 failed; 0 ignored; 0 measured; 20 filtered out; finished in 0.00s
```

生ログ: [red-6-write-tool-envelope.log](shell-write-targets-logs/red-6-write-tool-envelope.log)

### 7. 実行入口（旧テストが偽になったことの実測）

置換前の `a_shell_write_is_allowed_but_recorded_as_uninspected` を、封筒を繋いだ状態で走らせた。
**同じ入力が exit 0 ではなく exit 2 になり、逐語の拒否理由が標準エラーへ出る**ことを確認してから、
テストを置き換えた。

```
cargo test -p aidlc --test review_guards_contract a_shell_write
```

```
---- a_shell_write_is_allowed_but_recorded_as_uninspected stdout ----
assertion `left == right` failed: シェル書込みは本 build では凍結できない: Output {
  status: ExitStatus(unix_wait_status(512)), stdout: "",
  stderr: "review-freeze: \".../inception/requirements-analysis/requirements.md\" is this stage's
  output document for stage \"requirements-analysis\", and its latest review is final. ..." }
  left: Some(2)
 right: Some(0)

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 11 filtered out; finished in 71.99s
```

生ログ: [red-7-review-guards-shell.log](shell-write-targets-logs/red-7-review-guards-shell.log)

この 71.99 秒は他担当の同時コンパイルによるもので、解析の遅さではない。置換後の同じ条件で
2.32 秒（本テスト単体）を実測している。

### 8. `env` の読めないスイッチ（自分の移植ミスを見つけて直した段）

移植後に本家と 1 行ずつ突き合わせ直したところ、`env` の未知スイッチの扱いが**本家と違って
いた**。本家 `shellInvocation` は知らない `env` スイッチを `{ambiguous: true}`（実行対象が
**読めない** → 作業ディレクトリ自身が書込み先）で返し、`-S` の値欠落だけを `null`（実行対象が
**無い**）で返す。移植初版は両方を `null` にしており、`env --unknown-switch rm x` が
**何の書込み先も生まない**（凍結が外れる方向の）差になっていた。

先にテストを足して red を確認してから直した。

```
cargo test -p harness-infrastructure --lib
```

```
---- shell_mutation::tests::an_unreadable_env_switch_is_ambiguous_but_a_missing_split_value_names_nothing stdout ----
assertion `left == right` failed: []
  left: 0
 right: 1

---- shell_write_targets::tests::a_data_stream_or_an_unreadable_launcher_names_the_working_directory stdout ----
assertion `left == right` failed
  left: []
 right: ["/r"]

test result: FAILED. 53 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

生ログ: [red-8-env-ambiguous.log](shell-write-targets-logs/red-8-env-ambiguous.log)、
是正後: [green-12-env-ambiguous.log](shell-write-targets-logs/green-12-env-ambiguous.log)

いずれの red も「テストが見つからない」「依存不足」「構文不正」ではなく、意図した振る舞いの
不一致で失敗している。

## Green の一覧

### 追加・変更した境界

| 検査 | コマンド | 結果 |
| --- | --- | --- |
| 語の分割 | `cargo test -p harness-infrastructure --lib redirection_free_words` | 11 passed / 0 failed |
| 実行位置の分割 | `cargo test -p harness-infrastructure --lib command_segments` | 7 passed / 0 failed |
| 引数の読み | `cargo test -p harness-infrastructure --lib shell_options` | 8 passed / 0 failed |
| ラッパーの剥がし | `cargo test -p harness-infrastructure --lib shell_mutation` | 11 passed / 0 failed |
| 書込み先の抽出 | `cargo test -p harness-infrastructure --lib shell_write_targets` | 18 passed / 0 failed |
| crate 全体 | `cargo test -p harness-infrastructure` | 55 passed / 0 failed |
| 封筒 | `cargo test -p harness-claude --lib` | 29 passed / 0 failed |
| harness-claude 全 target | `cargo test -p harness-claude` | lib 29 + 統合 4 target 各 1、全 target 0 failed |
| 凍結フックの入口 | `cargo test -p aidlc --test review_guards_contract` | 15 passed / 0 failed |

### 変更した境界の既存回帰

| コマンド | 結果 |
| --- | --- |
| `cargo test -p aidlc --lib` | 301 passed / 0 failed |
| `cargo test -p aidlc --test claude_hook_contract` | 10 passed / 0 failed |
| `cargo test -p aidlc --test session_hooks_contract` | 16 passed / 0 failed |
| `cargo test -p aidlc --test review_receipt_contract` | 7 passed / 0 failed |
| `cargo test -p aidlc --test intent_lifecycle` | 123 passed / 0 failed |
| `cargo test -p aidlc --test next_branches` | 44 passed / 0 failed |
| `cargo test -p aidlc --test cli_golden_test` | 10 passed / 0 failed |
| `cargo test -p aidlc --test steering_across_processes` | 本報告時点で実行中（他担当の同時ビルドで待たされている） |
| `cargo test -p aidlc --test diagnostic_record_contract` | 同上 |

生ログ: [green-11-app-regression.log](shell-write-targets-logs/green-11-app-regression.log)

### 静的検査

| 検査 | コマンド | 結果 |
| --- | --- | --- |
| 整形 | `rustfmt --edition 2024 --config-path rustfmt.toml <変更ファイル>` | 差分なし |
| 静的検査 | `cargo clippy -p harness-infrastructure -p harness-claude --all-targets -- -D warnings` | 所見なし |
| 静的検査（app） | `cargo clippy -p aidlc --all-targets -- -D warnings -A clippy::indexing_slicing -A clippy::string_slice` | 所見なし |
| 独自検査 | `cargo lint` | 所見 0 件（出力なし・終了 0） |

`clippy` は途中 4 件の所見を出し、いずれも是正した。

- `while_let_loop` / `nonminimal_bool` / `missing_const_for_fn`（`shell_mutation.rs`）— 表現を直した。
- `unused_self`（契約テストの入力組立て）— 自由関数へ移した。

`app` の 2 除外は前スライスと同じで、他担当が編集中の
`modules/harness/claude/src/runtime_compile_envelope.rs` が出す既存所見を避けるためである。
**本担当の変更ではこの 2 つを使っていない。**

## 本家との突き合わせ

### 実行による差分採取（101 件、差分 0）

ソース読解だけで済ませず、**本家の実装を実際に走らせて同じ入力の出力を比較した**。
`shellWriteTargets` は副作用の無い純粋な関数なので、フック全体を動かさずに関数単位で
比較できる。

- corpus: [differential-corpus.txt](shell-write-targets-logs/differential-corpus.txt) 101 行。
  リダイレクトの全形、記述子の複製・閉鎖、`$PWD`・グロブ・展開、変更系コマンド全種、
  `sed`/`perl` の `-i` 有無、`find` の各述語、実行ラッパー 12 種、PowerShell 系、連結、
  引用・エスケープ、`dd`、`rsync`、ヒアドキュメント、`for` ループを含む。作業ディレクトリは `/r` 固定。
- 比較対象 (1): 本家 + 承認済み是正パッチ = `.claude/hooks/review-freeze-command.ts`。
  この来歴は**実際に組み直して検証した** — vendor の固定コミット `a277af21` の同名ファイルへ
  `scripts/aidlc-sync/patches/shell-redirection-tokens.patch` を当てた結果と、リポジトリの
  `.claude/` 側はバイト一致する。
  出力 [differential-upstream-patched.tsv](shell-write-targets-logs/differential-upstream-patched.tsv)。
- 比較対象 (2): 本家そのまま
  `vendor/aidlc-workflows/dist/claude/.claude/hooks/review-freeze-command.ts`。
  出力 [differential-upstream-vendor-unpatched.tsv](shell-write-targets-logs/differential-upstream-vendor-unpatched.tsv)。
- 本 build: `harness_infrastructure::ShellWriteTargets`。
  出力 [differential-rust.tsv](shell-write-targets-logs/differential-rust.tsv)。

| 比較 | 差分 |
| --- | --- |
| (1) 本家 + パッチ ↔ 本 build | **0 行 / 101 件** |
| (2) 本家そのまま ↔ 本 build | 3 行 |

(2) の 3 行はすべて記述子の複製・閉鎖であり、**未是正の本家だけが `/r/2` を書込み先として
拾う**（[differential-vs-unpatched.diff](shell-write-targets-logs/differential-vs-unpatched.diff)）。

```
< "rm /r/a.md 2>&1"	["/r/a.md","/r/2"]
> "rm /r/a.md 2>&1"	["/r/a.md"]
< "rm /r/a.md 2>&-"	["/r/a.md","/r/2"]
> "rm /r/a.md 2>&-"	["/r/a.md"]
< "rm /r/a.md 2>&1 | tee /r/b.md"	["/r/a.md","/r/2","/r/b.md"]
> "rm /r/a.md 2>&1 | tee /r/b.md"	["/r/a.md","/r/b.md"]
```

これはパッチの意図そのものであり、**本 build がパッチ後の側に一致している**ことを実行で
確かめた。手順は [differential-README.md](shell-write-targets-logs/differential-README.md) にある。

**この採取は一度きりで、固定化していない。** 比較に使った一時テストは記録後に削除しており、
CI で回る検査にはなっていない（[残課題](#残課題) 1）。

**差分採取が踏んでいない枝が 1 つある。** corpus の作業ディレクトリ `/r` は実在しないので、
`cp` / `install` / `mv` の宛先が**実在ディレクトリ**のときの分岐（本家 `statSync`）は
両側とも「ディレクトリでない」で一致しており、区別できていない。この枝は本 build 側の
単体テスト `an_existing_directory_destination_is_detected_from_the_filesystem` が
実ディレクトリを作って固定しているだけである。`-t` 指定と源が複数のときのディレクトリ判定は
corpus に入っており差分 0 を実測した。

### ソース読解で突き合わせた観測

出典は `vendor/aidlc-workflows/dist/claude/.claude/hooks/review-freeze-command.ts:7-125,585-1068`
（固定コミット `a277af21`）と、当リポジトリの是正パッチ
`scripts/aidlc-sync/patches/shell-redirection-tokens.patch` を当てた
`.claude/hooks/review-freeze-command.ts:7-125` である。行ごとの対応は次のとおり。

| 観測 | 本家の出典 | 本 build |
| --- | --- | --- |
| `Bash` は `shellWriteTargets(command, cwd)` の結果を書込み先にする | `review-freeze-command.ts:1053-1056` | 同じ。 |
| 相対綴りの基点は封筒の `cwd`、無ければ project dir | `aidlc-review-freeze.ts:279` | 同じ。 |
| 出力リダイレクト `>` `>>` `>\|` `>&file` `&>file` の送り先を拾う | `:846-883` | 同じ。詰めた綴り `printf x>>file` も引用付きも拾う。 |
| `2>&1` `2>&-` `<&0` は記述子の複製・閉鎖で宛先を生まない | `:871-878`、パッチ | 同じ。**幽霊コマンドも作らない**（下記）。 |
| `$PWD` / `${PWD}/` だけ解決し、他の展開・グロブ（`$` `` ` `` `*` `?`）を含む綴りは宛先にしない | `:616-630` | 同じ。 |
| 端の `,` `:` `[` `]` `{` `}` `(` `)` と先頭の `of=` を落として正規化する | 同上 | 同じ。 |
| 変更系コマンドの列挙 | `:715-806` | 逐語。`cp` `dd` `install` `mv` `rm` `rsync` `tee` `touch` `truncate` `unlink` `copy-item` と PowerShell 系 3 集合、`sed -i` / `perl -i` / `find -delete|-fprint*`。 |
| `cp` は宛先だけ、`mv` は源と宛先の両方を宛先にする | `:915-973` | 同じ。 |
| ディレクトリ宛（`-t` 指定・源が複数・実在ディレクトリ）は各源の葉名を足した候補も並べる | `:826-842` | 同じ。実在判定だけ実ファイルシステムを見る。 |
| `install -d` は被演算子すべてを宛先にする | `:943-945` | 同じ。 |
| `sed` / `perl` はプログラム本体がスイッチで与えられていなければ最初の被演算子を飛ばす | `:997-1010` | 同じ。飛ばすスイッチの一覧も逐語（sed は `-e` `-f` `--expression` `--file`、perl は `-e` `-E`）。 |
| `find` は `-delete` のときだけ走査起点を宛先にし、`-fprint*` の出力先は常に拾う | `:1011-1022` | 同じ。起点が無ければ `.`。 |
| `rsync` は宛先を常に、源は `--remove-source-files` のときだけ | `:1035-1041` | 同じ。 |
| `dd` は `of=` の綴りだけを宛先にし、他の引数を見ない | `:899-902` | 同じ。 |
| 実行ラッパー（`sudo` `env` `nice` `nohup` `xargs` `timeout` `busybox` `command` `builtin` ほか）を剥がす | `:230-548` | 同じ。スイッチ仕様も逐語。 |
| 綴りを読み切れないラッパーは作業ディレクトリ自身を宛先にする | `:894-897` | 同じ。 |
| `xargs` などデータ駆動 × 変更系は作業ディレクトリを足す | `:898` | 同じ。 |
| `command -v` と `for` / `case` / `select` の見出しはコマンドを名乗らない | `:255-276` | 同じ。 |
| 実行ファイル名は葉名・小文字・Windows 拡張子落とし | `:141-145` | 同じ。 |
| 重複は発見順を保って 1 度だけ残す | `:1044` | 同じ。 |
| 拒否は既存の凍結判定と同じ経路（接尾辞一致 → 受領証 → exit 2 と `REVIEW_FREEZE_BLOCKED`） | `aidlc-review-freeze.ts:280-375` | 同じ。工具名は `Bash` として監査に載る。 |

### 是正パッチの意図の反映

`scripts/aidlc-sync/patches/shell-redirection-tokens.patch` は本家の既知不具合 2 つを直す。
どちらも**移植側で最初から是正した形**にしてある。

1. **`shellCommandSegments` が `2>&1` の `&` をコマンド区切りと誤認する。**
   是正前は `rm x 2>&1` が `rm x 2>` と `1` の 2 区間になり、幽霊コマンド `1` が生まれた。
   本 build は直前の 1 文字を見て、`>` `<` の直後の `&` と `>` の直前の `&` を区切りにしない。
   固定テスト: `command_segments::a_descriptor_duplication_never_splits_a_command`、
   `a_both_streams_redirection_never_splits_a_command`。
2. **`shellWords` が記述子番号を語に混ぜる。**
   是正前は `rm x 2>&1` の語が `["rm","x","2"]` になり、**`2` が `rm` の削除対象として
   宛先に載った**。本 build は裸の記述子番号を演算子側へ寄せ、`>&n` `>&-` `<&n` `<&-` を
   演算子ごと読み飛ばす。固定テスト:
   `redirection_free_words::a_descriptor_duplication_names_no_word_at_all`、
   `shell_write_targets::a_descriptor_number_never_becomes_an_operand_of_the_command`、
   プロセス面では `a_read_only_or_unrelated_shell_call_passes_untouched` の
   `rm -f /tmp/other.md 2>&1`。

### 確認できなかった範囲・写していない分岐

- **突き合わせたのは `shellWriteTargets` 関数だけで、フック全体の実走行ではない。**
  上の差分採取は純粋関数の入出力比較である。`tests/golden/selfhost-stage1/` に review-freeze
  フックの採取は無く、本工程でも新規採取していない。**フックの標準入出力・exit・監査行を
  本家と並走させて比べた事実は無い**（本 build 側のプロセス実測はある）。
- **依頼文が例示した `mkdir` は本家の列挙に無い。** 本家 `invocationMayMutate` と
  `shellWriteTargets` のどちらにも `mkdir` の枝は無く、`mkdir -p x` は宛先 0 件で通る。
  依頼文自身が「本家が列挙するもの」と限定しているため、**上流に合わせた**。
  読み替えではなく、差分としてここに記録する。裁定が要るなら本行を根拠にできる。
- **`shellCommandInvocations` と `shellCommandAltersExecutableResolution` は移していない。**
  この 2 つは同じファイルが `aidlc-plan-approval-guard.ts` へ輸出している別用途の口であり、
  凍結判定は読まない。付随して本家 `ShellInvocationDetails` の `executable` / `launchers` /
  `dataDrivenMutation` / `executableResolutionChanged` の 4 項目も持っていない。
  計画承認ガードの移植を行う担当は、この 4 項目を足す必要がある。
- **`${PWD}` 単独は本家でも宛先にならない。** 正規化が端の `}` を先に落とすため
  `${PWD}` は `${PWD` になり、`$PWD`/`${PWD}` の等値判定に当たらず、続く `$` 判定で捨てられる。
  本 build も同じ挙動にしてある（等値判定の枝は逐語で写したが、到達しない）。
  `${PWD}/x` は正しく解決する。
- **`cwd` が相対のときの挙動が本家と違う。** 本家の `path.resolve` はプロセスの作業
  ディレクトリを継ぎ足すが、本 build は相対のまま字句正規化する。フックが受け取る `cwd` は
  常に絶対であり、宣言成果物の照合は接尾辞一致なので凍結の結論は変わらない。
- **`reverse-engineering` の codekb 分岐、per-unit 受領証、reviewer-scope の越境拒否**は
  前スライスと同じく未移植のままである（本スライスの範囲外）。

## 扱わない字句と理由

以下は**本家も扱っていない**ため、本 build も扱わない。推測で拡張していない。
「扱わない」の意味は、その綴りを展開・追跡しないということであり、
結果として宛先が採れない場合は**凍結されない**（fail-open）。

| 字句 | 扱い | 理由 |
| --- | --- | --- |
| ヒアドキュメント `<<EOF … EOF` | 演算子 `<<` は落とし、タグは語にする。**本文は隠さない** | 本家 `shellWriteTargets` に heredoc の遮蔽が無い。本文中の `>` はリダイレクト走査に拾われうる |
| コマンド置換 `$(…)` / `` `…` `` | 中身を再帰的に解析しない | 本家に再帰が無い。`$` `` ` `` を含む綴りは正規化で捨てられる |
| 変数展開 `$VAR` `${VAR}` | `$PWD` 起点以外は解決せず、宛先にしない | 実行前に確定しない値を凍結の材料にしない（本家 `:628` の明示的な拒否） |
| グロブ `*` `?` | 展開しない。宛先にしない | 同上 |
| ブレース展開 `{a,b}` | 展開しない。`{` `}` は端のトリムで落ちるだけ | 本家に展開が無い |
| 部分シェル `( … )` / グループ `{ … }` | 区間を分けない（語は分ける） | 本家 `shellCommandSegments` は `;` 改行 `\|` `&` だけで切る |
| ANSI-C 引用 `$'…'` | `$` は普通の文字、`'` が引用を開く | 本家 `shellWords` に `$'` の特別扱いが無い |
| 行継続 `\`+改行 | 逃がしとして改行を語に入れる（行は繋がらない） | 本家 `shellWords` の `\` 処理が単一 |
| コメント `#` | 見ない | 本家 `shellCommandSegments` にコメント処理が無い |
| 別名・関数定義・`set -o noclobber` | 展開・追跡しない | 本家に無い |
| 引用しない Windows パス `C:\a\b` | `\` が逃がしとして消え、1 語に潰れる | 本家 `shellWords` と同じ。引用すれば葉名を取れる |

## 残課題

1. 差分採取の固定化（golden 化）と、フック全体の実走行採取。関数単位の 101 件は差分 0 を
   実測したが、一時テストで採ったもので CI に載っていない。corpus と 3 系統の出力は
   [shell-write-targets-logs/](shell-write-targets-logs/) に残してあるので、golden の
   来歴規則に沿って固定化する担当が拾える。
2. `mkdir` を凍結対象にするかどうかの裁定。現状は上流どおり対象外。
3. `shellCommandInvocations` / `shellCommandAltersExecutableResolution` と、それが要求する
   `ShellInvocationDetails` の 4 項目の移植（計画承認ガードの担当）。
4. reviewer-scope の越境拒否と `REVIEWER_SCOPE_BLOCKED` の保存（別スライス、未着手のまま）。
5. per-unit 受領証を配線するかどうかの裁定（前スライスからの持ち越し）。
6. `cargo test --workspace` と `cargo clippy --workspace`、カバレッジ（行 90.0% 床・相対ゲート）、
   Quint / ITF は**本スライスでは実行していない**。承認済み
   [テスト手順](unit-test-instructions.md) がこれらを「B1 統合前の共通検査」と位置づけているため、
   本書は単位限定・回帰コマンドまでを実測した。
7. フックの登録（`.claude/settings.json` を TS から `aidlc hook <name>` へ向ける）は
   セルフホスト切替の作業であり、本スライスでは触っていない。

## Sources

- 本家 `vendor/aidlc-workflows/dist/claude/.claude/hooks/review-freeze-command.ts:7-125,585-1068`、
  `hooks/aidlc-review-freeze.ts:44-49,270-300`、
  `hooks/aidlc-plan-approval-guard.ts:596-610,753-760`。
- 是正パッチ `scripts/aidlc-sync/patches/shell-redirection-tokens.patch` と、
  それを当てた `.claude/hooks/review-freeze-command.ts:7-125`。
- 現コード `modules/harness/infrastructure/src/{shell_words,shell_segments,shell_invocation}.rs`
  （既存の別移植）、`modules/harness/claude/src/write_tool_envelope.rs`、
  `modules/app/aidlc/src/runtime/review_guards.rs`、
  `modules/core/command/domain/src/orchestration/{write_target,write_targets}.rs`。
- 前スライスの記録 [review-guards-verification.md](review-guards-verification.md)、
  承認済み [実装計画](code-generation-plan.md)、[テスト手順](unit-test-instructions.md)。
- 設計規則 [coding-rules/](../../../../knowledge/aidlc-shared/coding-rules/) の
  `infrastructure-layer.md` / `abstract-data-type.md` / `first-class-collections.md` /
  `no-backward-compatibility.md` / `field-visibility.md`。

## Assumptions & Open Questions

- 上の「残課題」1〜5 は裁定または後続スライスが要る。読み替えて閉じていない。
- とくに 2（`mkdir`）は依頼文の例示と上流の実装が食い違う点であり、上流に合わせた判断を
  人間の裁定にかける必要がある。
