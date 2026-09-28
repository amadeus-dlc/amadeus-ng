#!/usr/bin/env python3
"""takt が起動したエージェントに、.takt/ の中身をソースコードとして読ませない。

Claude Code の PreToolUse フック。scripts/run-takt.sh から起動した takt の Claude にだけ、
scripts/takt-claude.sh が --settings で登録する。

.takt/ は takt の作業領域で、過去のランの記録・ほかのタスクの指示書・takt の設定が
入っている。読んでよいのは今のラン (meta.json が running) の context/ と reports/ だけで、
takt は指示書・ポリシー・前のステップの結果をそこに置いてエージェントに渡している。

拒否は終了コード 2 と標準エラーの理由で返す。入力が読めないときは止めない (0 で抜ける)。
`grep -r foo .` のように .takt を名指ししない再帰読み取りは見分けられないので止めない。
"""

import json
import os
from pathlib import Path
import re
import sys


TAKT_DIR = ".takt"
READABLE = ("context", "reports")
# シェルの区切りとして扱う文字。`HEAD:.takt/x` や `--file=.takt/x` も拾う。
BASH_SEPARATORS = re.compile(r"[\s'\"`;|&<>(){}=:,]+")


def running_slugs(takt_dir):
    runs = takt_dir / "runs"
    if not runs.is_dir():
        return []
    slugs = []
    for meta in sorted(runs.glob("*/meta.json")):
        try:
            if json.loads(meta.read_text()).get("status") == "running":
                slugs.append(meta.parent.name)
        except (OSError, ValueError, AttributeError):
            continue
    return slugs


def blocked(path, cwd):
    """path が .takt/ の中で、今のランの context/ と reports/ の外なら True。"""
    path = os.path.expanduser(path)
    parts = Path(os.path.normpath(os.path.join(cwd, path))).parts
    if TAKT_DIR not in parts:
        return False
    index = parts.index(TAKT_DIR)
    takt_dir = Path(*parts[:index + 1])
    if not takt_dir.is_dir():
        # `$ROOT/.takt/...` のように展開できない前置きは、作業ディレクトリの .takt とみなす。
        takt_dir = Path(cwd) / TAKT_DIR
    rest = parts[index + 1:]
    return not (len(rest) >= 3 and rest[0] == "runs" and rest[2] in READABLE
                and rest[1] in running_slugs(takt_dir))


def joined(base, pattern):
    return pattern if not base else os.path.join(base, pattern)


def candidates(tool, arguments):
    """ツール入力のうち、読み取り先を表す値を返す。"""
    if tool == "Read":
        return [arguments.get("file_path")]
    if tool == "NotebookRead":
        return [arguments.get("notebook_path")]
    if tool == "LS":
        return [arguments.get("path")]
    if tool == "Glob":
        base = arguments.get("path")
        return [base, joined(base, arguments.get("pattern") or "")]
    if tool == "Grep":
        base = arguments.get("path")
        return [base] + ([joined(base, arguments["glob"])] if arguments.get("glob") else [])
    if tool == "Bash":
        return [token for token in BASH_SEPARATORS.split(arguments.get("command") or "")
                if TAKT_DIR in token.split("/")]
    return []


def main():
    try:
        event = json.load(sys.stdin)
        tool = event["tool_name"]
        arguments = event.get("tool_input") or {}
        cwd = event.get("cwd") or os.getcwd()
    except (ValueError, KeyError, TypeError):
        return 0
    for path in candidates(tool, arguments):
        if isinstance(path, str) and path and blocked(path, cwd):
            slugs = running_slugs(Path(cwd) / TAKT_DIR)
            readable = "、".join(f"{TAKT_DIR}/runs/{slug}/{name}/"
                                for slug in slugs for name in READABLE) or "なし"
            print(f"takt の実行中は {TAKT_DIR}/ の中を読めません: {path}\n"
                  f"{TAKT_DIR}/ は takt の作業領域で、ソースコードではありません。"
                  f"読んでよいのは今のランの context/ と reports/ だけです（{readable}）。",
                  file=sys.stderr)
            return 2
    return 0


if __name__ == "__main__":
    sys.exit(main())
