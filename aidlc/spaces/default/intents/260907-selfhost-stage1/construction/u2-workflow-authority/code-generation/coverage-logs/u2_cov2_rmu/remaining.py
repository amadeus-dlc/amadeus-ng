import json,sys,os
sys.path.insert(0,os.path.dirname(os.path.abspath(__file__)))
from uncov_lib import uncovered, ranges
parent=json.load(open(sys.argv[1])); mine=json.load(open(sys.argv[2]))
pf={f['filename'].split('modules/')[-1]:set(uncovered(f)) for f in parent['data'][0]['files'] if 'read-model-updater/src' in f['filename']}
mf={f['filename'].split('modules/')[-1]:set(uncovered(f)) for f in mine['data'][0]['files'] if 'read-model-updater/src' in f['filename']}
tb=0; ta=0; rows=[]
for fn,p in pf.items():
    m=mf.get(fn,set()); rem=p & m
    tb+=len(p); ta+=len(rem)
    if p: rows.append((len(rem),len(p),fn,sorted(rem)))
rows.sort(reverse=True)
for r,pn,fn,lines in rows:
    if r or '-v' in sys.argv: print(f"{r:4d}/{pn:4d} {fn}\n      {ranges(lines)}")
print("TOTAL remaining",ta,"of",tb, f"({100*(1-ta/tb):.1f}% reduced)")
