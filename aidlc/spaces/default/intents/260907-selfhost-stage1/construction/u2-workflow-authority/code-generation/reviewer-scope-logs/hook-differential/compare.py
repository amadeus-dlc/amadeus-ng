#!/usr/bin/env python3
"""両側の観測を正規化して突き合わせる。
usage: compare.py <up_out> <rs_out> <up_project> <up_record_name> <rs_project> <rs_record_name> <report.tsv>"""
import json, re, sys, glob, os
up_out, rs_out, up_p, up_rec, rs_p, rs_rec, report = sys.argv[1:8]
TS = re.compile(r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d+)?Z")
def norm(text, project, rec):
    if text is None: return None
    text = text.replace(project, "<P>").replace(rec, "<REC>")
    return TS.sub("<TS>", text)
def load(d, project, rec):
    out = {}
    for p in sorted(glob.glob(f"{d}/*.result.json")):
        r = json.load(open(p))
        out[r["id"]] = {
            "exit": r["exit"],
            "stdout": norm(r["stdout"], project, rec),
            "stderr": norm(r["stderr"], project, rec),
            "audit_delta": norm(r["audit_delta"], project, rec),
            "drops_delta": norm(r["drops_delta"], project, rec),
            "last_exists": r["last_exists"],
            "missing_marker_exists": r["missing_marker_exists"],
            "dispatch_exists_after": r["dispatch_exists_after"],
        }
    return out
up = load(up_out, up_p, up_rec); rs = load(rs_out, rs_p, rs_rec)
rows = []; diffs = 0
for cid in sorted(set(up) | set(rs)):
    u, r = up.get(cid), rs.get(cid)
    fields = ["exit", "stdout", "stderr", "audit_delta", "drops_delta", "last_exists", "missing_marker_exists", "dispatch_exists_after"]
    bad = [f for f in fields if (u or {}).get(f) != (r or {}).get(f)]
    rows.append((cid, "SAME" if not bad else "DIFF", ",".join(bad), json.dumps({f: (u or {}).get(f) for f in bad}, ensure_ascii=False), json.dumps({f: (r or {}).get(f) for f in bad}, ensure_ascii=False)))
    diffs += bool(bad)
with open(report, "w") as f:
    f.write("case\tverdict\tdiffering_fields\tupstream\tthis_build\n")
    for row in rows: f.write("\t".join(row) + "\n")
print(f"{diffs} / {len(rows)} cases differ -> {report}")
for row in rows:
    if row[1] == "DIFF": print(row[0], row[2])
