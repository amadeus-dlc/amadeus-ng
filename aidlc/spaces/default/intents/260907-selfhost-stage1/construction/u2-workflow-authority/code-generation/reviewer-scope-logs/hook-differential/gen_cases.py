#!/usr/bin/env python3
"""両側で同じケース列を、その側の絶対パスで生成する。
usage: gen_cases.py <project_dir> <record_dir> <out_dir>
各ケースは <out_dir>/<id>.json (フックの標準入力) と <id>.meta.json (前状態・環境) を持つ。"""
import json, sys, os
P, REC, OUT = sys.argv[1], sys.argv[2], sys.argv[3]
os.makedirs(OUT, exist_ok=True)
REVIEWER = "aidlc-architecture-reviewer-agent"
SIB = f"{REC}/construction/u2-y/contract.md"
CUR = f"{REC}/construction/u1-x/functional-design/entities.md"
EXEMPT = f"{REC}/construction/u2-y/functional-design/entities.md"
GOOD = {"reviewer": REVIEWER, "stage": "functional-design", "unit": "u1-x", "exempt": [EXEMPT, "construction/u3-z/contract.md"]}
def inp(tool, ti, agent=REVIEWER, cwd=P, extra=None):
    d = {"hook_event_name": "PreToolUse", "tool_name": tool, "tool_input": ti}
    if agent is not None: d["agent_type"] = agent
    if cwd is not None: d["cwd"] = cwd
    if extra: d.update(extra)
    return json.dumps(d)
cases = [
 ("c01-read-sibling-block", inp("Read", {"file_path": SIB}), {"dispatch": GOOD}),
 ("c02-read-current-allow", inp("Read", {"file_path": CUR}), {"dispatch": GOOD}),
 ("c03-read-exempt-allow", inp("Read", {"file_path": EXEMPT}), {"dispatch": GOOD}),
 ("c04-ls-exempt-dir-block", inp("LS", {"path": f"{REC}/construction/u2-y/functional-design"}), {"dispatch": GOOD}),
 ("c05-grep-pathless-block", inp("Grep", {"pattern": "anything"}), {"dispatch": GOOD}),
 ("c06-glob-wildcard-block", inp("Glob", {"pattern": "construction/*/design.md"}), {"dispatch": GOOD}),
 ("c07-glob-current-allow", inp("Glob", {"pattern": "construction/u1-x/**/*.md"}), {"dispatch": GOOD}),
 ("c08-bash-cat-sibling-block", inp("Bash", {"command": f"cat {SIB}"}), {"dispatch": GOOD}),
 ("c09-bash-rg-dot-block", inp("Bash", {"command": "rg foo ."}), {"dispatch": GOOD}),
 ("c10-write-sibling-block", inp("Write", {"file_path": f"{REC}/construction/u2-y/out.md", "content": "x"}), {"dispatch": GOOD}),
 ("c11-other-agent-allow", inp("Read", {"file_path": SIB}, agent="aidlc-developer-agent"), {"dispatch": GOOD}),
 ("c12-no-agent-allow", inp("Read", {"file_path": SIB}, agent=None), {"dispatch": GOOD}),
 ("c13-scoped-registration-block", inp("Read", {"file_path": SIB}, agent=None, extra={"scoped_registration": True}), {"dispatch": GOOD}),
 ("c14-non-inspected-tool-allow", inp("Task", {"file_path": SIB, "prompt": "x"}), {"dispatch": GOOD}),
 ("c15-missing-record-advisory", inp("Read", {"file_path": CUR}), {"dispatch": None}),
 ("c16-missing-record-nonreviewer", inp("Read", {"file_path": CUR}, agent="aidlc-developer-agent"), {"dispatch": None}),
 ("c17-malformed-record", inp("Read", {"file_path": SIB}), {"dispatch": {"reviewer": REVIEWER, "stage": "functional-design", "unit": "u1-x"}}),
 ("c18-orphaned-record", inp("Read", {"file_path": SIB}), {"dispatch": GOOD, "age_hours": 7}),
 ("c19-off-switch", inp("Read", {"file_path": SIB}), {"dispatch": GOOD, "env": {"AIDLC_DISABLE_REVIEWER_SCOPE_HOOK": "1"}}),
 ("c20-malformed-stdin", "not json", {"dispatch": GOOD}),
 ("c21-array-stdin", "[]", {"dispatch": GOOD}),
 ("c22-nocwd-relative-grep-path", inp("Grep", {"pattern": "x", "path": "aidlc"}, cwd=None), {"dispatch": GOOD}),
 ("c23-cwd-relative-grep-path", inp("Grep", {"pattern": "x", "path": "aidlc"}), {"dispatch": GOOD}),
 ("c24-stage-nonslug-record", inp("Read", {"file_path": SIB}), {"dispatch": dict(GOOD, stage="Functional Design")}),
 ("c25-unit-with-slash-record", inp("Read", {"file_path": SIB}), {"dispatch": dict(GOOD, unit="a/b")}),
 ("c26-read-relative-sibling-block", inp("Read", {"file_path": "construction/u2-y/contract.md"}), {"dispatch": GOOD}),
 ("c27-notebookread-sibling-block", inp("NotebookRead", {"notebook_path": f"{REC}/construction/u2-y/nb.ipynb"}), {"dispatch": GOOD}),
 ("c28-read-relative-exempt-allow", inp("Read", {"file_path": f"{REC}/construction/u3-z/contract.md"}), {"dispatch": GOOD}),
 ("c29-ls-relative-exempt-parent-block", inp("LS", {"path": f"{REC}/construction/u3-z"}), {"dispatch": GOOD}),
 ("c30-bash-cd-then-cat-block", inp("Bash", {"command": f"cd {REC}/construction && cat u2-y/contract.md"}), {"dispatch": GOOD}),
 ("c31-empty-stdin", "", {"dispatch": GOOD}),
 ("c32-nocwd-read-sibling-block", inp("Read", {"file_path": SIB}, cwd=None), {"dispatch": GOOD}),
]
for cid, stdin, meta in cases:
    with open(f"{OUT}/{cid}.json", "w") as f: f.write(stdin)
    with open(f"{OUT}/{cid}.meta.json", "w") as f: json.dump(meta, f)
print(len(cases))
