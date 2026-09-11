#!/usr/bin/env bash
# hash-canonical 受入表の再採取 (FR7.1 / BR2.1 / BR2.5)。
#
# upstream ピン `a277af21` の `dist/claude/.claude/tools/aidlc-testing-posture.ts` を
# 取得し、`canonicalize` / `sha256` / `hashObject` の 3 関数をスニペットとして抽出、
# sha256 で照合してから bun で実行し、期待値を採る。
#
#   bash scripts/goldens/recapture-hash-canonical.sh
#
# ゴールデンの更新は **upstream ピン更新の intent でのみ** 行う (BR2.5)。ピンが変わらない
# 限り、このスクリプトの再実行は `captured_at` 以外に差分を出してはならない。

set -euo pipefail

readonly UPSTREAM_REPO="https://github.com/awslabs/aidlc-workflows"
readonly UPSTREAM_COMMIT="a277af218f0df7f325d3b8be7b6d90fce2c5bd40"
readonly UPSTREAM_VERSION="2.7.1"
readonly SOURCE_PATH="dist/claude/.claude/tools/aidlc-testing-posture.ts"
readonly SOURCE_URL="https://raw.githubusercontent.com/awslabs/aidlc-workflows/${UPSTREAM_COMMIT}/${SOURCE_PATH}"

# ピン留めコミットにおける実測値。どちらかがずれたら upstream 側が動いたということなので停止する。
readonly EXPECTED_SOURCE_SHA256="c06fb41743d9c10c5d50f88530096db8a1a5e5c8f61468855cfbdab39ee2c4f0"
readonly SNIPPET_LINES="160-179"
readonly EXPECTED_SNIPPET_SHA256="c8894a433d620538e1701f178b8542528603f012b98680b6b79233f70704418f"

readonly SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
readonly OUT_DIR="${1:-${REPO_ROOT}/tests/golden/upstream-a277af21/hash-canonical}"
[[ ! -e "${OUT_DIR}" ]] || { echo "error: 採取先は新規ディレクトリを指定してください" >&2; exit 1; }
readonly COMMAND="bash scripts/goldens/recapture-hash-canonical.sh"

need() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "error: '$1' が必要です (PATH に見つかりません)" >&2
    exit 1
  }
}
need curl
need bun
need shasum

sha256_of() { shasum -a 256 "$1" | cut -d' ' -f1; }

workdir="$(mktemp -d)"
trap 'rm -rf "${workdir}"' EXIT

echo "==> upstream ${UPSTREAM_COMMIT} から ${SOURCE_PATH} を取得"
if [[ -n "${2:-}" ]]; then
  cp "$2/.claude/tools/aidlc-testing-posture.ts" "${workdir}/aidlc-testing-posture.ts"
else
  curl -fsSL -o "${workdir}/aidlc-testing-posture.ts" "${SOURCE_URL}"
fi

actual_source_sha256="$(sha256_of "${workdir}/aidlc-testing-posture.ts")"
if [[ "${actual_source_sha256}" != "${EXPECTED_SOURCE_SHA256}" ]]; then
  echo "error: 取得ファイルの sha256 が期待値と一致しません" >&2
  echo "  expected: ${EXPECTED_SOURCE_SHA256}" >&2
  echo "  actual  : ${actual_source_sha256}" >&2
  exit 1
fi
echo "    sha256 一致: ${actual_source_sha256}"

echo "==> canonicalize / sha256 / hashObject を ${SNIPPET_LINES} 行目から抽出"
sed -n "${SNIPPET_LINES/-/,}p" "${workdir}/aidlc-testing-posture.ts" >"${workdir}/snippet.ts"

actual_snippet_sha256="$(sha256_of "${workdir}/snippet.ts")"
if [[ "${actual_snippet_sha256}" != "${EXPECTED_SNIPPET_SHA256}" ]]; then
  echo "error: 抽出スニペットの sha256 が期待値と一致しません (行番号がずれた可能性)" >&2
  echo "  expected: ${EXPECTED_SNIPPET_SHA256}" >&2
  echo "  actual  : ${actual_snippet_sha256}" >&2
  exit 1
fi
echo "    sha256 一致: ${actual_snippet_sha256}"

# import 可能なモジュールへ仕立てる (関数本体は 1 バイトも変えない — 前置と export だけ)。
{
  echo 'import { createHash } from "node:crypto";'
  echo ''
  sed 's/^function /export function /' "${workdir}/snippet.ts"
} >"${workdir}/upstream-snippet.ts"

bun_version="$(bun --version)"

cat >"${workdir}/meta.json" <<JSON
{
  "upstream_repo": "${UPSTREAM_REPO}",
  "upstream_commit": "${UPSTREAM_COMMIT}",
  "upstream_version": "${UPSTREAM_VERSION}",
  "source_path": "${SOURCE_PATH}",
  "source_url": "${SOURCE_URL}",
  "source_file_sha256": "${EXPECTED_SOURCE_SHA256}",
  "snippet_lines": "${SNIPPET_LINES}",
  "snippet_sha256": "${EXPECTED_SNIPPET_SHA256}",
  "command": "${COMMAND}",
  "bun_version": "${bun_version}"
}
JSON

echo "==> bun ${bun_version} で採取 -> ${OUT_DIR}"
bun "${SCRIPT_DIR}/capture-hash-canonical.ts" \
  "${workdir}/upstream-snippet.ts" \
  "${OUT_DIR}" \
  "${workdir}/meta.json"

cp "${workdir}/snippet.ts" "${OUT_DIR}/source-snippet.ts"
echo "==> 完了"
