#!/usr/bin/env python3
"""run-takt.sh 用。アカウント設定を保持したまま、対象プロジェクトの再開データを移す。

コピー中は両アカウントの対象セッションを停止しておくこと。
保存形式: https://code.claude.com/docs/en/claude-directory#application-data
"""

import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile


def check_path(root, path):
    """設定ディレクトリ内のリンクを辿って別のデータを書き換えない。"""
    current = root
    for part in path.relative_to(root).parts:
        current = current / part
        if current.is_symlink():
            raise ValueError(f"シンボリックリンクはコピーできません: {current}")


def collect_files(root, path):
    check_path(root, path)
    if not path.exists():
        return []
    if path.is_file():
        return [path]
    files = []
    for child in sorted(path.iterdir()):
        # タスクツールの実行中ロックは移さない。
        if child.name == ".lock":
            continue
        files.extend(collect_files(root, child))
    return files


def config_path(directory):
    # 既定の ~/.claude では、旧形式の ~/.claude.json も使われる。
    local = directory / ".claude.json"
    if not local.exists() and directory == (Path.home() / ".claude").resolve():
        return Path.home() / ".claude.json"
    return local


def load_config(path):
    if path.is_symlink():
        raise ValueError(f"設定ファイルがシンボリックリンクです: {path}")
    result = json.loads(path.read_text()) if path.exists() else {}
    if not isinstance(result, dict) or not isinstance(result.get("projects", {}), dict):
        raise ValueError(f"設定ファイルの形式が不正です: {path}")
    return result


def atomic_write(path, data):
    path.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
    fd, temporary = tempfile.mkstemp(prefix=".takt-inherit-", dir=path.parent)
    try:
        with os.fdopen(fd, "wb") as stream:
            stream.write(data)
        os.replace(temporary, path)
    finally:
        if os.path.exists(temporary):
            os.unlink(temporary)


def load_projects(repository):
    result = subprocess.run(
        ["takt", "list", "--non-interactive", "--format", "json"],
        cwd=repository, text=True, stdout=subprocess.PIPE, check=True,
    )
    tasks = json.loads(result.stdout)["tasks"]
    if not isinstance(tasks, list):
        raise ValueError("takt list の tasks が配列ではありません")
    if any(task.get("kind") == "running" for task in tasks):
        raise ValueError("実行中の TAKT タスクがあります。停止してから引き継いでください")
    projects = {str(repository)}
    for task in tasks:
        worktree = task.get("worktreePath")
        if worktree:
            path = Path(worktree)
            projects.add(str(path if path.is_absolute() else repository / path))
    return projects


def discover_sources(target):
    """既知のアカウント配置だけを浅く探索し、別名リンクの重複を除く。"""
    candidates = set(Path.home().glob(".claude*"))
    candidates.update(target.parent.iterdir())
    return sorted({path.resolve() for path in candidates
                   if path.is_dir() and (path / "projects").is_dir()
                   and path.resolve() != target})


def context_files(source, projects, *, automatic=False):
    files = set()
    sessions = set()
    plans = set()
    for project in sorted(projects):
        # Claude のプロジェクトディレクトリは絶対パスの非英数字を '-' に変換する。
        directory = source / "projects" / re.sub(r"[^a-zA-Z0-9]", "-", project)
        if automatic:
            # 自動メモリはアカウントごとに異なる。セッション一覧の索引も再開本体ではない。
            check_path(source, directory)
            project_files = []
            if directory.is_dir():
                for child in sorted(directory.iterdir()):
                    if child.name not in ("memory", "sessions-index.json"):
                        project_files.extend(collect_files(source, child))
        else:
            project_files = collect_files(source, directory)
        files.update(project_files)
        for file in project_files:
            if file.suffix != ".jsonl":
                continue
            sessions.add(file.stem)
            with file.open() as stream:
                for line in stream:
                    try:
                        entry = json.loads(line)
                    except json.JSONDecodeError:
                        # 中断時の末尾の不完全な行は、そのままコピーして Claude に委ねる。
                        continue
                    if not isinstance(entry, dict):
                        continue
                    slug = entry.get("slug")
                    if isinstance(slug, str) and re.fullmatch(r"[a-zA-Z0-9_-]+", slug):
                        plans.add(slug)

    for session in sessions:
        for category in ("file-history", "tasks", "image-cache", "uploads"):
            files.update(collect_files(source, source / category / session))
        for todo in (source / "todos").glob(f"{session}-*.json"):
            files.update(collect_files(source, todo))
    for plan in plans:
        files.update(collect_files(source, source / "plans" / f"{plan}.md"))
    return files


def choose_file(previous, candidate, destination):
    """同一セッションは追記の関係にある場合だけ長い履歴を選ぶ。"""
    old, new = previous.read_bytes(), candidate.read_bytes()
    if old == new:
        return previous
    if candidate.suffix == ".jsonl":
        if old.startswith(new):
            return previous
        if new.startswith(old):
            return candidate
    raise ValueError(
        f"内容が分岐しています。上書きしません: {destination}\n"
        f"  候補: {previous}\n  候補: {candidate}\n"
        "引き継ぎ元を --inherit-from <dir> で指定してください。"
        "引き継ぎ先自体に分岐がある場合は、先に履歴を確認してください"
    )


def inherit(source, target, repository):
    if source == target:
        raise ValueError("引き継ぎ元と引き継ぎ先が同じです")
    projects = load_projects(repository)
    candidates = [source] if source is not None else discover_sources(target)
    sources = []
    selected = {}
    for candidate in candidates:
        files = context_files(candidate, projects, automatic=source is None)
        if not files:
            continue
        sources.append(candidate)
        for file in sorted(files):
            relative = file.relative_to(candidate)
            previous = selected.get(relative)
            selected[relative] = file if previous is None else choose_file(previous, file, relative)
    if not selected:
        if source is not None:
            raise ValueError("引き継ぎ元に対象プロジェクトの履歴がありません")
        print("==> 自動探索: 対象プロジェクトの引き継ぎ元はありません。通常どおり起動します")
        return

    # 全ファイルを事前検証し、分岐や形式エラーがあればコピー前に止める。
    copies = []
    for relative, file in sorted(selected.items()):
        destination = target / relative
        check_path(target, destination)
        if destination.exists():
            if choose_file(destination, file, destination) == destination:
                continue
        copies.append((file, destination))

    target_config_path = config_path(target)
    target_config = load_config(target_config_path)
    target_projects = target_config.setdefault("projects", {})
    trust_changed = False
    for candidate in sources:
        source_config = load_config(config_path(candidate))
        for project in projects:
            previous = source_config.get("projects", {}).get(project, {})
            current = target_projects.get(project, {})
            if not isinstance(previous, dict) or not isinstance(current, dict):
                raise ValueError("Claude のプロジェクト設定の形式が不正です")
            # A で承認済みの同じパスだけを引き継ぐ。B の明示的な false は尊重する。
            if previous.get("hasTrustDialogAccepted") is True and "hasTrustDialogAccepted" not in current:
                target_projects[project] = {**current, "hasTrustDialogAccepted": True}
                trust_changed = True

    for candidate in sources:
        print(f"==> 引き継ぎ元: {candidate}")
    for file, destination in copies:
        atomic_write(destination, file.read_bytes())
    if trust_changed:
        if target_config_path.exists():
            fd, backup = tempfile.mkstemp(prefix=".claude.json.before-takt-", dir=target_config_path.parent)
            os.close(fd)
            shutil.copyfile(target_config_path, backup)
        atomic_write(target_config_path, (json.dumps(target_config, ensure_ascii=False, indent=2) + "\n").encode())
    print(f"==> 再開用データを {len(copies)} ファイル引き継ぎました (認証情報・アカウント設定は保持)")
    if trust_changed:
        print("==> 引き継ぎ元で承認済みのプロジェクト信頼設定を追加しました")


if __name__ == "__main__":
    try:
        if len(sys.argv) != 4:
            raise ValueError("run-takt.sh --inherit-from <dir> から呼び出してください")
        source = None if sys.argv[1] == "--auto" else Path(sys.argv[1]).resolve()
        inherit(source, Path(sys.argv[2]).resolve(), Path(sys.argv[3]).resolve())
    except (OSError, ValueError, KeyError, TypeError, subprocess.CalledProcessError) as error:
        print(f"error: 引き継ぎを中止しました: {error}", file=sys.stderr)
        sys.exit(2)
