#!/usr/bin/env bash
#
# scripts/takt-claude.sh — takt が起動する Claude に、.takt/ の読み取り制限を足す
#
# scripts/run-takt.sh が TAKT_CLAUDE_CLI_PATH にこのファイルを指定する。takt の claude
# プロバイダーはそれを Claude の実行ファイルとして起動するので、ここで本来の実行ファイル
# (AMADEUS_TAKT_CLAUDE_BIN) に、scripts/takt-read-guard.py を PreToolUse フックとして
# 登録する設定を --settings で足す。
#
# .claude/settings.json には書かない。そこに書くと、指揮役の対話の Claude Code にも効いて、
# 指揮役が .takt/runs/ の記録を読めなくなる。--settings で渡せば takt の中の Claude だけに効き、
# 対話セッションには影響しない。
#
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

[ -n "${AMADEUS_TAKT_CLAUDE_BIN:-}" ] || {
  printf 'error: AMADEUS_TAKT_CLAUDE_BIN が未設定です。scripts/run-takt.sh から takt を起動してください\n' >&2
  exit 2
}

# フックのコマンドは設定の JSON に固定で書き、スクリプトの場所は環境変数で渡す
# (パスを JSON やシェルの文字列へ埋め込むときの引用を避ける)。python3 は無いファイルを
# 終了コード 2 で報告し、それが拒否と読まれて全ツールが止まるので、存在を先に確かめる。
export AMADEUS_TAKT_GUARD_SCRIPT="${SCRIPT_DIR}/takt-read-guard.py"
exec "${AMADEUS_TAKT_CLAUDE_BIN}" --settings '{"hooks":{"PreToolUse":[{"matcher":"Read|NotebookRead|LS|Glob|Grep|Bash","hooks":[{"type":"command","command":"[ ! -f \"$AMADEUS_TAKT_GUARD_SCRIPT\" ] || python3 \"$AMADEUS_TAKT_GUARD_SCRIPT\""}]}]}}' "$@"
