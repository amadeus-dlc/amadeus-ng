"""別アカウントへの履歴引き継ぎを、実際のラッパーと偽の TAKT で検証する。"""

import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import unittest


SCRIPTS = Path(__file__).resolve().parents[1]


class RunTaktTest(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name).resolve()
        (self.root / "scripts").mkdir()
        for name in ("run-takt.sh", "takt-inherit.py"):
            if (SCRIPTS / name).exists():
                shutil.copyfile(SCRIPTS / name, self.root / "scripts" / name)
        self.source = self.root / "account A"
        self.target = self.root / "account B"
        self.source.mkdir()
        self.target.mkdir()
        self.worktree = self.root / "worktree"
        self.worktree.mkdir()
        self.project = Path("projects") / re.sub(r"[^a-zA-Z0-9]", "-", str(self.worktree))
        self.transcript = self.project / "session-1.jsonl"
        self.write(self.source, self.transcript, '{"slug":"example-plan"}\n')
        self.write(self.source, self.project / "session-1/subagents/agent-1.jsonl", '{}\n')
        self.write(self.source, self.project / "session-1/tool-results/result.txt", 'result')
        self.write(self.source, "file-history/session-1/snapshot", 'snapshot')
        self.write(self.source, "tasks/session-1/1.json", '{}')
        self.write(self.source, "plans/example-plan.md", 'plan')
        self.write(self.source, "projects/unrelated/private.jsonl", '{}\n')
        for directory, account in ((self.source, "A"), (self.target, "B")):
            self.write(directory, ".credentials.json", account)
            self.write(directory, "settings.json", account)
            self.write(directory, ".claude.json", json.dumps({
                "oauthAccount": account,
                "projects": {str(self.worktree): {"hasTrustDialogAccepted": True}}
                if account == "A" else {},
            }))
        self.calls = self.root / "calls.jsonl"
        binary = self.root / "bin/takt"
        binary.parent.mkdir()
        binary.write_text('''#!/usr/bin/env python3
import json, os, sys
with open(os.environ["TEST_CALLS"], "a") as stream:
    stream.write(json.dumps({"args": sys.argv[1:],
        "config": os.environ.get("CLAUDE_CONFIG_DIR"),
        "token": "CLAUDE_CODE_OAUTH_TOKEN" in os.environ,
        "inherited": os.path.exists(os.environ["TEST_TRANSCRIPT"])}) + "\\n")
if sys.argv[1:] == ["list", "--non-interactive", "--format", "json"]:
    print(os.environ["TEST_TASKS"])
    sys.exit(int(os.environ.get("TEST_LIST_EXIT", "0")))
sys.exit(int(os.environ.get("TEST_EXIT", "0")))
''')
        binary.chmod(0o755)
        self.env = {
            **os.environ, "PATH": str(binary.parent) + os.pathsep + os.environ["PATH"],
            "HOME": str(self.root / "home"),
            "CLAUDE_CONFIG_DIR": str(self.target), "CLAUDE_CODE_OAUTH_TOKEN": "test-token",
            "TEST_CALLS": str(self.calls), "TEST_TRANSCRIPT": str(self.target / self.transcript),
            "TEST_TASKS": json.dumps({"tasks": [{"kind": "failed", "worktreePath": str(self.worktree)}]}),
        }

    def write(self, root, path, text):
        file = root / path
        file.parent.mkdir(parents=True, exist_ok=True)
        file.write_text(text)

    def run_script(self, *args, **env):
        result = subprocess.run(
            ["/bin/bash", str(self.root / "scripts/run-takt.sh"), *args],
            cwd=self.root, env={**self.env, **env}, text=True, capture_output=True,
        )
        calls = [json.loads(line) for line in self.calls.read_text().splitlines()] \
            if self.calls.exists() else []
        return result, calls

    def inherit(self, *args, **env):
        return self.run_script("--inherit-from", "account A", *args, **env)

    def test_inherits_worktree_context_before_list_without_copying_auth(self):
        result, calls = self.inherit("list")
        self.assertEqual(result.returncode, 0, result.stderr)
        for path in (self.transcript, self.project / "session-1/subagents/agent-1.jsonl",
                     self.project / "session-1/tool-results/result.txt",
                     Path("file-history/session-1/snapshot"), Path("tasks/session-1/1.json"),
                     Path("plans/example-plan.md")):
            self.assertEqual((self.target / path).read_bytes(), (self.source / path).read_bytes())
        self.assertFalse((self.target / "projects/unrelated").exists())
        self.assertEqual((self.target / ".credentials.json").read_text(), "B")
        self.assertEqual((self.target / "settings.json").read_text(), "B")
        config = json.loads((self.target / ".claude.json").read_text())
        self.assertEqual(config["oauthAccount"], "B")
        self.assertTrue(config["projects"][str(self.worktree)]["hasTrustDialogAccepted"])
        self.assertEqual(calls[-1]["args"], ["list"])
        self.assertTrue(calls[-1]["inherited"])
        self.assertTrue(all(c["config"] == str(self.target) and not c["token"] for c in calls))

    def test_extends_old_transcript_and_is_repeatable(self):
        self.write(self.target, self.transcript, '')
        for _ in range(2):
            result, _ = self.inherit("run")
            self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual((self.target / self.transcript).read_bytes(), (self.source / self.transcript).read_bytes())

    def test_keeps_destination_transcript_if_it_has_progressed(self):
        progressed = (self.source / self.transcript).read_text() + '{"extra":true}\n'
        self.write(self.target, self.transcript, progressed)
        result, _ = self.inherit()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual((self.target / self.transcript).read_text(), progressed)

    def test_divergent_history_stops_before_any_copy_or_run(self):
        self.write(self.target, self.transcript, '{"different":true}\n')
        result, calls = self.inherit("run")
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(len(calls), 1)
        self.assertFalse((self.target / "file-history").exists())
        self.assertEqual((self.target / self.transcript).read_text(), '{"different":true}\n')

    def test_list_failure_does_not_start_takt(self):
        result, calls = self.inherit("run", TEST_LIST_EXIT="7")
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(len(calls), 1)
        self.assertFalse((self.target / self.transcript).exists())

    def test_running_task_blocks_copy(self):
        result, calls = self.inherit(TEST_TASKS=json.dumps({"tasks": [{"kind": "running"}]}))
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(len(calls), 1)

    def test_symlink_destination_does_not_overwrite_external_file(self):
        outside = self.root / "outside"
        outside.mkdir()
        (self.target / "projects").symlink_to(outside, target_is_directory=True)
        result, _ = self.inherit()
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(list(outside.iterdir()), [])

    def test_normal_run_and_exit_status_are_unchanged(self):
        result, calls = self.run_script("--no-inherit", TEST_EXIT="9")
        self.assertEqual(result.returncode, 9)
        self.assertEqual([c["args"] for c in calls], [["run"]])
        self.assertFalse(calls[0]["token"])

    def test_invalid_source_and_empty_value_do_not_call_takt(self):
        for value in ("missing", ""):
            result, calls = self.run_script("--inherit-from=" + value)
            self.assertEqual(result.returncode, 2)
            self.assertEqual(calls, [])

    def test_repository_context_is_inherited_without_worktrees(self):
        project = Path("projects") / re.sub(r"[^a-zA-Z0-9]", "-", str(self.root))
        self.write(self.source, project / "root-session.jsonl", '{}\n')
        result, _ = self.inherit(TEST_TASKS='{"tasks": []}')
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual((self.target / project / "root-session.jsonl").read_text(), '{}\n')
        self.assertFalse((self.target / self.transcript).exists())

    def test_target_trust_denial_and_other_project_settings_are_preserved(self):
        original = {"oauthAccount": "B", "projects": {
            str(self.worktree): {"hasTrustDialogAccepted": False, "other": 123},
            "/unrelated": {"hasTrustDialogAccepted": True},
        }}
        self.write(self.target, ".claude.json", json.dumps(original))
        result, _ = self.inherit()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads((self.target / ".claude.json").read_text()), original)

    def test_invalid_config_stops_before_copy(self):
        self.write(self.target, ".claude.json", "broken-json")
        result, calls = self.inherit()
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(len(calls), 1)
        self.assertFalse((self.target / self.transcript).exists())

    def test_trust_change_has_backup(self):
        original = (self.target / ".claude.json").read_bytes()
        result, _ = self.inherit()
        self.assertEqual(result.returncode, 0, result.stderr)
        backups = list(self.target.glob(".claude.json.before-takt-*"))
        self.assertEqual(len(backups), 1)
        self.assertEqual(backups[0].read_bytes(), original)

    def test_source_without_matching_history_stops(self):
        shutil.rmtree(self.source / "projects")
        result, calls = self.inherit()
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(len(calls), 1)

    def test_same_account_is_rejected(self):
        result, calls = self.run_script("--inherit-from", str(self.target))
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(calls, [])

    def test_command_arguments_are_preserved(self):
        result, calls = self.inherit("--", "--task", "task with spaces")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(calls[-1]["args"], ["--task", "task with spaces"])

    def test_auto_discovers_source_without_flag(self):
        result, calls = self.run_script("list")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue(calls[-1]["inherited"])
        self.assertEqual(calls[-1]["args"], ["list"])
        self.assertIn(str(self.source), result.stdout)
        self.assertEqual((self.target / ".credentials.json").read_text(), "B")

    def test_auto_merges_compatible_accounts_using_history_not_mtime(self):
        third = self.root / "account C"
        progressed = (self.source / self.transcript).read_text() + '{"new":true}\n'
        self.write(third, self.transcript, progressed)
        self.write(third, self.project / "session-2.jsonl", '{}\n')
        os.utime(third / self.transcript, (1, 1))
        result, _ = self.run_script()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual((self.target / self.transcript).read_text(), progressed)
        self.assertTrue((self.target / self.project / "session-2.jsonl").exists())

    def test_auto_conflicting_accounts_stop_before_copy(self):
        third = self.root / "account C"
        self.write(third, self.transcript, '{"different":true}\n')
        result, calls = self.run_script("run")
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(len(calls), 1)
        self.assertFalse((self.target / self.transcript).exists())
        self.assertIn("--inherit-from", result.stderr)

    def test_explicit_source_bypasses_auto_conflicts(self):
        self.write(self.root / "account C", self.transcript, '{"different":true}\n')
        result, _ = self.inherit("list")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual((self.target / self.transcript).read_bytes(), (self.source / self.transcript).read_bytes())

    def test_auto_without_source_runs_normally(self):
        shutil.rmtree(self.source / "projects")
        result, calls = self.run_script()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(calls[-1]["args"], ["run"])
        self.assertFalse((self.target / "projects").exists())

    def test_auto_discovers_home_accounts_outside_target_parent(self):
        source = Path(self.env["HOME"]) / ".claude-A"
        source.parent.mkdir()
        shutil.move(str(self.source), source)
        result, _ = self.run_script("list")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue((self.target / self.transcript).exists())

    def test_auto_deduplicates_aliases_and_excludes_target(self):
        (self.root / "source alias").symlink_to(self.source, target_is_directory=True)
        (self.root / "target alias").symlink_to(self.target, target_is_directory=True)
        result, _ = self.run_script("list")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout.count("引き継ぎ元: "), 1)

    def test_help_does_not_scan_or_copy(self):
        result, calls = self.run_script("--help")
        self.assertEqual(result.returncode, 0)
        self.assertEqual(calls, [])
        self.assertIn("自動", result.stdout)

    def test_unrelated_commands_do_not_scan(self):
        result, calls = self.run_script("--version")
        self.assertEqual(result.returncode, 0)
        self.assertEqual([call["args"] for call in calls], [["--version"]])

    def test_conflicting_inherit_options_are_rejected(self):
        for args in (("--no-inherit", "--inherit-from", str(self.source)),
                     ("--inherit-from", str(self.source), "--no-inherit")):
            result, calls = self.run_script(*args)
            self.assertEqual(result.returncode, 2)
            self.assertEqual(calls, [])

    def test_auto_preserves_account_memory_and_does_not_merge_indexes(self):
        third = self.root / "account C"
        for account, content in ((self.source, "A"), (self.target, "B"), (third, "C")):
            self.write(account, self.project / "memory/MEMORY.md", content)
            self.write(account, self.project / "sessions-index.json", content)
        self.write(third, self.project / "session-2.jsonl", '{}\n')
        result, _ = self.run_script("list")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual((self.target / self.project / "memory/MEMORY.md").read_text(), "B")
        self.assertEqual((self.target / self.project / "sessions-index.json").read_text(), "B")
        self.assertTrue((self.target / self.transcript).exists())

    def test_auto_source_symlink_is_rejected_before_copy(self):
        outside = self.root / "outside.jsonl"
        outside.write_text('{}\n')
        (self.source / self.project / "linked.jsonl").symlink_to(outside)
        result, calls = self.run_script("list")
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(len(calls), 1)
        self.assertFalse((self.target / self.transcript).exists())


if __name__ == "__main__":
    unittest.main()
