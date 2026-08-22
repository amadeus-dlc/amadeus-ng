#!/usr/bin/env bash
# scripts/governance/toolchain-inputs.sh — rust-toolchain.toml を正本として CI の toolchain
# アクション入力 (channel / components) を導出する (NFR4.2 — U10 CI ガバナンス)。
#
# dtolnay/rust-toolchain@master は `toolchain:` 入力が必須で rust-toolchain.toml を自動では
# 読まない (PR #25 の初回 CI で実測: "'toolchain' is a required input")。バージョンと
# コンポーネントを ci.yml に書き写すと正本が 2 つになるため、このスクリプトでファイルから
# 読み取り `$GITHUB_OUTPUT` 形式 (key=value) で印字する。
#
# 使い方: bash scripts/governance/toolchain-inputs.sh [rust-toolchain.toml のパス]
# 出力:   channel=1.95.0
#         components=rustfmt,clippy,llvm-tools
set -euo pipefail

file="${1:-rust-toolchain.toml}"
[[ -f "${file}" ]] || { echo "error: ${file} が見つかりません" >&2; exit 1; }

channel="$(sed -nE 's/^[[:space:]]*channel[[:space:]]*=[[:space:]]*"([^"]+)".*/\1/p' "${file}" | head -n1)"
[[ -n "${channel}" ]] || { echo "error: ${file} に channel = \"...\" が無い" >&2; exit 1; }

components_raw="$(sed -nE 's/^[[:space:]]*components[[:space:]]*=[[:space:]]*\[([^]]*)\].*/\1/p' "${file}" | head -n1)"
components="$(printf '%s' "${components_raw}" | tr -d '" ' | tr ',' '\n' | sed '/^$/d' | paste -sd, -)"

printf 'channel=%s\n' "${channel}"
printf 'components=%s\n' "${components}"
