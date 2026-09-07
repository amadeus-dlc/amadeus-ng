# unit-test-instructions — U9 正本・仕様の canon 追従（`u9-canon-docs`）

> Code Generation（Construction 3.5）の検査手順（Unit: U9、kind: spec）。**改訂履歴**: 初版 2026-08-23（B4、受入 6 項目）→ **再走 2026-09-07（本版、Modify）**:
> `../nfr-design/security-design.md` §4 の受入検査 10 項目に合わせて更新し、コマンドはすべて fenced bash へ移した（B4 の pending-revision 項目 2）。
> 出典: `code-generation-plan.md` §5、`../nfr-design/security-design.md` §4、`../functional-design/gap-measurement-20260907.md`（基線値）。
>
> 本 Unit はコードを持たないため「ユニットテスト」= 文書の受入検査。すべて本 Unit の対象ファイルに限定したコマンドで、リポジトリルートで実行する。
> `cargo test` は実行しない（コード diff ゼロなので既存スイートは `origin/main` の緑のまま。CI 7 ジョブが PR で再実行する）。
> 基線はいずれも 2026-09-07 の実測（改訂前 = 赤）。合格 = すべて期待を満たす。

## 1. 受入検査

| # | 検査 | 期待（緑） | 基線（赤、2026-09-07） | 要求 |
|---|---|---|---|---|
| 1 | コード変更ゼロ | 出力が空。`docs/specs/research` も空 | — | NFR2.1 / NFR1.1 |
| 2 | sentinel 10 語 grep（履歴マーカー除外） | 0 件 | 44 件（coding-rules 12 + CONSISTENCY-AUDIT 4（範囲外化）+ 仕様 4 号 28） | NFR2.2 |
| 3 | README の無矛盾 | 規則ファイル 22 = 表の規則行 22、索引ずれ 2 点が解消（:10 の告知行が節外へ、「13 本」→ 22） | 22 = 22 だが索引ずれ 2 点あり | NFR2.3 |
| 4 | 表の列数・見出し重複 | `tables ok`、重複見出し出力なし | — | NFR2.4 |
| 5 | 逸脱登録の維持 | `docs/specs/deviations.md` の diff が空 | 空 | NFR1.2 |
| 6 | `## Review` 節の履歴保全 | 3 本とも diff が空。`decisions.md` の diff は ADR-010 追記のみ | :430 / :486 / :207 | NFR1.5 |
| 7 | 実装状態の表記 | 4 号の各号に `予定（未実装` があり、§4.7 の項目（unpark / jump / recompose、フック 4、doctor、workspace 集約 3 と供給面 4、`intents.json`、Bolt / SwarmBatch）が予定表記 | 0 件 | NFR1.4 |
| 8 | 実測表との突合 | gap-measurement §2.1〜§2.6 の『処置』が全行反映、『維持』行が不変（diff の全件目視） | — | BR5.1 (e) |
| 9 | 用語 | RMU 呼出の文脈で「同期」が 0 件 | 0 件（components.md:30 の集約の純粋性は対象外） | 追加実測 1 |
| 10 | レビュー | CodeRabbit 未解決スレッド 0・全件返信、ステージレビュー READY、CI 7 ジョブ緑（PR 作成後） | — | NFR2.4 |

### (1) コード変更ゼロ

```bash
git diff --stat origin/main..HEAD -- modules tools scripts .github Cargo.toml Cargo.lock formal
git diff --stat origin/main..HEAD -- docs/specs/research
```

### (2) sentinel 10 語 grep

範囲 = `coding-rules/*.md`（`CONSISTENCY-AUDIT-*.md` を除く）+ `docs/specs/*.md`（`research/` は glob に入らない）。同一行に `~~` / 旧 / 失効 / 是正済み / 改名 / 履歴 の
いずれかがある行は履歴として除外。`StageGraphReader` は BR5.1 (c) で sentinel 外。

```bash
grep -rnE 'effective_plan_action|next_in_scope_stage|AuditLedgerRepository|AuditLedgerService|StateFileStore|report_forward|gate_start|WorkflowExecution|RehydratedWorkflowExecution|message-catalog' \
  $(ls aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/*.md | grep -v CONSISTENCY-AUDIT) docs/specs/*.md \
  | grep -vE '~~|旧|失効|是正済み|改名|履歴' | wc -l
```

（件数が 0 でないときは `| wc -l` を外して残存行を列挙し、code-summary に根拠を書く。）

### (3) README の無矛盾

```bash
ls aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/*.md | grep -vE 'README|good-examples|CONSISTENCY-AUDIT' | wc -l
grep -cE '^\| \[' aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/README.md
grep -nE '規則が [0-9]+ 本|^2026-[0-9-]+追加:' aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/README.md
```

期待: 1 行目 = 22、2 行目 = 23（good-examples 行 1 を含む → 規則行 22）、3 行目は「規則が 22 本」の 1 件のみで、`2026-09-06追加:` の告知行は出ない。
各行の一言・機械強制が本文と一致することは diff レビューで目視。

### (4) 表の列数・見出し重複

```bash
python3 - <<'EOF'
import re
files = [
 "aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/README.md",
 "aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/error-handling.md",
 "aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/factory-naming.md",
 "aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/gateway-taxonomy.md",
 "aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/module-visibility.md",
 "aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/use-case-rules.md",
 "aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/command-query-separation.md",
 "aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/interior-mutability.md",
 "aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/field-visibility.md",
 "docs/specs/01-domain-model.md","docs/specs/10-orchestration.md","docs/specs/11-workspace.md","docs/specs/12-workflow-definition.md",
 "aidlc/spaces/default/intents/260822-stage1-selfhost/inception/domain-design/components.md",
 "aidlc/spaces/default/intents/260822-stage1-selfhost/inception/contract-design/contract-summary.md",
 "aidlc/spaces/default/intents/260822-stage1-selfhost/inception/units-generation/unit-of-work.md",
 "aidlc/spaces/default/intents/260822-stage1-selfhost/inception/domain-design/decisions.md",
]
def cells(line):
    s = line.strip()
    if s.startswith('|'): s = s[1:]
    if s.endswith('|') and not s.endswith('\\|'): s = s[:-1]
    return len(re.split(r'(?<!\\)\|', s))
bad = 0
for f in files:
    lines = open(f, encoding='utf-8').read().split('\n')
    hdr = None; infence = False
    for i, l in enumerate(lines, 1):
        if l.strip().startswith('```'): infence = not infence; continue
        if infence: continue
        if l.strip().startswith('|'):
            n = cells(l)
            if hdr is None: hdr = n
            elif n != hdr: print(f"mismatch {f}:{i} cells={n} header={hdr}"); bad += 1
        else: hdr = None
    heads = [l for l in lines if l.startswith('#')]
    for h in sorted(set(heads)):
        if heads.count(h) > 1 and h != '## Review': print(f"dup-heading {f}: {h!r} x{heads.count(h)}"); bad += 1
print("tables ok" if bad == 0 else f"{bad} problem(s)")
EOF
```

### (5) 逸脱登録の維持

```bash
git diff --stat origin/main..HEAD -- docs/specs/deviations.md
```

### (6) `## Review` 節の履歴保全

```bash
for f in aidlc/spaces/default/intents/260822-stage1-selfhost/inception/domain-design/components.md \
         aidlc/spaces/default/intents/260822-stage1-selfhost/inception/contract-design/contract-summary.md \
         aidlc/spaces/default/intents/260822-stage1-selfhost/inception/units-generation/unit-of-work.md; do
  diff <(git show origin/main:"$f" | sed -n '/^## Review$/,$p') <(sed -n '/^## Review$/,$p' "$f") && echo "review-intact $f"
done
git diff origin/main..HEAD -- aidlc/spaces/default/intents/260822-stage1-selfhost/inception/domain-design/decisions.md | grep -E '^[-+]' | grep -vE '^(\+\+\+|---)'
```

期待: `review-intact` が 3 行。`decisions.md` の diff 行は ADR-010 の当該段落（打消し線 + 失効注記）だけで、削除行（`-`）が無い。

### (7) 実装状態の表記

```bash
grep -cE '予定（未実装' docs/specs/01-domain-model.md docs/specs/10-orchestration.md docs/specs/11-workspace.md docs/specs/12-workflow-definition.md
grep -nE 'unpark|recompose|doctor|WorktreeService|OpaqueFlagStore|ScopedStorage|SessionStampStore|intents\.json|SwarmBatch' docs/specs/01-domain-model.md docs/specs/10-orchestration.md docs/specs/11-workspace.md docs/specs/12-workflow-definition.md | grep -vE '予定|~~|旧|失効|履歴'
```

期待: 1 つ目は各号 1 以上。2 つ目（未実装項目の言及で予定表記も履歴マーカーも無い行）は、逐語契約の引用（CLI 動詞の列挙など、実装状態を主張しない行）以外は 0 件。
残る行は code-summary に理由を書く。

### (8) 実測表との突合

`git diff origin/main..HEAD -- <対象ファイル>` を gap-measurement §2.1〜§2.6 の表と行ごとに突合する（メインの目視、根拠列は `developer-report-3.md` / `-4.md`）。

### (9) 用語

```bash
grep -nE '同期' docs/specs/01-domain-model.md docs/specs/10-orchestration.md docs/specs/11-workspace.md docs/specs/12-workflow-definition.md \
  aidlc/spaces/default/intents/260822-stage1-selfhost/inception/domain-design/components.md \
  aidlc/spaces/default/intents/260822-stage1-selfhost/inception/contract-design/contract-summary.md | grep -iE 'rmu|catch_up|投影'
```

期待: 0 件。

### (10) レビュー（PR 作成後）

```bash
gh pr view <n> --json statusCheckRollup --jq '.statusCheckRollup[] | "\(.name) \(.conclusion)"'
gh api graphql -f query='query($o:String!,$r:String!,$n:Int!){repository(owner:$o,name:$r){pullRequest(number:$n){reviewThreads(first:100){nodes{isResolved isOutdated}}}}}' -F o=<owner> -F r=<repo> -F n=<n> --jq '[.data.repository.pullRequest.reviewThreads.nodes[] | select(.isResolved==false and .isOutdated==false)] | length'
```

期待: CI 7 ジョブ（aidlc-distribution / check / quint / coverage / audit / review-thread-resolution / ci-success）がすべて SUCCESS、未解決スレッド 0。

## 2. 期待カバレッジ・モック・テストデータ

- カバレッジ目標なし（コード変更ゼロ。`scripts/coverage.sh` は PR の CI で走り、基線維持を確認する）。モック・テストデータなし。
- (1)〜(9) は PR 作成前にローカルで、(10) は PR 作成後に実行する。結果は `code-summary.md` と PR 本文に貼る。派遣は (2) と (4) を所有ファイルの範囲で
  自己実行し、`developer-report-<n>.md` に貼る（メインの再実測と突合）。
