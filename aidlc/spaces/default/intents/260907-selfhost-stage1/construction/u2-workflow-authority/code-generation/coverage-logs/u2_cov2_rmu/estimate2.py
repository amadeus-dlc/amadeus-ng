import json,sys,collections,re
sys.path.insert(0,'/private/tmp/claude-501/-Users-j5ik2o-orca-workspaces-amadeus-ng-stage1/9d1d85aa-6e19-407b-ba66-8f78afcb9e92/scratchpad')
from fsum2 import fn_segments
MY_TESTS=['journal_reader_impl_test','publication_recovery_contract','read_model_updater_test','runtime_graph_projection_contract','plan_approval_projection_contract','hook_health_projection_contract','workflow_continuation_projection_contract','journal_corruption_contract','projection_rows_contract','execution_event_dto_contract','read_tables_test','cross_shard_read_test','audit_block_golden_test','plan_source_contract','plan_receipt_projection','projection_golden_test','publication_file_contract','report_result_projection_contract','source_baseline_publication_contract']
HASH=re.compile(r'(Cs[0-9A-Za-z]+)_23core_read_model_updater')
def shift(path,f,L):
    # 親 JSON (s9b) 以後に projection.rs から 2 行が消えた (1247〜1250 付近)。行番号を現物へ写す。
    if 'coverage-head-per-file' in path and f.endswith('workspace/projection.rs') and L>=1251:
        return L-2
    return L
def load(path):
    d=json.load(open(path))
    fns=[]
    hashes=collections.Counter(); lib_votes=collections.Counter()
    for fn in d['data'][0]['functions']:
        n=fn['name']
        m=HASH.search(n)
        if not m: continue
        h=m.group(1); hashes[h]+=1
        if any(t in n for t in MY_TESTS): lib_votes[h]+=1
    lib=lib_votes.most_common(1)[0][0]
    unit=[h for h in hashes if h!=lib]
    unit=max(unit,key=lambda h:hashes[h]) if unit else None
    groups=collections.defaultdict(lambda:{'lib':[], 'unit':[], 'mytest':[], 'app':[]})
    for fn in d['data'][0]['functions']:
        n=fn['name']
        m=HASH.search(n)
        if not m: continue
        for i,f in enumerate(fn['filenames']):
            if 'read-model-updater/src' not in f: continue
            regs=[r for r in fn['regions'] if r[5]==i and r[7]==0]
            if not regs: continue
            st=min((r[0],r[1]) for r in regs)
            st=(shift(path,f,st[0]),st[1])
            key=(f.split('read-model-updater/')[-1],st)
            seg={shift(path,f,L):c for L,c in fn_segments(fn,i).items()}
            h=m.group(1)
            if any(t in n for t in MY_TESTS): kind='mytest'
            elif h==lib and n.count('Cs')==1: kind='lib'
            elif h==unit and n.count('Cs')==1: kind='unit'
            elif h==lib: kind='app'
            else: kind='unit'
            groups[key][kind].append(seg)
    return groups
def merge(a,b):
    out=dict(a)
    for L,c in b.items(): out[L]=max(out.get(L,0),c)
    return out
ws=load(sys.argv[1]); mine=load(sys.argv[2])
tb=ta=0; per=collections.defaultdict(lambda:[0,0,[]])
for key,g in ws.items():
    allws=g['lib']+g['unit']+g['mytest']+g['app']
    best=max(allws,key=lambda l:sum(1 for c in l.values() if c>0))
    before=sum(1 for c in best.values() if c==0)
    m=mine.get(key,{'lib':[],'unit':[],'mytest':[],'app':[]})
    cands=list(g['app'])+list(m['unit'])+list(m['mytest'])
    if g['lib'] or m['lib']:
        l=g['lib'][0] if g['lib'] else {}
        for x in m['lib']: l=merge(l,x)
        cands.append(l)
    if not cands: cands=allws
    b2=max(cands,key=lambda l:sum(1 for c in l.values() if c>0))
    after=sum(1 for c in b2.values() if c==0)
    tb+=before; ta+=after
    per[key[0]][0]+=before; per[key[0]][1]+=after
    if after: per[key[0]][2].append((key[1],sorted(L for L,c in b2.items() if c==0)))
print('estimated ws uncovered: before',tb,'after',ta)
for f,(b,a,z) in sorted(per.items(), key=lambda kv:-kv[1][1]):
    if a==0 and b==0: continue
    print(f"{b:4d} -> {a:4d}  {f}")
    if '-v' in sys.argv:
        for st,L in sorted(z): print(f"        fn@{st[0]}:{st[1]} -> {L}")
