# amadeus-ng のエージェント向け指針

## 共通の挙動

- 返答と文書は日本語にする。
- 次に何をすべきかを、重複や漏れのない選択肢で示す。推奨する選択肢を先頭に置き、理由を添える。
- GitHub の Pull Request に言及するときは、リンクを付ける。
- worktree で作業するときは、その worktree で `mise trust` を実行する。

## このリポジトリ

AI-DLC Workflows（`vendor/aidlc-workflows/`）を Rust で再実装したもの。コマンド側・クエリ側・リードモデル更新器（RMU）を
クレートで分けた CQRS+ES の構成で、形式モデル（Quint）を `formal/` に置く。

- このリポジトリの開発に AI-DLC そのものは使わない。自己開発は、開発速度が上がらず逆に落ちたため、2026-10-05 にやめた。
  `/aidlc` の実行や intent の作成をしない。
- upstream 互換の観測可能な契約（ゴールデン `tests/golden/upstream-a277af21/` と `vendor/aidlc-workflows/`）は、
  設計規則より優先する。逐語の文言・出力は変えない。
- `tests/golden/distribution-2.8.2/` は AI-DLC 2.8.2 配布物の写しで、契約テストが本家の正解として読む。手で編集しない。

## コーディング規則

- 正本は `docs/coding-rules/`（1 規則 1 ファイル）。コードを書く前に `README.md` の一覧を読み、変更する層に関係する規則の本文を読む。
  規則どうしが衝突したときの優先順も `README.md` にある。
- 規則はレビューで強制し、印のあるものは `cargo lint`（`tools/lint`）で機械的に強制する。
- 後方互換を残さない。改名や署名の変更では、呼び出し側をすべて一度に直す（`no-backward-compatibility.md`）。

## 検査

変更を完了とする前に、リポジトリの直下で次を実行する（CI の `check` と同じ）。

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo lint
cargo test --workspace
```

- `tools/lint/` を変えたときは、`--manifest-path tools/lint/Cargo.toml` を付けた fmt・clippy・test も実行する。
- `formal/` を変えたときは `bash scripts/quint-gate.sh` も実行する。
- `scripts/` を変えたときは `python3 -B -m unittest discover -s scripts/tests -p 'test_*.py'` も実行する。
- 検査を通すために、テスト・lint の規則・カバレッジの基準を弱めない。
- macOS でワークスペース全体のテストを回すと、子プロセスが signal 9 で落ちることがある（毎回別のテストで、単独では通る）。
  落ちたテストは単独で流し直す。ワークスペースのテストを並行して走らせない。

## takt の中で動くとき

- 実作業は takt の中で行う。起動と監督は Claude Code の指揮役が行う（`CLAUDE.md`）。
- 依頼（issue）の対象・書き込み範囲・受け入れの条件を守る。範囲の外の変更や、依頼と規則に答えの無い設計判断が要るときは、
  自分で判断せずに `blocked` で止まり、理由を返す。
- git の commit・push・PR の作成は、`--pipeline --auto-pr` で起動された takt が、作業の終わりにシステムとして行う。
  各ステップのエージェントは、git の commit・push・マージを自分では行わない。
- `.takt/` は takt の設定と実行の記録で、ソースコードではない。takt の実行中は、今の実行の `context/` と `reports/` 以外の
  `.takt/` を読まない。リポジトリを広く探すとき（`grep -r`・`git grep`・`find`）は `.takt/` を外す。
