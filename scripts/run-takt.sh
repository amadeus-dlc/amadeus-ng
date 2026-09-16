#!/usr/bin/env bash
#
# scripts/run-takt.sh — 使う Claude アカウントを固定して takt を起動する
#
# takt (provider: claude) は親プロセスの環境変数をそのまま Claude に渡す。シェルに
# CLAUDE_CODE_OAUTH_TOKEN が残っていると Claude はそのトークンを優先し、/login し直した
# アカウントの認証情報を使わない。そのため takt の起動前に次の 2 つを必ず行う
# (片方だけでは足りない):
#   1. CLAUDE_CODE_OAUTH_TOKEN を unset する
#   2. CLAUDE_CONFIG_DIR を、使いたいアカウントの設定ディレクトリに設定する
#
# 使い方:
#   scripts/run-takt.sh [--config-dir <dir>] [--] [takt の引数...]
#
#   --config-dir を省略したときは、呼び出し元の CLAUDE_CONFIG_DIR を使う。どちらも無いときは
#   既定の ~/.claude がどのアカウントかを確かめずに走らせることになるので、エラーで止める。
#   takt の引数を省略したときは `takt run` を実行する。
#
# 例:
#   scripts/run-takt.sh --config-dir ~/.claude-<account>        # takt run
#   scripts/run-takt.sh --config-dir ~/.claude-<account> list   # takt list
#
# bash 3.2 (macOS 標準) 互換のため、配列は使用しない。
#
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

usage() {
  cat <<'EOF'
usage: scripts/run-takt.sh [--config-dir <dir>] [--] [takt の引数...]

  --config-dir <dir>  使う Claude アカウントの設定ディレクトリ (省略時は呼び出し元の CLAUDE_CONFIG_DIR)
  -h, --help          この説明を表示する

takt の引数を省略したときは `takt run` を実行する。
EOF
}

die() {
  printf 'error: %s\n' "$1" >&2
  exit 2
}

CONFIG_DIR="${CLAUDE_CONFIG_DIR:-}"
while [ "$#" -gt 0 ]; do
  case "$1" in
    --config-dir)
      [ "$#" -ge 2 ] || die "--config-dir には値が必要です"
      CONFIG_DIR="$2"
      shift 2
      ;;
    --config-dir=*)
      CONFIG_DIR="${1#--config-dir=}"
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    --)
      shift
      break
      ;;
    *)
      break
      ;;
  esac
done

[ -n "${CONFIG_DIR}" ] \
  || die "CLAUDE_CONFIG_DIR が決まっていません。--config-dir <dir> で使うアカウントの設定ディレクトリを指定してください"
[ -d "${CONFIG_DIR}" ] || die "設定ディレクトリが見つかりません: ${CONFIG_DIR}"
# 相対パスで渡されても REPO_ROOT へ移動した後に解決がずれないよう、先に絶対パスにする。
CONFIG_DIR="$(cd "${CONFIG_DIR}" && pwd)"

command -v takt >/dev/null 2>&1 || die "takt が PATH にありません (mise の設定を確認してください)"

if [ "$#" -eq 0 ]; then
  set -- run
fi

unset CLAUDE_CODE_OAUTH_TOKEN
export CLAUDE_CONFIG_DIR="${CONFIG_DIR}"

cd "${REPO_ROOT}"
printf '==> CLAUDE_CONFIG_DIR=%s (CLAUDE_CODE_OAUTH_TOKEN は unset 済み)\n' "${CLAUDE_CONFIG_DIR}"
printf '==> takt %s\n' "$*"
exec takt "$@"
