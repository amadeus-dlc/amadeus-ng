# stage-1 引き継ぎ（2026-09-23 作成 / 作業は 2026-09-22 UTC）

前セッションの終了時点。PR #138 / #140 のマージまで完了し、次は stage-1 切替の 3 条件を埋める段階。

- 直近の main: `fe0093cb`（PR #140 = 本ファイルの追加、2026-09-23T03:25:02Z）
- その 1 つ前: `27521f67`（PR #138、2026-09-22T15:41:24Z）
- このファイルの置き場: space レベルの `knowledge/`。過去の handoff は `intents/260822-stage1-selfhost/construction/handoff-*.md` にあったが、その intent 記録は 2026-09-07 のリセット（`3ac8a36c`）で消えている

---

## 1. 前セッションで終えたこと

PR #136 の最終ゲートがオーナーへ引き継いだ**2 件の裁定**に決着をつけ、コード変更が必要な側を実装してマージした。

### 裁定 1（D19）— Stop フックの案内は 2.8.2 と逐語一致させる

`order.md` §9 Scenario 3 の 2 つの `Then` が両立しない問題。

| 行 | 要件文 |
|---|---|
| `:204` | 案内文が綴るコマンドは、2.8.2 の同じ場面の案内文と**同じ呼び方である** |
| `:205` | 案内文は `bun .claude/tools/aidlc-orchestrate.ts` を**含まない** |

本家 2.8.2 の `continue-workflow` フック自身がこの `bun` 綴りを出す
（`.claude/hooks/aidlc-continue-workflow.ts:1179,1190,1194`）ため両立しない。

**`:204`（2.8.2 追従）を正とした。** 根拠は Scenario 2 の `But` 節（`:199`）と D14「綴りを推測しない」。
`continuation.rs` は現状のままが正しく、**コード変更は無い**。

記録先は 2 か所:

- `.takt/tasks/20260916-125336-aidlc-2-8-2-native-bugfix-1-4/order.md` — §2 に D19、Scenario 3 に例外注記
- `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/upstream-contracts.md` — 「実行時に出す指示の綴りも、借り物の契約である（2026-09-22 裁定）」節

2 か所に書いたのは、`.takt/.gitignore` が `.takt/` 配下を全除外しており、order.md だけではリポジトリに残らないため。

### 裁定 2（D20）— 写像だけで受理できる 1 件を配線、残り 6 件は Issue

`NotWired` のまま残っていた 7 入口を実測したところ**一様ではなかった**。

| 入口 | 写像先の一段形 | 分類 | 扱い |
|---|---|---|---|
| `jump execute` | `aidlc-jump execute` | **mapped** | **配線済み（PR #138）** |
| `state unpark` | `aidlc-state unpark` | needs-implementation | #137 |
| `orchestrate help` | `aidlc-utility help` | needs-implementation | #137 |
| `scope change` | `aidlc-utility scope-change` | needs-implementation | #137 |
| `config set` | `aidlc-utility change-control` | needs-implementation | #137 |
| `graph compile` | **無し**（graph 面が無い） | needs-implementation | #137 |
| `status` | **無し**（本家は routeOnly） | needs-implementation | #137 |

`jump execute` の一段形は元から実装済み（`runtime/jump.rs:18`）で、`WIRED` 表に組が無いだけだった。
`engine_route.rs` の `WIRED` を 28 → 29 件に。

`required-surface.json` は**変更していない**。`acceptance` / `classification` は一段形の受理状況を
記録する欄であり、二段形の配線有無では変わらないため。`doctor` も変更なし（`missing_from` は
`bugfix_required: true` の 27 入口だけを照合し、`jump execute` は `false`）。

### 成果物

| 種別 | リンク |
|---|---|
| PR #138（マージ済み、`27521f67`） | https://github.com/amadeus-dlc/amadeus-ng/pull/138 |
| PR #140（マージ済み、`fe0093cb`） — 本ファイル | https://github.com/amadeus-dlc/amadeus-ng/pull/140 |
| Issue #137 — 残り 6 入口の新規実装 | https://github.com/amadeus-dlc/amadeus-ng/issues/137 |
| Issue #139 — 監査 M7 の例外作法 4 段階 | https://github.com/amadeus-dlc/amadeus-ng/issues/139 |
| Issue #134 にコメント — CI 失敗の 2 例目 | https://github.com/amadeus-dlc/amadeus-ng/issues/134#issuecomment-5790538612 |

Issue #139 が生まれた経緯: CodeRabbit が「規則ヘッダに `**例外**: <段階>` を足せ」と指摘したが、
出典は `CONSISTENCY-AUDIT-2026-08-24.md:180` の**推奨**であって規約ではなかった。README `:5` が
定義するヘッダ欄は 裁定日／適用例（PR）／機械強制 の 3 つで、`**例外**:` を持つ規則ファイルは
**0 / 22 本**。根拠つきで反論して resolve し、推奨自体は妥当なので Issue 化した。

---

## 2. stage-1 の現在地

Issue #7 は最終更新が 2026-09-04 前後で、`docs/specs/00-policy.md`（2026-09-07 に削除済み）を
参照したままなど古い。**実際のゲートは `scripts/aidlc-selfhost/host-binding.json` の
`switch_preconditions` 3 件**である。

| 条件 | 記録上 | 実測の評価 |
|---|---|---|
| `doctor_pass` | **met** | 2026-09-19 に 28 passed / 0 failed。ただし測定した実体は `sha256 69ae73b4c48c09c860d2c35036f0e09307036e8fa1dc58b6af95c3ab2944a8ad` で、その後 #136 / #138 が入ったため**再取得が必要** |
| `all_ci_jobs_pass` | unmet | **実質達成済み**。main `27521f67` の CI（`merge_group`）が success。証拠を書き込めば met にできる |
| `live_bugfix_smoke_pass` | unmet | **未実施**。実 Claude Code・実フック・人の応答が必要。order.md §7 が「人間が行う」とした唯一の条件 |

実地スモークの題材は **Issue #134**（並行した pipeline link の完了報告が WouldBlock で失敗として返る）
を温存してある（order.md §7「Issue #134 の不具合の修正（実地スモークの題材として取っておく）」）。

ただしこの温存には勘定に入れるべきコストがある。**#134 は latent なバグではなく、実際に CI を
赤くする**（→ §4-6）。スモークの実施を待つあいだも `coverage` が落ち続けるので、「題材として
取っておく」か「先に直してスモークの題材は別に選ぶ」かはオーナー裁定の余地がある。

---

## 3. 前セッションの実測で新たに判明したこと

### 3-1. 切替の起動条件は 1 つに単純化されている

2.8.2 で `.claude/settings.json` の登録の綴りが**全件 `aidlc engine hook <name>` に統一**され、
設定本文からは native / 配布を区別できなくなった。`hook-binding.json` の `command_template_note`
がそう言っている:

> パスを綴らず PATH 上の aidlc を解決するため、切替の起動条件は『PATH 上の aidlc が本 build であること』だけである。

登録は 18 件 / フック名 16 種（`fold-usage` と `record-human-turn` が二重登録）。

### 3-2. その PATH が native を指していない

```text
/Users/j5ik2o/.local/bin/aidlc          # ランチャ（1866 バイトの sh）
  → ~/.local/share/aidlc/active-version  = 2.9.0
  → ~/.local/share/aidlc/versions/2.9.0/aidlc
$ aidlc --version
aidlc 2.9.0 (runtime 2.9.0)
```

つまり今このリポジトリのフックを実行しているのは**配布版 2.9.0** である。

### 3-3. 上流が 2.9.0 へ動いている ← 要判断

| 対象 | 版 |
|---|---|
| このリポジトリの `.claude/tools/data/aidlc-stamp.json` | **2.8.2** |
| `scripts/aidlc-selfhost/required-surface.json` の `upstream_version` | **2.8.2** |
| マシンにインストール済み | 2.8.2 と **2.9.0**（アクティブは 2.9.0） |

native は 2.8.2 の面へ追従してきたので、**追従先が 1 つ古くなった**。
`required-surface.json`・ゴールデン（`tests/golden/selfhost-stage1/required-surface-sources/`）・
凍結出典がすべて 2.8.2 基準なので、2.9.0 へ動かすなら影響範囲の調査から。

### 3-4. native ビルドが古い

`target/release/aidlc` は 2026-09-16 21:33 のもので、#136 / #138 を含まない。
どの証拠を採るにも先に作り直しが要る。

---

## 4. 再開時の注意点

### 4-1. `.claude/settings.json` にローカル変更がある（コミットしない）

作業ツリーに未コミットの変更が残っている。`plan-approval-guard` と `run-sensors` の 2 本が
`aidlc engine hook <name>` → `bun "$CLAUDE_PROJECT_DIR/.claude/hooks/aidlc-<name>.ts"` に
書き換えられている。

これは **D17 が「切替時に人間が書き換える」とした 2 本**で、切替手順 step 4 の該当部分が
すでに手元で適用された状態。**コミット対象に含めないこと。**

### 4-2. その結果 `engine_hook_wiring_contract` が手元でだけ落ちる

```text
the_binding_definition_counts_registrations_apart_from_hook_names
  assertion `left == right` failed: 登録件数が配布の現物と一致しない
  left: Some(18)   # hook-binding.json の registration_count
 right: Some(16)   # 実際に二段形で登録されている数（2 本が bun 形なので 2 減）
```

**コードの問題ではない。** `.claude/settings.json` を退避すれば 7 件すべて通る。
フルテストを採るときは退避してから走らせ、終わったら戻すこと。

```bash
git stash push -- .claude/settings.json
PROPTEST_RNG_SEED=20260823 cargo test --workspace --no-fail-fast
git stash pop
```

前セッションではこの状態で **3999 passed / 0 failed**、`cargo clippy --workspace --all-targets`
警告なし、`cargo lint` 終了コード 0 を確認済み。

### 4-3. `.takt/` は git 管理外

`.takt/.gitignore` が `*` で全除外している。order.md の D19 / D20 はリポジトリに残らないので、
恒久的に残すべき裁定は `aidlc/spaces/default/knowledge/` 側にも書くこと。

### 4-4. マージはマージキュー経由

`main` にマージキュー（`MQ_kwDOT_l16c4AA9UH`）があり、かつリポジトリ設定で auto-merge が無効
（`autoMergeAllowed: false`）。`gh pr merge` は
`Auto merge is not allowed for this repository` で弾かれる。GraphQL で直接投入する:

```bash
PRID=$(gh api graphql -f query='{ repository(owner:"amadeus-dlc",name:"amadeus-ng"){ pullRequest(number:NNN){ id } } }' --jq '.data.repository.pullRequest.id')
gh api graphql -f query="mutation { enqueuePullRequest(input: {pullRequestId: \"$PRID\"}) { mergeQueueEntry { position state } } }"
```

### 4-5. レビュースレッドのゲートが cancelled を fail と表示する

`review-thread-resolution.yml` は `pull_request_review` と `pull_request_review_comment` の
両方で発火し、`concurrency: cancel-in-progress: true` が片方を打ち切る。打ち切られた実行を
`gh pr checks` は **fail** と表示し、PR が `UNSTABLE` に見える。打ち切られた run を
`gh run rerun <id>` すれば `CLEAN` に戻る。必須ゲートは commit status の
「Check unresolved comments」の方。

### 4-6. `coverage` は自分の変更と無関係に落ちることがある（Issue #134）

`coverage` ジョブの `pipeline_link_contract::concurrent_duplicate_completions_persist_only_one_receipt`
が、並行した完了報告のロック競合で落ちる。**Issue #134 の既知バグ**である。

```text
assertion `left == right` failed:
[... "already completed this attempt." ..., ... "projection: read: io: WouldBlock at .../.aidlc-store.sqlite" ...]
  left: 0    ← 成功した起動の数
 right: 1
```

書き込みに成功した側が、その後の投影の読み取りでロック競合に当たって失敗として返っている。

- **変更内容と無関係に起きる。** 2026-09-23 の 2 例目（run 35810746755）は、Markdown 1 本を
  足しただけの PR #140 で発生した。Rust には一切触れていない
- **`check` は通り `coverage` だけ落ちる。** coverage の計測で処理が遅くなり、並行する 2 つの
  起動が重なりやすくなるため
- **再実行で通る。** `gh run rerun <run-id> --failed`。ただし `coverage` は 20 分かかる

`coverage` が落ちたら、まずこのテスト名かどうかを確認すること。該当すれば自分の変更は疑わなくてよい。
別の flake として Issue #82（相対ゲートの経路揺れ、±0.01〜0.02pp）もある。

### 4-7. markdownlint はこのリポジトリで強制されていない

`.markdownlint-cli2.jsonc` は存在するが、**CI には配線されていない**（`.github/workflows/` に
記述なし）。このファイルの役割は監査シャードと developer-brief を `ignores` することだけで、
追跡済みの既存ファイルも既定規則を破っている。

```text
upstream-contracts.md / coding-rules/README.md / CLAUDE.md の 3 本:
  MD013/line-length            12
  MD040/fenced-code-language    1
  MD012/no-multiple-blanks      1
  MD001/heading-increment       1
```

一方 **CodeRabbit はこの設定ファイルを根拠に MD040 を指摘してくる**（PR #140 で実際に来た）。
新しい Markdown を足すときは、コードフェンスに言語指定を付けておくと 1 往復省ける。出力の
貼り付けなら `text`。

MD013（80 桁）などは直さなくてよい。この 1 本だけ折り返すとリポジトリの他の文書と揃わなくなる。
規約として効かせるなら CI 配線と既存是正をまとめて行う話になる。

### 4-8. マージ済みブランチが残る

リポジトリ設定で `delete_branch_on_merge` が無効なので、マージ後もリモートにブランチが残る。
2026-09-23 時点で `origin/feat/selfhost-wire-jump-execute` と `origin/docs-stage1-handoff` が
未削除。掃除するかは任意。

---

## 5. 次の選択肢（前セッションの提示内容）

1. **（推奨）安く取れる証拠から埋める** — native を作り直し、`--doctor` を再取得して
   `doctor_pass` を更新。あわせて `all_ci_jobs_pass` を main `27521f67` の CI 成功で met にする。
   残るは実地スモーク 1 件だけになる

   ```bash
   cargo build --release -p aidlc
   git rev-parse HEAD
   shasum -a 256 target/release/aidlc
   target/release/aidlc --doctor
   ```

   注意: 2026-09-19 の測定時は、作業記録もイベントストアも無い状態で採る必要があった
   （`aidlc/spaces/default/intents/.aidlc-store.sqlite` を測定のあいだだけ作業ツリー外へ退避）。
   `host-binding.json` の `doctor_pass.evidence` に採取条件が詳述してある

2. **2.9.0 への追従方針を先に決める** — 2.8.2 のまま stage-1 を切るか、2.9.0 へ追従してから切るか。
   `required-surface.json` / ゴールデン / 凍結出典がすべて 2.8.2 基準なので、影響範囲の調査から

3. **実地スモークの準備をする** — Issue #134 を題材に、`host-binding.json` の
   `switch_procedure` step 1〜5 を踏む段取りを詰める（実行自体は人間）

4. **Issue #7 を現状に合わせて更新する** — 削除済みの `docs/specs/00-policy.md` 参照など、
   古くなった記載を直す

---

## 6. 参照先

| 対象 | 場所 |
|---|---|
| 切替 3 条件と手順の正本 | `scripts/aidlc-selfhost/host-binding.json` |
| native / 配布のフック分類 | `scripts/aidlc-selfhost/hook-binding.json` |
| 必要集合（入口 49 件、bugfix 必須 27 件） | `scripts/aidlc-selfhost/required-surface.json` |
| 二段形の写像表 | `modules/app/aidlc/src/cli/engine_route.rs`（`WIRED` 29 / `WIRED_VERBLESS` 1 / `ROUTES` 23 noun） |
| 実行時の指示の契約テスト | `modules/app/aidlc/tests/runtime_directive_contract.rs` |
| 前タスクの指示書（git 管理外） | `.takt/tasks/20260916-125336-aidlc-2-8-2-native-bugfix-1-4/order.md` |
| 上流契約の扱い（裁定 1 の記録先） | `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/upstream-contracts.md` |
| stage-1 トラッキング（古い） | Issue #7 https://github.com/amadeus-dlc/amadeus-ng/issues/7 |
