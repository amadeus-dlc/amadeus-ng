#!/usr/bin/env python3
"""1 つの側でケース列を走らせ、観測を <out>/<id>.result.json に残す。
usage: run_side.py <cases_dir> <out_dir> <record_dir> <audit_shard> <cwd> <env_json> -- <cmd...>
env_json: 実行環境 (完全に置き換える)。"""
import json, os, subprocess, sys, time, glob, shutil
args = sys.argv[1:]
sep = args.index("--")
cases_dir, out_dir, record, audit, cwd, env_json = args[:sep]
cmd = args[sep+1:]
base_env = json.loads(env_json)
os.makedirs(out_dir, exist_ok=True)
health = os.path.join(record, ".aidlc-hooks-health")
dispatch = os.path.join(record, ".aidlc-reviewer-dispatch.json")
def read(p):
    try:
        with open(p, encoding="utf-8", errors="replace") as f: return f.read()
    except FileNotFoundError: return None
def snapshot():
    s = {"audit": read(audit) or "", "drops": read(os.path.join(health, "reviewer-scope.drops")) or ""}
    return s
ids = sorted(os.path.basename(p)[:-5] for p in glob.glob(f"{cases_dir}/c*.json") if not p.endswith(".meta.json"))
for cid in ids:
    stdin = read(f"{cases_dir}/{cid}.json")
    meta = json.load(open(f"{cases_dir}/{cid}.meta.json"))
    # 前状態: 差し向け記録と目印を整える (drops / 監査は差分で見る)
    if os.path.exists(dispatch): os.remove(dispatch)
    for marker in ("reviewer-scope.last", "reviewer-scope.missing-record.last"):
        p = os.path.join(health, marker)
        if os.path.exists(p): os.remove(p)
    if meta.get("dispatch") is not None:
        with open(dispatch, "w") as f: f.write(json.dumps(meta["dispatch"]) + "\n")
        if meta.get("age_hours"):
            t = time.time() - meta["age_hours"] * 3600
            os.utime(dispatch, (t, t))
    env = dict(base_env); env.update(meta.get("env", {}))
    before = snapshot()
    started = time.time()
    proc = subprocess.run(cmd, input=stdin.encode(), cwd=cwd, env=env, capture_output=True)
    elapsed = time.time() - started
    after = snapshot()
    def delta(k):
        b, a = before[k], after[k]
        return a[len(b):] if a.startswith(b) else "<<REWRITTEN>>" + a
    result = {
        "id": cid, "exit": proc.returncode,
        "stdout": proc.stdout.decode("utf-8", "replace"), "stderr": proc.stderr.decode("utf-8", "replace"),
        "audit_delta": delta("audit"), "drops_delta": delta("drops"),
        "last_exists": os.path.exists(os.path.join(health, "reviewer-scope.last")),
        "last_content": read(os.path.join(health, "reviewer-scope.last")),
        "missing_marker_exists": os.path.exists(os.path.join(health, "reviewer-scope.missing-record.last")),
        "dispatch_exists_after": os.path.exists(dispatch),
        "elapsed_s": round(elapsed, 2),
    }
    with open(f"{out_dir}/{cid}.result.json", "w") as f: json.dump(result, f, indent=1, ensure_ascii=False)
    print(f"{cid}\texit={proc.returncode}\tstderr={proc.stderr.decode()[:60]!r}")
