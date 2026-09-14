# U4: セルフホスト切替の実行準備

配布の入口をこの build の Rust プロセスへ接続するための、本リポジトリ固有の手順である。
一般向けインストーラへは広げない (C8)。

**この文書が揃っていることは準備であって達成ではない。** 実地スモーク (FR7)・自己診断・
CI 全ジョブ成功・切替 (FR8) は、後続の検証・切替工程で別々に実施して証拠を残す
(C8 `not_proof_of_completion`)。

## 資産

| 資産 | 役割 | 検査 |
| --- | --- | --- |
| [`scripts/aidlc-selfhost/hook-binding.json`](../../../../../../../../scripts/aidlc-selfhost/hook-binding.json) | 配布フック登録 16 本の分類 (native 14 / 配布 TS 2) と native 側の起動形 | `cargo test -p aidlc --test harness_binding_contract` |
| [`scripts/aidlc-selfhost/host-binding.json`](../../../../../../../../scripts/aidlc-selfhost/host-binding.json) | ホスト/ターゲットの識別、切替前提、切替手順、復帰手順 | `cargo test -p aidlc --test host_binding_contract` |
| [`scripts/aidlc-selfhost/required-surface.json`](../../../../../../../../scripts/aidlc-selfhost/required-surface.json) | bugfix 一周が踏む入口の必要集合と配線状態 | `cargo test -p aidlc --test required_surface_contract` |
| [required-surface.md](required-surface.md) | 同上の読み物版と U2 への申し送り | — |

CI では `check` と `coverage` の 2 ジョブへ Bun の導入段を足した。`read_only_reuse_contract` が
配布 TypeScript を実際に回して副作用の無さを見るためである。版と Action は既存の
`aidlc-distribution` ジョブと同じ固定値を使い、それ以外の CI 設定は変えていない。

## 実体の識別

ホストは「作業を進める実体」、ターゲットは「開発中の実体」である。両者はタグ・コミット・
バイナリ実体の 3 つ組で識別する。3 つのうち 1 つでも欠けていれば `unverified` であり、
安定版として参照しない。

現在:

- **ホスト**: 無し (`status: unverified`)。安定タグをまだ作っていない。作成は切替工程である。
- **ターゲット**: 開発版 (`status: development`)。実体は `target/release/aidlc`。
  - 作る: `cargo build --release -p aidlc`
  - コミット: `git rev-parse HEAD`
  - 指紋: `shasum -a 256 target/release/aidlc`

固定値を資産へ書き込まないのは、別の版を同じ実体と取り違えないためである。

## 実地スモーク (FR7) の実行準備

### 前提

1. `cargo build --release -p aidlc` が成功し、`target/release/aidlc` が存在すること。
2. `target/release/aidlc --doctor` が本リポジトリで終了 0・失敗 0 であること。
3. 接続先が確定していること (下の「未解決事項」を参照)。
4. 人間が承認ゲートに実際に応答できる状態であること。承認保護を無効にしない。

### 手順

1. 小さな bugfix 相当の intent を 1 本用意する (本リポジトリの実在の小さな不具合)。
2. `target/release/aidlc` を作業進行の実体として、開始 → 質問 → 人間のゲート承認 → 完了まで通す。
3. 実 Claude Code・実フック・人の応答を使う。

### 証拠の残し方

作業記録から次を追えるようにする (FR7 の成功証拠)。

- 対象コミット (`git rev-parse HEAD`) と、使用したバイナリの指紋 (`shasum -a 256`)
- 実行した呼出し (面・動詞・引数)
- 回答・承認・完了の状態と監査行 (`aidlc-state.md` と `<record>/audit/<host>-<clone>.md`)
- 自己診断の出力と終了コード
- CI 全ジョブの結果 (`cargo audit` を含む)

### 証拠にならないもの

- `runtime::run` を繰り返す統合テスト
- 試験装置による人間応答の注入
- 承認保護の無効化 (`AIDLC_SKIP_HUMAN_PRESENCE_GUARD` などの迂回)
- 本工程で追加した fixture テストの成功

拒否が起きた場合も、迂回して成功扱いにしない。

## 切替 (FR8) と復帰

手順は [`host-binding.json`](../../../../../../../../scripts/aidlc-selfhost/host-binding.json) の
`switch_procedure` / `rollback` が正本である。各段に戻し方 (`undo`) を持たせてある。

要点だけを再掲する。

1. ターゲットを作り、コミットと指紋を採る。
2. 切替条件 3 つ (実地スモーク・自己診断・CI 全ジョブ) を満たし、証拠を intent 記録へ残す。
3. 検証済みコミットに安定タグを付ける。**タグの作成は切替工程で行う。本工程では作らない。**
4. `.claude/settings.json` のフック登録を `hook-binding.json` の native 側へ向ける。
5. Claude Code を完全に再起動し、`--doctor` で接続を確かめる。
6. ホストの識別と接続先の記録を更新する。

復帰は検証済みホストへ戻す。ホストが未検証のあいだは戻り先が無く、`git checkout --
.claude/settings.json` で「切替前の状態」へ戻せるだけである。これはホストへの復帰ではない。

## live `.claude/settings.json` の接続 (実施済み)

利用者の裁定 (2026-09-12) を受けて **native 14 本 + 配布 2 本へ書き換えた**。`statusLine` は
本リポジトリ現物の `statusline-combined.sh` のまま変えていない。

裁定の内容は 2 つである。

1. **D2.e の判定基準を接続定義へ追従させる。** `hook-binding.json` の宣言を正として照合し、
   宣言どおりの混在は失敗にしない。未知の native 名・解決できない呼出し・宣言と食い違う面は
   引き続き失敗である。U3 が予告していた追従であり (U3 の
   [implementation-verification.md](../../u3-selfhost-doctor/code-generation/implementation-verification.md)
   §5-9「U4 が接続定義を確定したら判定基準を追従する」)、読み替えではない。
2. **CI の bun 導入段は承認。** `check` / `coverage` の 2 ジョブへ足した段はそのまま残す。

### 接続前後の実測 (`target/release/aidlc --doctor`、本リポジトリ)

| 行 | 接続前 | 接続後 |
| --- | --- | --- |
| D2.a `aidlc-plan-approval-guard.ts present` / `aidlc-run-sensors.ts present` | 成功 | 成功 |
| D2.e `Native hook bindings` | **失敗** `binding mismatch (declared native, registered distributed: fold-usage)` | **成功** |
| D4.c `Native workflow identity` | 失敗 `.aidlc-execution: missing` | 失敗 (同じ) |
| D5.b `Native projection consistency` | 失敗 `projection unavailable (execution cursor missing)` | 失敗 (同じ) |
| 集計 / 終了 | 30 passed, 3 failed / 1 | 17 passed, 2 failed / 1 |

接続で消えたのは D2.e の 1 行だけである。残る D4.c / D5.b は**接続とは無関係の記録側の状態**で
あり、接続前から同じ原因で失敗している — この作業記録には監査シャードがあるのに native の
実行カーソル (`.aidlc-execution`) が無い。現在この作業記録を進めているのが配布 TypeScript の
エンジンであり、native のカーソルを作らないためである。したがって **`--doctor` の終了 0 は
まだ達成していない**。切替条件 4 (doctor が成功) は未達のままであり、この 2 行の解消は
記録側の経路 (U2 の未配線 8 件を含む) の話である。ログは
[tdd-logs/30-repo-doctor-after-binding.log](tdd-logs/30-repo-doctor-after-binding.log) と
[tdd-logs/31-repo-doctor-before-binding.log](tdd-logs/31-repo-doctor-before-binding.log)。

診断の `HEALTH_CHECKED` 記録も、実行カーソルが無いため成立していない (stderr に
`the active record has an audit trail but no execution cursor to record the health check`)。
この実行で作業記録へ書かれた事実は無い。

### 復帰手順 (確定)

1. `git checkout -- .claude/settings.json` — 配布 TypeScript の 16 本へ戻る。
2. Claude Code を**完全に再起動**する (`/clear` では足りない。フック設定はセッション開始時に
   読まれるため、書き換えても走行中のセッションには効かない — 逆に言えば、戻しも再起動まで
   効かない)。
3. `./target/release/aidlc --doctor` を実行し、`Native hook bindings` が
   `binding mismatch (declared native, registered distributed: fold-usage)` で失敗する
   「接続前の姿」に戻ったことを確かめる (上表の左列)。
4. 同期側も戻す場合は `scripts/aidlc-sync/patches/selfhost-native-hook-bindings.patch` を削除し、
   `scripts/aidlc-sync/installed.json` の `preserved[".claude/settings.json"].sha256` を
   `4d6190fc56a54aa11f20156b57e3337143a6e93e40ffe9321b58835122be4cfa` へ戻してから
   `bun scripts/aidlc-sync.ts --check` の成功を確かめる。

### 同期パッチ

`scripts/aidlc-sync/patches/selfhost-native-hook-bindings.patch` が、配布側
(`vendor/aidlc-workflows/dist/claude/.claude/settings.json`) の `hooks` ブロックの 16 登録を
native 形へ向ける差分を持つ。`.claude/settings.json` は同期ツールの保持対象なので、この差分は
作業ツリーのファイルを上書きしない — 上流が settings.json を変えたときの**レビューの基準**に
なる。基準を変えたので `scripts/aidlc-sync/installed.json` の
`preserved[".claude/settings.json"].sha256` も `179270c6…` へ更新した。この値は同期ツールが
staged copy から計算し直すため、誤っていれば `--check` が落ちる (実測: 「コピー 0、削除 0、
保持設定の確認 0」で成功)。

## 同期パッチの確認 (承認済み計画 Step 1)

承認済み計画は `claude-without-bedrock.patch` と現物 `settings.json` の不一致を疑っていたが、
実測の結果**不一致ではない**。このパッチは同期ツールが一時展開した**配布側**
(`vendor/aidlc-workflows/dist/claude/.claude/settings.json`、`env` / `model` / `effortLevel` を
持つ) に当たるものであり、作業ツリーの `.claude/settings.json` (保持対象 = プロジェクト所有、
既に配布と別内容) に当たるものではない。`bun scripts/aidlc-sync.ts --check` は「コピー 0、削除 0、
保持設定の確認 0」で成功する。したがってパッチの追従は不要であり、本工程では
`scripts/aidlc-sync/patches/` を変更していない。

## 未解決事項

1. **`--doctor` の終了 0 (切替条件 4)**: 未達である。残る D4.c / D5.b は、この作業記録に
   native の実行カーソルが無いことに由来する。接続の形の問題ではなく、記録側 (U2) の話である。
2. **未配線 8 件**: U2 の責任として送る。埋まるまで bugfix 一周を native だけでは通せない。
   実行カーソルが生まれないのもここに連なる。
3. **ゴールデンの採用版**: 配布 2.7.1 系と受入 `a277af21` (2.6.40 系) の差は未裁定のままである。
   切替条件 2 を判定する前に裁定を受ける。
4. **接続の実発火**: D2.a / D2.e が成功しても、それは「登録が宣言どおりに読める」ことであって、
   Claude Code が実際に native 面を起動したことではない。フック設定はセッション開始時に
   読まれるため、実発火の確認は**再起動後の実地スモーク** (FR7) で採る。
