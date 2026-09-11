def uncovered(f):
    segs=f['segments']
    if not segs: return []
    maxl=max(s[0] for s in segs)
    by={}
    for s in segs: by.setdefault(s[0],[]).append(s)
    res=[]; wrapped=None
    for L in range(1,maxl+1):
        ls=by.get(L,[])
        starts=[s for s in ls if (not s[5]) and s[3] and s[4]]
        skipped= bool(ls) and (not ls[0][3]) and ls[0][4]
        mapped = (not skipped) and ((wrapped is not None and wrapped[3]) or len(starts)>0)
        if mapped:
            cnt = wrapped[2] if wrapped is not None else 0
            for s in starts: cnt=max(cnt,s[2])
            if cnt==0: res.append(L)
        if ls: wrapped=ls[-1]
    return res
def ranges(lines):
    out=[];st=None;pv=None
    for L in lines:
        if st is None: st=pv=L
        elif L==pv+1: pv=L
        else: out.append(f"{st}-{pv}" if st!=pv else str(st)); st=pv=L
    if st is not None: out.append(f"{st}-{pv}" if st!=pv else str(st))
    return ",".join(out)
