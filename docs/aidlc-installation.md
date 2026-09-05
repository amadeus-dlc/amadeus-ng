# AI-DLC の導入と更新

配布元は `vendor/aidlc-workflows/` の Git submodule で管理する。参照先は
`https://github.com/j5ik2o/aidlc-workflows.git`。親リポジトリが記録するコミットが
導入元の正本であり、ローカルマシンの絶対パスやブランチの最新状態には依存しない。

| 配布物 | 導入先 |
| --- | --- |
| `dist/claude/.claude/` | `.claude/` |
| `dist/codex/.codex/` | `.codex/` |
| `dist/codex/.agents/` | `.agents/` |
| `dist/kimi/.kimi-code/` | `.kimi-code/` |

導入済みのファイルも親リポジトリにコミットする。submodule の参照変更と
コピー後の差分は同じコミットにまとめる。`dist/` は配布元に生成済みなので、
通常のコピーには依存パッケージのインストールやビルドは不要。

このプロジェクトは **AWS Bedrock を使用しない**。各ハーネスの通常の認証設定を使う。
Claude の配布設定・個人設定サンプルには `claude-without-bedrock.patch` を適用し、
Bedrock の有効化とモデル ID・リージョンの初期指定を取り除く。
既存の設定や `.claude/settings.local.json` で Bedrock が有効になっていた場合は、
同期スクリプトが更新前に拒否する。AWS の設計資料やエージェント名、任意の AWS MCP
接続は Bedrock の有効化設定とは別であり、この更新では削除しない。

## clone・worktree 作成後

```sh
git submodule update --init --recursive
mise trust
bun scripts/aidlc-sync.ts --check
```

submodule の版は明示的に選ぶ。同期スクリプトは fetch や版比較を行わないため、
`2.7.1-j5ik2o.1` を通常の SemVer で誤ってダウングレード扱いすることもない。

## 配布元の更新を取り込む

1. フォーク側の変更をコミット・push し、共有リポジトリから取得可能にする。
2. 次のコマンドで対象コミットを選び、差分を確認する。`<commit>` は取り込む
   完全なコミットハッシュに置き換える。

   ```sh
   git -C vendor/aidlc-workflows fetch origin
   git -C vendor/aidlc-workflows checkout --detach <commit>
   bun scripts/aidlc-sync.ts
   ```

3. `REVIEW` が表示された設定は、配布元の変更と既存設定を比較して必要な変更を
   手でマージする。独自のコード変更はパッチへ移す。
4. この作業ツリーで実行中の他のハーネスを終了してから適用する。

   ```sh
   bun scripts/aidlc-sync.ts --apply
   # REVIEW の差分を確認・マージ済みの場合だけ使用する。
   bun scripts/aidlc-sync.ts --apply --accept-preserved
   ```

5. 検証し、新しいセッションで各ハーネスのフックと作業再開を確認する。

   ```sh
   bun test ./scripts/aidlc-sync.test.ts ./.codex/hooks/aidlc-codex-adapter.test.ts
   bun scripts/aidlc-sync.ts --check
   bun .claude/tools/aidlc-utility.ts doctor
   bun .codex/tools/aidlc-utility.ts doctor
   bun .kimi-code/tools/aidlc-utility.ts doctor
   ```

`--check` は同期済みなら終了コード 0、差分またはエラーがあれば 1 を返す。
引数なしでは差分表示だけを行い、導入先を書き換えない。

## 削除と保持

`scripts/aidlc-sync/installed.json` は、前回コピーしたファイルのパス・ハッシュ・
実行権限を記録する。バージョンの正本は submodule の gitlink であり、台帳は
削除対象とローカル変更の検出に使う。

更新時には一時ディレクトリに配布物をコピーし、パッチを適用してから検証する。
導入先では、台帳にある配布ファイルを退避・削除し、新しい配布物をコピーする。
これにより、配布元で削除されたファイルが残らない。台帳にない独自ファイルは
保持する。管理対象にローカル変更がある場合や、追加ファイルが管理外ファイルと
衝突する場合は、書き換える前にエラーにする。

設定の保持対象は `scripts/aidlc-sync.ts` の `PRESERVE` が正本。
Claude の設定と `CLAUDE.md`、Codex の設定・フック登録・許可ルール、Kimi の MCP
設定、各ハーネスのルール参照を保持する。初回導入で存在しないものだけ配布物から
配置する。保持対象の配布元に変更があれば `REVIEW` を表示する。

ルートの `AGENTS.md`、`.gitignore`、`aidlc/` 全体は同期対象外。ルール・進行状態・
監査記録を初期配布物で上書きしない。このスクリプトは、本リポジトリに既にある
共有ワークスペースを前提とする。新しいアプリへ流用する場合は、その初期配置が別途必要。
Rust の互換性基準である本家の固定仕様・ゴールデンデータも更新対象外。

## 独自パッチ

`scripts/aidlc-sync/patches/*.patch` をファイル名順に適用する。パッチのパスは
プロジェクトルートからの相対パスとする。配布元に同じ修正が入った場合も含め、
適用できないパッチは自動で無視せずエラーにする。内容を確認してパッチを更新・削除する。

`codex-input-rewrite.patch` は既存の Codex PreToolUse 入力書き換え修正を保持する。
`.codex/hooks/aidlc-codex-adapter.test.ts` で入力書き換えの出力を検証する。

## 初回の既存環境からの移行

台帳のない既存環境だけ、以前の配布コミットから管理対象を復元する。
旧配布物と異なるローカル変更は、保持対象またはパッチとして説明できなければ拒否する。

```sh
bun scripts/aidlc-sync.ts --adopt-from a277af218f0df7f325d3b8be7b6d90fce2c5bd40
bun scripts/aidlc-sync.ts --adopt-from a277af218f0df7f325d3b8be7b6d90fce2c5bd40 --apply --accept-preserved
```

台帳が作成された後は `--adopt-from` を指定しない。

## 失敗時の復元

適用前のファイルと台帳を `.aidlc-sync/backups/<id>/files/` に退避する。
コピーに失敗した場合は自動で元へ戻し、退避も残す。退避は Git 管理外。

プロセスを強制終了した場合は自動復元できない。表示された退避先、または
`.aidlc-sync/backups/` の該当更新を確認する。`restore.json` の `targets` は更新対象、
`existing` は更新前に存在したファイルである。対象ファイルだけを削除し、`files/`
の内容をプロジェクトへ戻す。更新プロセスが終了していることを確認してから
`.aidlc-sync/lock/` を削除する。管理外ファイルや `aidlc/` を削除してはいけない。

コミット済みの版へ戻す場合は、submodule の参照・導入物・パッチ・台帳を一緒に戻す。
submodule の参照だけを戻すと、導入物との対応が崩れる。

## ハーネスのフック登録

Claude はプロジェクトのフック設定を利用する。フック更新後は新しいセッションで確認する。
Codex の `.codex/hooks.json` を変更した場合は、配布元の
`docs/guide/harnesses/codex-cli.md` に従ってフックの信頼設定を再生成する。

Kimi はプロジェクト内のフック登録を読まないため、各マシンで一度
`.kimi-code/hooks.snippet.toml` のフックをユーザー設定へマージする。
既存の登録を保ち、同じフックを重複登録しない。`KIMI_CODE_HOME` を使用する環境では
そのディレクトリ、それ以外は `~/.kimi-code/config.toml` が対象となる。
登録はユーザー全体に作用する。以後 snippet が変わった場合も、このマージが必要。
設定変更後は Kimi を再起動し、`/skill:aidlc --doctor` で確認する。
