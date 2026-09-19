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
#   scripts/run-takt.sh [--config-dir <dir>] [--inherit-from <dir> | --no-inherit] [--] [takt の引数...]
#
#   --config-dir を省略したときは、呼び出し元の CLAUDE_CONFIG_DIR を使う。どちらも無いときは
#   既定の ~/.claude がどのアカウントかを確かめずに走らせることになるので、エラーで止める。
#   takt の引数を省略したときは `takt run` を実行する。
#
# 例:
#   scripts/run-takt.sh --config-dir ~/.claude-<account>        # takt run
#   scripts/run-takt.sh --config-dir ~/.claude-<account> list   # takt list
#   scripts/run-takt.sh --config-dir ~/.claude-B --inherit-from ~/.claude-A list
#   scripts/run-takt.sh --config-dir ~/.claude-B run
#
# list/run/resume の前に別アカウントの再開用データを自動探索してコピーする。
# --inherit-from で引き継ぎ元を固定し、--no-inherit でコピーを無効にできる。
# list で Requeue を選ぶ手順は従来どおり。コピー中は両アカウントの対象作業を停止する。
#
# bash 3.2 (macOS 標準) 互換のため、配列は使用しない。
#
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

usage() {
  cat <<'EOF'
usage: scripts/run-takt.sh [--config-dir <dir>] [--inherit-from <dir> | --no-inherit] [--] [takt の引数...]

  --config-dir <dir>  使う Claude アカウントの設定ディレクトリ (省略時は呼び出し元の CLAUDE_CONFIG_DIR)
  --inherit-from <dir> 切り替え前の Claude 設定ディレクトリから再開用データをコピー (Python 3 が必要)
  --no-inherit        自動探索・コピーを無効にする
  -h, --help          この説明を表示する

takt の引数を省略したときは `takt run` を実行する。
list/run/resume の前に ~/.claude* と指定アカウントの兄弟ディレクトリから履歴を自動探索する。
自動探索にも Python 3 が必要。候補がなければ通常どおり起動する。

アカウントを切り替えて再開する例:
  scripts/run-takt.sh --config-dir ~/.claude-B list
  # 失敗タスクの Requeue と保存済みの停止位置を選ぶ
  scripts/run-takt.sh --config-dir ~/.claude-B run

対象はこのリポジトリと `takt list` に記録された worktree の履歴・関連データ。
認証情報とアカウント設定はコピーしない。既存履歴が分岐している場合はエラーで停止する。
複数候補でも履歴が追記関係なら長い方を採用する。分岐時は --inherit-from <dir> で指定する。
自動探索ではプロジェクトの memory/ と sessions-index.json はコピーしない。
同じ作業パスの承認済み信頼設定のみ追加する (変更前の .claude.json はバックアップ)。
コピー中は両アカウントの対象作業を停止しておくこと。
EOF
}

die() {
  printf 'error: %s\n' "$1" >&2
  exit 2
}

CONFIG_DIR="${CLAUDE_CONFIG_DIR:-}"
INHERIT_FROM=""
NO_INHERIT=0
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
    --inherit-from)
      [ "$#" -ge 2 ] || die "--inherit-from には値が必要です"
      [ -n "$2" ] || die "--inherit-from には値が必要です"
      INHERIT_FROM="$2"
      shift 2
      ;;
    --inherit-from=*)
      INHERIT_FROM="${1#--inherit-from=}"
      [ -n "${INHERIT_FROM}" ] || die "--inherit-from には値が必要です"
      shift
      ;;
    --no-inherit)
      NO_INHERIT=1
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

if [ "${NO_INHERIT}" -eq 1 ] && [ -n "${INHERIT_FROM}" ]; then
  die "--no-inherit と --inherit-from は同時に指定できません"
fi

[ -n "${CONFIG_DIR}" ] \
  || die "CLAUDE_CONFIG_DIR が決まっていません。--config-dir <dir> で使うアカウントの設定ディレクトリを指定してください"
[ -d "${CONFIG_DIR}" ] || die "設定ディレクトリが見つかりません: ${CONFIG_DIR}"
# 相対パスで渡されても REPO_ROOT へ移動した後に解決がずれないよう、先に絶対パスにする。
CONFIG_DIR="$(cd "${CONFIG_DIR}" && pwd)"
if [ -n "${INHERIT_FROM}" ]; then
  [ -d "${INHERIT_FROM}" ] || die "引き継ぎ元が見つかりません: ${INHERIT_FROM}"
  INHERIT_FROM="$(cd "${INHERIT_FROM}" && pwd)"
fi

command -v takt >/dev/null 2>&1 || die "takt が PATH にありません (mise の設定を確認してください)"

if [ "$#" -eq 0 ]; then
  set -- run
fi

if [ -z "${INHERIT_FROM}" ] && [ "${NO_INHERIT}" -eq 0 ]; then
  case "$1" in
    list|run|resume) INHERIT_FROM="--auto" ;;
  esac
fi
if [ -n "${INHERIT_FROM}" ]; then
  command -v python3 >/dev/null 2>&1 || die "履歴の引き継ぎには Python 3 が必要です (--no-inherit で無効化できます)"
fi

unset CLAUDE_CODE_OAUTH_TOKEN
export CLAUDE_CONFIG_DIR="${CONFIG_DIR}"

cd "${REPO_ROOT}"
if [ -n "${INHERIT_FROM}" ]; then
  python3 "${SCRIPT_DIR}/takt-inherit.py" "${INHERIT_FROM}" "${CONFIG_DIR}" "${REPO_ROOT}"
fi
printf '==> CLAUDE_CONFIG_DIR=%s (CLAUDE_CODE_OAUTH_TOKEN は unset 済み)\n' "${CLAUDE_CONFIG_DIR}"
printf '==> takt %s\n' "$*"
exec takt "$@"
