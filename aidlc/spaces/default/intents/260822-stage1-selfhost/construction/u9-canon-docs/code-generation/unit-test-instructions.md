# unit-test-instructions — U9 正本・仕様の canon 追従（`u9-canon-docs`）

> Code Generation（Construction 3.5）の検査手順（Unit: U9、kind: spec、Bolt: B4）。出典: `code-generation-plan.md` §5、`../nfr-design/security-design.md`
> §3（受入チェックリスト）。本 Unit はコードを持たないため、「ユニットテスト」= 文書の受入検査。すべて本 Unit の対象ファイルに限定したコマンド
> （`cargo test` は実行しない — コード変更ゼロなので既存スイートは `origin/main` の緑のまま）。

## 1. 受入検査（すべてリポジトリルートで実行、期待値を満たせば合格）

| # | 検査 | コマンド | 期待 |
|---|---|---|---|
| 1 | コード変更ゼロ | `git diff --stat origin/main..HEAD -- modules tools scripts .github Cargo.toml Cargo.lock` | 出力が空 |
| 1b | 抽出文書の不変 | `git diff --stat origin/main..HEAD -- docs/specs/research` | 出力が空 |
| 2 | sentinel grep | `grep -rnE 'effective_plan_action\|next_in_scope_stage\|AuditLedgerRepository\|AuditLedgerService\|StateFileStore\|report_forward\|gate_start' aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/*.md docs/specs/*.md` | 残るヒットが履歴注記（「旧」明記の比較表の行）だけ。各ヒットの根拠を code-summary に列挙 |
| 3 | README の無矛盾 | `ls aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/*.md \| grep -v README \| wc -l` と `grep -c '^\| \[' aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/README.md` | 両方 7。各行の一言・機械強制がファイル本文と一致（目視） |
| 4 | 表の列数 | 下記 §2 のスクリプト（改訂した全ファイル） | `mismatch` 出力なし |
| 4b | 見出し重複 | `for f in <改訂ファイル>; do grep -n '^#' "$f" \| sed 's/^[0-9]*://' \| sort \| uniq -d; done` | 出力が空 |
| 5 | 逸脱登録 | `grep -n '^| 4 |' docs/specs/deviations.md` | 1 行あり、理由欄に ADR-001 / 003 / 004 / 007 |
| 6 | レビューボット | `gh api repos/{owner}/{repo}/pulls/<n>/comments --paginate --jq length` と GraphQL の `reviewThreads(isResolved:false)` | 未解決スレッド 0（PR 作成後） |

## 2. 表の列数検査スクリプト

```bash
python3 - <<'EOF'
import re,sys,glob
files = [
 "aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/use-case-rules.md",
 "aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/gateway-taxonomy.md",
 "aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/error-handling.md",
 "aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/README.md",
 "docs/specs/01-domain-model.md","docs/specs/10-orchestration.md","docs/specs/11-workspace.md",
 "docs/specs/12-workflow-definition.md","docs/specs/deviations.md",
 "aidlc/spaces/default/intents/260822-stage1-selfhost/inception/domain-design/components.md",
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
print("tables ok" if bad == 0 else f"{bad} mismatch")
EOF
```

## 3. 期待カバレッジ・モック・テストデータ

- カバレッジ目標なし（コード変更ゼロ、`scripts/coverage.sh` は PR の CI で走り基線維持を確認する）。モック・テストデータなし。
- 検査 1〜5 は PR 作成前にローカルで、6 は PR 作成後に実行する。結果は `code-summary.md` と PR 本文に貼る。
