# stage-1 切替の手順書（2026-09-23）

stage-1 の切替条件 3 つのうち、機械で取れる 2 つ（`doctor_pass` / `all_ci_jobs_pass`）の準備と、人間が行う実地スモーク（`live_bugfix_smoke_pass`）の手順をまとめる。条件の正本は `scripts/aidlc-selfhost/host-binding.json` である。

---

## 1. いまの到達点

### 1-1. 何が stage-1 を阻んでいたか

前の引き継ぎ（`stage1-handoff-20260923.md`）の時点では、native の「2.8.2 追従」は指示の綴り・二段形コマンドの対応表・doctor に閉じていた。実行時のプロトコル（受領の形式・ガード・指示の中身）は 2.7 系のままだった。

配布 `.claude/` の 2.8.2 手順書どおりに指揮役が打つコマンド列を再生すると、native は requirements-analysis のレビュー記録で止まっていた。

### 1-2. 直したもの（PR の積み重ね）

| PR | 内容 | 再生（native） |
|---|---|---|
| [#142](https://github.com/amadeus-dlc/amadeus-ng/pull/142) | 再生ハーネス（`scripts/aidlc-selfhost/e2e/`） | 52/86 |
| [#143](https://github.com/amadeus-dlc/amadeus-ng/pull/143) | レビュー受領の 2.8.2 形式、runtime-graph の再構築、承認・差し戻しの人間の返答ガード | 69/82 |
| [#144](https://github.com/amadeus-dlc/amadeus-ng/pull/144) | 計画承認の 2 行タグと射影、run-stage 指示の文脈（consumes・知識・レビュー形・memory.md）、要約確認のガード、Unit を切らない CG のゲート | 82/82 |
| [#145](https://github.com/amadeus-dlc/amadeus-ng/pull/145) | plan-approval-guard の native 化（配布 TS 版は native の承認を読めず、承認後も CG の書込を全部止めた） | 89/89 |
| [#146](https://github.com/amadeus-dlc/amadeus-ng/pull/146) | 再生に PreToolUse の保護フックと Stop フックを追加 | 102/102 |

配布 2.8.2 も同じハーネスで 102/102 である。run-stage 指示は全ステージで 2.8.2 と一致する（`compare-directives.sh` の差分 0）。session-start の文脈と deliver-stage-rules の配送内容も一致を確かめた。

### 1-3. `--doctor`

#146 の先端で `target/release/aidlc --doctor` は 28 passed / 0 failed（クリーンな clone、記録もイベントストアも無い状態）。`doctor_pass` の証拠は、マージ後の main で採り直す（→ §2）。

---

## 2. 切替の手順

スタックした PR がすべて main へ入ってから行う。

### 2-1. native を作り、識別を採る（`host-binding.json` step 1）

```bash
git switch main && git pull
cargo build --release -p aidlc
git rev-parse HEAD
shasum -a 256 target/release/aidlc
```

### 2-2. 機械で取れる証拠を埋める（step 2 の前半）

- `doctor_pass`: 記録とイベントストアが無い状態で `target/release/aidlc --doctor` を実行し、終了コード 0・失敗 0 を採る。`aidlc/spaces/default/intents/.aidlc-store.sqlite` が在れば、測定のあいだだけ作業ツリーの外へ退避する（2026-09-19 の測定と同じ採取条件）
- `all_ci_jobs_pass`: 2-1 のコミットの CI（`merge_group`）が全ジョブ成功であることを確認する。coverage が Issue #134 のテスト（`pipeline_link_contract::concurrent_duplicate_completions_persist_only_one_receipt`）で落ちたら、再実行で通れば既知の flake である

採った値を `host-binding.json` の各 `evidence` へ書く。

### 2-3. このリポジトリの中だけ `aidlc` を native に向ける

フックの登録は全件 `aidlc engine hook <name>` と綴られ、PATH 上の `aidlc` で解決される。マシン全体の `aidlc`（`~/.local/bin/aidlc` → 配布版）は変えず、このリポジトリの中だけを native に向ける。`mise.local.toml` は gitignore 済みである。

```toml
# mise.local.toml（リポジトリ直下）
[env]
_.path = ["{{config_root}}/target/release"]
```

```bash
mise trust
command -v aidlc   # → <repo>/target/release/aidlc であること
```

Claude Code は mise が有効なシェルから起動する（フックは Claude Code の環境変数を引き継ぐ）。

### 2-4. フックの登録を戻す（step 4）

作業ツリーの `.claude/settings.json` は、2026-09-23 にフックを外した状態になっている。コミット済みの登録へ戻し、配布のまま残す 1 本（run-sensors）だけを配布 TypeScript へ向ける。

```bash
git checkout -- .claude/settings.json
```

`run-sensors` の登録（PostToolUse `Write|Edit`）の `command` を次へ書き換える。ほかの 15 本は `aidlc engine hook <name>` のままでよい。

```text
bun "$CLAUDE_PROJECT_DIR/.claude/hooks/aidlc-run-sensors.ts"
```

plan-approval-guard は #145 で native へ移したので、書き換えない。

### 2-5. 再起動と接続確認（step 5）

Claude Code を**完全に**再起動する（`/clear` では足りない）。フックの承認を求められたら `/hooks` で承認する。

```bash
aidlc --doctor   # 「Native hook bindings」が ✓ であること
```

### 2-6. 実地スモーク（`live_bugfix_smoke_pass`）

題材は Issue #134（並行した pipeline link の完了報告が WouldBlock で失敗として返る）である。

```text
/aidlc bugfix Issue #134: 並行した pipeline link の完了報告が、記録に成功した側も WouldBlock で失敗として返る
```

開始 → 質問 → 人間のゲート承認 → 完了までを、6 つのゲート（RE / RA / CG / B&T / DP / DE）すべて人間の返答で通す。`runtime::run` の繰返し、試験装置による人間応答の注入、承認保護の無効化は証拠にならない。

見ておく所:

- 各ゲートで「人間の返答が無い」「選択肢と一致しない」という拒否が出ないこと（出たら、返答が `Approve` そのものかを確かめる）
- CG で、計画承認の後に開発者の派遣とソースの書込が止められないこと
- 止まったら、その時点の `aidlc/spaces/default/intents/<record>/audit/` と拒否文を残す

### 2-7. 記録とタグ（step 3・6）

証拠を intent 記録と `host-binding.json` へ残し、検証済みコミットに安定タグを付ける。`binding_selected` を `native` へ、`host` をタグ・コミット・指紋つきで更新する。

---

## 3. 戻し方

- `mise.local.toml` の `_.path` を消す → PATH の `aidlc` はマシン全体の配布版へ戻る。**ただし配布版のアクティブは 2.9.0 である**（`~/.local/share/aidlc/active-version`）。2.8.2 の手順書のまま配布へ戻すなら、アクティブを 2.8.2 にするか、2.9.0 との差を先に確かめる
- フックの登録は `git checkout -- .claude/settings.json` で配布 TypeScript の形へ戻る（`host-binding.json` の `rollback`）

---

## 4. 既知の差と未対応（実地で気にするもの）

| 項目 | 状態 | bugfix スモークへの影響 |
|---|---|---|
| plan-approval-guard の Unit ごとの書換え | 段階全体の承認だけで判定する | なし（bugfix は Unit を切らない） |
| 書込先を特定できないシェル（`eval` など）・未知の工具 | native は通す（2.8.2 は止める） | なし（緩い側） |
| 計画承認の指紋 | `sha256:<hex>`（v3 を名乗らない）。発行エポックとソース床も束ねる | `next` をやり直すと承認のやり直しが要る場合がある |
| Change Control（strict / relaxed） | 未実装。`[Planned Source]` は出すが判定には使わない | なし |
| `--review-file` | 未対応（依頼が返す既定の置き場だけを読む） | なし（手順書の既定経路） |
| `aidlc engine orchestrate help` など 6 入口 | 未配線（Issue #137） | 指揮役が help を引くと拒否される。bugfix の手順には無い |
| `aidlc --version` | native は受け付けない | なし |
| 配布 2.8.2 の plan-approval-guard | 承認待ちを開いた後、CG の `report --result approved` まで止める（再生で観測） | native では起きない（#145） |

---

## 5. 参照先

| 対象 | 場所 |
|---|---|
| 再生ハーネス | `scripts/aidlc-selfhost/e2e/replay-bugfix.sh` / `compare-directives.sh` |
| 切替条件と手順の正本 | `scripts/aidlc-selfhost/host-binding.json` |
| native / 配布のフック分類 | `scripts/aidlc-selfhost/hook-binding.json` |
| 前の引き継ぎ | `aidlc/spaces/default/knowledge/stage1-handoff-20260923.md` |
