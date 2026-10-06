# distribution-2.8.2 — AI-DLC 2.8.2 配布物の写し（テストの参照物）

このリポジトリには以前、AI-DLC 2.8.2 の Claude 向け配布物（`.claude/`）と memory 層の雛形
（`aidlc/spaces/default/memory/`）がインストールされていた。Rust の契約テストはその実バイトを
「本家の正解」として読んでいた。2026-10-05 のオーナー裁定で、このリポジトリの開発に AI-DLC を
使うのをやめ、インストールを外すことにしたので、テストが読む部分だけをここへ固定した。

- 出所: main `34f21717a` の時点でコミットされていたバイトを、そのまま取り出した（`git archive`）。
- 版: `.claude/tools/data/aidlc-manifest.json` の `frameworkVersion` が `2.8.2`。
- 置き方: `.claude/` は `claude/`（点なし）と綴って置く。`.claude` のままだと、Claude Code が入れ子の設定
  ディレクトリとして中のスキル・エージェントを見つけ、ここで作業するセッションに AI-DLC が紛れ込むため。
  テストは `.claude/...` のパスを `claude/...` へ読み替えて読む。
- 入れたもの: `.claude/` の `aidlc-common/`・`agents/`・`hooks/`・`scopes/`・`tools/`・`settings.json`、
  `knowledge/aidlc-shared/memory-template.md`（`scripts/aidlc-selfhost/e2e/replay-bugfix.sh` が使う）、
  `aidlc/spaces/default/memory/`。
- 入れなかったもの: `.claude/CLAUDE.md`・`.claude/rules/`・`.claude/skills/`・`.claude/sensors/`・
  `.claude/knowledge/` の残り。テストは読まない（`required-surface.json` が出典に挙げるスキルは、
  `tests/golden/selfhost-stage1/required-surface-sources/` の写しを読む）。

中身は配布物の実バイトなので、手で編集しない。本家の新しい版に追従するときは、別の写しを作る。
