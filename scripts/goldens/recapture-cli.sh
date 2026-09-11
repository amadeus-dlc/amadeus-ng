#!/usr/bin/env bash
# CLI 主要遷移とフック代表ケースの実行出力ゴールデンの再採取 (FR7.2 / BR2.1 / BR2.4 / BR2.5)。
#
# upstream ピン `a277af21` の配布シェル `dist/claude/` を使い捨てディレクトリへ取得し、
# ツリー全体の sha256 マニフェストを期待値と照合してから、**そのピンのコードを bun で
# 実行して** 出力を採る (インストール済みの別バージョンのシェルは使わない)。
#
#   bash scripts/goldens/recapture-cli.sh
#
# 取得は SHA 指定の shallow fetch を第一手段とし、失敗したら codeload の tarball 取得へ
# フォールバックする (どちらもピン留めコミットのバイト列を取る。差はプロトコルだけで、
# 取得物はマニフェスト sha256 で同一性を機械照合する)。
#
# ゴールデンの更新は **upstream ピン更新の intent でのみ** 行う (BR2.5)。ピンが変わらない
# 限り、このスクリプトの再実行は `captured_at` 以外に差分を出してはならない。

set -euo pipefail

readonly UPSTREAM_REPO="https://github.com/awslabs/aidlc-workflows"
readonly UPSTREAM_COMMIT="a277af218f0df7f325d3b8be7b6d90fce2c5bd40"
readonly UPSTREAM_VERSION="2.7.1"
readonly DIST_PATH="dist/claude"
readonly TARBALL_URL="https://codeload.github.com/awslabs/aidlc-workflows/tar.gz/${UPSTREAM_COMMIT}"

# ピン留めコミットにおける実測値。`dist/claude/` 配下の全ファイルを
# `<sha256>  <dist/claude からの相対パス>` の行にし、パスで LC_ALL=C ソートしたテキストの
# sha256。ずれたら upstream 側が動いたということなので停止する。
readonly EXPECTED_FILE_COUNT="277"
readonly EXPECTED_MANIFEST_SHA256="282b17c53cb82c24755b149f28e01508ce9eaebbdbe3020034a4dd4c8bda4459"

readonly SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
readonly OUT_DIR="${1:-${REPO_ROOT}/tests/golden/upstream-a277af21}"
[[ ! -e "${OUT_DIR}" ]] || { echo "error: 採取先は新規ディレクトリを指定してください" >&2; exit 1; }
readonly COMMAND="bash scripts/goldens/recapture-cli.sh"

need() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "error: '$1' が必要です (PATH に見つかりません)" >&2
    exit 1
  }
}
need curl
need bun
need git
need shasum
need tar

workdir="$(mktemp -d)"
trap 'rm -rf "${workdir}"' EXIT

fetch_method=""

fetch_via_git() {
  local src="${workdir}/src"
  mkdir -p "${src}"
  (
    cd "${src}"
    git init -q .
    git remote add origin "${UPSTREAM_REPO}"
    git fetch --depth 1 -q origin "${UPSTREAM_COMMIT}"
    git checkout -q FETCH_HEAD
  ) || return 1
  [[ -d "${src}/${DIST_PATH}" ]] || return 1
  cp -R "${src}/${DIST_PATH}" "${workdir}/dist-claude"
  fetch_method="git fetch --depth 1 origin ${UPSTREAM_COMMIT} && git checkout FETCH_HEAD"
  return 0
}

fetch_via_tarball() {
  local tgz="${workdir}/pin.tar.gz"
  local extract="${workdir}/extract"
  curl -fsSL -o "${tgz}" "${TARBALL_URL}" || return 1
  mkdir -p "${extract}"
  tar -xzf "${tgz}" -C "${extract}" || return 1
  # 展開先は `aidlc-workflows-<sha>/`。1 つだけのはずだが、明示的に絞る。
  local root
  root="$(find "${extract}" -mindepth 1 -maxdepth 1 -type d | head -n 1)"
  [[ -n "${root}" && -d "${root}/${DIST_PATH}" ]] || return 1
  cp -R "${root}/${DIST_PATH}" "${workdir}/dist-claude"
  fetch_method="curl ${TARBALL_URL} | tar -xz (git fetch フォールバック)"
  return 0
}

echo "==> upstream ${UPSTREAM_COMMIT} の ${DIST_PATH} を取得"
if [[ -n "${2:-}" ]]; then
  cp -R "$2" "${workdir}/dist-claude"
  fetch_method="verified local distribution"
elif fetch_via_git; then
  echo "    取得方法: SHA 指定の shallow fetch"
elif fetch_via_tarball; then
  echo "    取得方法: codeload tarball (shallow fetch 失敗のためフォールバック)"
else
  echo "error: upstream ピンを取得できませんでした (git fetch と tarball の両方が失敗)" >&2
  exit 1
fi

echo "==> 取得ツリーの sha256 マニフェストを照合"
(
  cd "${workdir}/dist-claude"
  find . -type f | sed 's|^\./||' | LC_ALL=C sort | while read -r f; do
    printf '%s  %s\n' "$(shasum -a 256 "${f}" | cut -d' ' -f1)" "${f}"
  done
) >"${workdir}/manifest.txt"

actual_file_count="$(wc -l <"${workdir}/manifest.txt" | tr -d ' ')"
actual_manifest_sha256="$(shasum -a 256 "${workdir}/manifest.txt" | cut -d' ' -f1)"

if [[ "${actual_file_count}" != "${EXPECTED_FILE_COUNT}" ]]; then
  echo "error: 取得ツリーのファイル数が期待値と一致しません" >&2
  echo "  expected: ${EXPECTED_FILE_COUNT}" >&2
  echo "  actual  : ${actual_file_count}" >&2
  exit 1
fi
if [[ "${actual_manifest_sha256}" != "${EXPECTED_MANIFEST_SHA256}" ]]; then
  echo "error: 取得ツリーのマニフェスト sha256 が期待値と一致しません" >&2
  echo "  expected: ${EXPECTED_MANIFEST_SHA256}" >&2
  echo "  actual  : ${actual_manifest_sha256}" >&2
  exit 1
fi
echo "    sha256 一致: ${actual_manifest_sha256} (${actual_file_count} ファイル)"

bun_version="$(bun --version)"

cat >"${workdir}/meta.json" <<JSON
{
  "upstream_repo": "${UPSTREAM_REPO}",
  "upstream_commit": "${UPSTREAM_COMMIT}",
  "upstream_version": "${UPSTREAM_VERSION}",
  "source_path": "${DIST_PATH}",
  "fetch_method": "${fetch_method}",
  "tree_manifest_sha256": "${EXPECTED_MANIFEST_SHA256}",
  "tree_file_count": ${EXPECTED_FILE_COUNT},
  "command": "${COMMAND}",
  "bun_version": "${bun_version}"
}
JSON

bun -e 'const fs=require("node:fs"); const [path,...args]=process.argv.slice(1); const meta=JSON.parse(fs.readFileSync(path,"utf8")); meta.capture_argv=["bash","scripts/goldens/recapture-cli.sh",...args.filter(Boolean)]; fs.writeFileSync(path,JSON.stringify(meta,null,2)+"\n");' "${workdir}/meta.json" "${OUT_DIR}" "${2:-}"

mkdir -p "${workdir}/corpus"
cp "${SCRIPT_DIR}/normalization.json" "${workdir}/corpus/normalization.json"
cp "${workdir}/manifest.txt" "${workdir}/corpus/source-manifest.sha256"
cp "${workdir}/meta.json" "${workdir}/corpus/source.json"
cp -R "${workdir}/dist-claude/.claude/tools/data" "${workdir}/corpus/data"
echo "==> bun ${bun_version} で採取 -> ${OUT_DIR}/{cli,hooks}"
bun "${SCRIPT_DIR}/capture-cli.ts" \
  "${workdir}/dist-claude" \
  "${workdir}/corpus" \
  "${workdir}/meta.json"

bun "${SCRIPT_DIR}/capture-supplemental.ts" "${workdir}/dist-claude" "${workdir}/corpus/supplemental"
bash "${SCRIPT_DIR}/recapture-hash-canonical.sh" "${workdir}/corpus/hash-canonical" "${workdir}/dist-claude"
bun "${SCRIPT_DIR}/capture-stage1.ts" "${workdir}/dist-claude" "${workdir}/corpus/stage1"
bun "${SCRIPT_DIR}/capture-extensions.ts" "${workdir}/dist-claude" "${workdir}/corpus"
bun "${SCRIPT_DIR}/prepare-corpus.ts" "${workdir}/corpus"
bun "${SCRIPT_DIR}/verify-corpus.ts" "${workdir}/corpus"
mkdir -p "$(dirname "${OUT_DIR}")"
mv "${workdir}/corpus" "${OUT_DIR}"
echo "==> 完了"
