"""takt が起動した Claude に .takt/ を読ませないフックを、settings.json の登録ごと検証する。"""

import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest


SCRIPTS = Path(__file__).resolve().parents[1]
SETTINGS = SCRIPTS.parent / ".claude/settings.json"
CURRENT = "20260928-100000-current"
OLD = "20260927-100000-old"


def guard_command():
    settings = json.loads(SETTINGS.read_text())
    commands = [hook["command"] for entry in settings["hooks"]["PreToolUse"]
                for hook in entry["hooks"] if "takt-read-guard.py" in hook["command"]]
    assert len(commands) == 1, commands
    return commands[0]


class TaktReadGuardTest(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name).resolve()
        self.project = self.root / "project"
        (self.project / "scripts").mkdir(parents=True)
        shutil.copyfile(SCRIPTS / "takt-read-guard.py", self.project / "scripts/takt-read-guard.py")
        for slug, status in ((CURRENT, "running"), (OLD, "completed")):
            run = self.project / ".takt/runs" / slug
            self.write(run / "meta.json", json.dumps({"status": status}))
            self.write(run / "context/task/order.md", "order")
            self.write(run / "reports/plan.md", "plan")
            self.write(run / "logs/log.jsonl", "{}")
        self.write(self.project / ".takt/config.yaml", "language: ja")
        self.write(self.project / ".takt/tasks/task-1/order.md", "order")
        self.write(self.root / "home/.takt/config.yaml", "language: ja")

    def write(self, path, text):
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text)

    def run_hook(self, tool, arguments, guard="1"):
        env = {**os.environ, "CLAUDE_PROJECT_DIR": str(self.project), "HOME": str(self.root / "home")}
        env.pop("AMADEUS_TAKT_GUARD", None)
        if guard is not None:
            env["AMADEUS_TAKT_GUARD"] = guard
        event = {"tool_name": tool, "tool_input": arguments, "cwd": str(self.project)}
        return subprocess.run(["/bin/bash", "-c", guard_command()], cwd=self.project, env=env,
                              input=json.dumps(event), text=True, capture_output=True)

    def assert_allowed(self, tool, arguments, **options):
        result = self.run_hook(tool, arguments, **options)
        self.assertEqual(result.returncode, 0, result.stderr)

    def assert_blocked(self, tool, arguments):
        result = self.run_hook(tool, arguments)
        self.assertEqual(result.returncode, 2, (tool, arguments))
        self.assertIn(f".takt/runs/{CURRENT}/context/", result.stderr)
        self.assertNotIn(OLD, result.stderr.split("（", 1)[1])

    def test_current_run_context_and_reports_are_readable(self):
        for path in (f".takt/runs/{CURRENT}/context/task/order.md",
                     f".takt/runs/{CURRENT}/reports/plan.md",
                     f".takt/runs/{CURRENT}/reports/sub/.takt-report-internal/history/a.md",
                     str(self.project / f".takt/runs/{CURRENT}/context/task")):
            self.assert_allowed("Read", {"file_path": path})
        self.assert_allowed("Bash", {"command": f"cat .takt/runs/{CURRENT}/reports/plan.md"})
        self.assert_allowed("Grep", {"pattern": "x", "path": f".takt/runs/{CURRENT}/reports"})
        self.assert_allowed("Glob", {"pattern": "**/*.md", "path": f".takt/runs/{CURRENT}/context"})

    def test_everything_else_under_takt_is_blocked(self):
        for path in (f".takt/runs/{OLD}/reports/plan.md",
                     f".takt/runs/{OLD}/context/task/order.md",
                     f".takt/runs/{CURRENT}/logs/log.jsonl",
                     f".takt/runs/{CURRENT}/meta.json",
                     f".takt/runs/{CURRENT}/reports/../../{OLD}/reports/plan.md",
                     ".takt/tasks/task-1/order.md",
                     ".takt/config.yaml",
                     ".takt",
                     str(self.project / ".takt/tasks.yaml"),
                     "~/.takt/config.yaml"):
            self.assert_blocked("Read", {"file_path": path})

    def test_bash_commands_naming_takt_paths_are_checked(self):
        for command in ("ls .takt/runs",
                        "cat .takt/runs/*/reports/plan.md",
                        f"cat .takt/runs/{CURRENT}/reports/plan.md .takt/runs/{OLD}/reports/plan.md",
                        "git show HEAD:.takt/config.yaml",
                        'rg --files "$ROOT/.takt/runs"',
                        "cd .takt && cat tasks.yaml",
                        "find . -path ./.takt/runs/{a,b}"):
            self.assert_blocked("Bash", {"command": command})
        for command in ("cargo test", "rg foo src", "cat foo.takt/x", "ls .takt-report-internal"):
            self.assert_allowed("Bash", {"command": command})

    def test_search_tools_are_checked_by_path_and_glob(self):
        self.assert_blocked("Grep", {"pattern": "x", "path": ".takt"})
        self.assert_blocked("Grep", {"pattern": "x", "glob": ".takt/runs/**"})
        self.assert_blocked("Glob", {"pattern": ".takt/**/*.md"})
        self.assert_blocked("Glob", {"pattern": "**/*.md", "path": str(self.project / ".takt/runs")})
        self.assert_allowed("Grep", {"pattern": r"\.takt", "path": "src"})
        self.assert_allowed("Glob", {"pattern": "**/*.rs"})

    def test_inactive_without_takt_guard_variable(self):
        for guard in (None, ""):
            self.assert_allowed("Read", {"file_path": ".takt/config.yaml"}, guard=guard)

    def test_broken_meta_does_not_unlock_a_run(self):
        self.write(self.project / f".takt/runs/{OLD}/meta.json", "{broken")
        self.assert_blocked("Read", {"file_path": f".takt/runs/{OLD}/reports/plan.md"})

    def test_missing_script_or_malformed_input_never_blocks(self):
        env = {**os.environ, "AMADEUS_TAKT_GUARD": "1", "CLAUDE_PROJECT_DIR": str(self.project)}
        result = subprocess.run(["python3", str(self.project / "scripts/takt-read-guard.py")],
                                env=env, input="not json", text=True, capture_output=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        (self.project / "scripts/takt-read-guard.py").unlink()
        self.assert_allowed("Read", {"file_path": ".takt/config.yaml"})


if __name__ == "__main__":
    unittest.main()
