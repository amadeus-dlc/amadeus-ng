"""llvm-cov summary の行数を再現: 関数ごとに行統計を取り、同一定義位置の実体化グループでは covered/total の最大を取り、合算する。
未カバーの候補行は「グループ内で最も covered の多い実体化が落とした行」として列挙する。"""
import json,sys,collections
sys.path.insert(0,'/private/tmp/claude-501/-Users-j5ik2o-orca-workspaces-amadeus-ng-stage1/9d1d85aa-6e19-407b-ba66-8f78afcb9e92/scratchpad')
def fn_segments(fn, fi):
    regs=[r for r in fn['regions'] if r[5]==fi]
    code=[r for r in regs if r[7]==0]
    gaps=[r for r in regs if r[7]==3]
    skip=[r for r in regs if r[7]==2]
    if not code: return {}
    lines={}
    lo=min(r[0] for r in code); hi=max(r[2] for r in code)
    def contains(r,L):
        return r[0]<=L<=r[2]
    for L in range(lo,hi+1):
        if any(contains(s,L) for s in skip) and not any(r[0]==L for r in code+gaps):
            continue
        starts=[r for r in code if r[0]==L]
        wrap=[r for r in code if r[0]<L and r[2]>=L]
        if not starts and not wrap:
            continue
        c=0
        if wrap:
            inner=min(wrap,key=lambda r:((r[2]-r[0]),-r[1]))
            c=inner[4]
        for r in starts: c=max(c,r[4])
        lines[L]=c
    return lines
def summarize(d,pred):
    groups=collections.defaultdict(list)
    for fn in d['data'][0]['functions']:
        for i,f in enumerate(fn['filenames']):
            if not pred(f): continue
            regs=[r for r in fn['regions'] if r[5]==i and r[7]==0]
            if not regs: continue
            st=min((r[0],r[1]) for r in regs)
            groups[(f,st)].append(fn_segments(fn,i))
    per=collections.defaultdict(lambda:[0,0,[]])
    for (f,st),insts in groups.items():
        best=max(insts,key=lambda l:sum(1 for c in l.values() if c>0))
        cov=sum(1 for c in best.values() if c>0); tot=len(best)
        per[f][0]+=cov; per[f][1]+=tot
        union=set()
        for l in insts:
            union|={L for L,c in l.items() if c>0}
        z=[L for L,c in best.items() if c==0]
        if z: per[f][2].append((st,sorted(z),sorted(L for L in z if L in union),len(insts)))
    return per
if __name__=='__main__':
    d=json.load(open(sys.argv[1]))
    per=summarize(d,lambda f:'read-model-updater/src' in f)
    tc=tt=0; rows=[]
    for f,(c,t,z) in per.items():
        tc+=c; tt+=t; rows.append((t-c,f,z))
    rows.sort(reverse=True)
    for u,f,z in rows:
        if u==0 and '-v' not in sys.argv: continue
        print(u, f.split('read-model-updater/')[-1])
        for st,L,u,n in sorted(z): print(f"    fn@{st[0]}:{st[1]} x{n} -> {L}" + (f"  [elsewhere: {u}]" if u else ""))
    print('TOTAL', tt, tc, tt-tc)
