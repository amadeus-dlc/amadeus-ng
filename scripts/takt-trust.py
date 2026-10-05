#!/usr/bin/env python3
"""run-takt.sh --trust-workspace 用。takt を起動するアカウントで、この checkout を信頼済みにする。

takt が起動する Claude は、信頼されていない作業ディレクトリでは .claude/settings.json の
permissions.allow を無視して失敗する。Orca などで新しく作った worktree は、どのアカウントでも
まだ承認されていないので、起動前に hasTrustDialogAccepted を立てる。明示的な false は上書きしない。
"""

import importlib.util
import json
import os
from pathlib import Path
import shutil
import sys
import tempfile


def load_inherit():
    """設定の読み書きは takt-inherit.py と同じ関数を使う（シンボリックリンクの拒否・原子的な置換）。"""
    spec = importlib.util.spec_from_file_location(
        "takt_inherit", Path(__file__).resolve().parent / "takt-inherit.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def trust(target, workspace):
    inherit = load_inherit()
    config_path = inherit.config_path(target)
    config = inherit.load_config(config_path)
    projects = config.setdefault("projects", {})
    current = projects.get(str(workspace), {})
    if not isinstance(current, dict):
        raise ValueError("Claude のプロジェクト設定の形式が不正です")
    accepted = current.get("hasTrustDialogAccepted")
    if accepted is True:
        print(f"==> 信頼設定: {workspace} は承認済みです")
        return
    if accepted is not None:
        raise ValueError(f"{workspace} の信頼設定は明示的に拒否されています。上書きしません")
    projects[str(workspace)] = {**current, "hasTrustDialogAccepted": True}
    if config_path.exists():
        fd, backup = tempfile.mkstemp(prefix=".claude.json.before-takt-", dir=config_path.parent)
        os.close(fd)
        shutil.copyfile(config_path, backup)
    inherit.atomic_write(config_path, (json.dumps(config, ensure_ascii=False, indent=2) + "\n").encode())
    print(f"==> 信頼設定: {workspace} を承認済みにしました (変更前の設定はバックアップ済み)")


if __name__ == "__main__":
    try:
        if len(sys.argv) != 3:
            raise ValueError("run-takt.sh --trust-workspace から呼び出してください")
        trust(Path(sys.argv[1]).resolve(), Path(sys.argv[2]).resolve())
    except (OSError, ValueError, KeyError, TypeError) as error:
        print(f"error: 信頼設定を中止しました: {error}", file=sys.stderr)
        sys.exit(2)
