# ハンドオフ — b50: `aidlc-bolt set-autonomy` 面の一括着地 — human presence ガード（I11）の CQRS 裁定 + `SwitchAutonomyUseCase` + `--mode` 逐語ピン（2026-09-05）

対象: GitHub #72（#7 キュー 6）。設計書: [`b50-set-autonomy/design.md`](b50-set-autonomy/design.md)。前段: [`handoff-b49.md`](handoff-b49.md)。
オーナー裁定（着手前に質問、設計 §0）: **A′** — ガードの材料は 2 種類あって性質が違う: (1) 直近のゲート解決の時刻は集約自身のイベントの帰結なので**集約が状態として持つ**（規則 4 — 監査シャードは投影＝遅延するリードモデルであり、そこから読むのは禁止パターン）、(2) `HUMAN_TURN` はフックがシャードへ直接書く一次の事実で我々のイベントの投影ではないので**外部の入力**として合成ルートが読み、値オブジェクトで集約に渡す。判断は集約のガード。**A** — 受理集合は upstream に揃える（状態ガードを外す）。

用語（初見向け）: **autonomy（自律モード）** = Construction の残り Bolt を人間のゲート承認なしで進めるモード。**昇格** = gated → autonomous（強い権限の付与）、**降格** = その逆。**human presence ガード** = 昇格のときだけ「直近のゲート解決より後に本物の人間がタイプしたか」を確かめる仕組み（仕様 I11）。**HUMAN_TURN** = ハーネスのフックがプロンプト送信のたびに監査シャードへ追記する「人が居た」行。

## やったこと
- **ドメイン**: `workspace::HumanTurns`（`find_in(buffer)` が唯一の構築経路 — `HUMAN_TURN` の最新時刻と「追跡が有効か」（DocumentKB の来歴行だけの台帳は追跡なし））。`AuditEventRecord::instant()`（秒精度 ISO の解釈は行の持ち主へ）。`IntentExecution` に `last_gate_resolution_at`（`GateApproved` / `GateRejected` / autonomous への `AutonomyModeSet` の適用で `occurred_at` を刻む。降格・park・他のイベントでは動かない）、クエリ `human_acted_since_gate(&HumanTurns)`（追跡なし → true、turn なし → false、解決なし → true、秒比較で同秒は fail-closed）、`switch_autonomy(intent, mode, &turns, guard, at)`（**状態ガード撤去**、昇格 ∧ ガード ∧ 不在 → `HumanPresenceRequired`）。スナップショット DTO に `last_gate_resolution_at`（欄不在は `None`）。
- **ユースケース**: `SwitchAutonomyUseCase<E, I>`（find → find intent → コマンド → store、`Conflict` 1 回再試行、CQS で `Ok(())`）、入力 VO `AutonomySwitchRequest { mode, turns, human_presence_guard }`、封筒 `SwitchAutonomyError`。
- **RMU**: 変更なし（`AUTONOMY_MODE_SET { Mode }` と `Construction Autonomy Mode` の投影は既存）。`jump_refusal` の綴り表に `human-presence-required`。
- **app**: 新しい面 **`aidlc-bolt`**（`Face::Bolt`）。`set-autonomy` は upstream `handleSetAutonomy` の順序と逐語（`Missing --mode <autonomous|gated>` → `Invalid --mode: <m>. Must be 'autonomous' or 'gated'.` → 実行カーソル → 投影の追いつき → 監査シャードの読取（`read_all_audit_shards` → `HumanTurns::find_in`）→ 状態ファイルの欄検査（`State update failed: Field not found in state file: "Construction Autonomy Mode". Cannot update — refusing to silently no-op.`）→ ユースケース → `catch_up` → stdout `{"emitted":"AUTONOMY_MODE_SET","mode":"<mode>","state_updated":true}`）。昇格拒否は `Refusing to switch Construction to autonomous: …` の逐語。他 7 動詞は not-wired、未知は upstream `:908` の逐語。`AIDLC_SKIP_HUMAN_PRESENCE_GUARD=1` は既存の判定を再利用。
- **Quint v2.7**: `actSetAutonomy` の `status == Running` を撤去。新しい不変条件は無し（既存 19 本 green のまま）。ITF 14 本を b49 と同じコマンドで再採取。
- **仕様**: `docs/specs/10-orchestration.md`（§2.3 の `human_acted_since_gate` を集約のクエリへ、§3、§6 I11、§9 v2.7、§10 実装ノート、B9）、`docs/specs/11-workspace.md`（B9 供給面）、`docs/specs/deviations.md` #7。
- **テスト**: 新規 51 本（2,067 → 2,118）。全ゲート緑（Quint 25 ステップ PASS、カバレッジ 99.13%）。

- **設計との差分 10 点**（設計 §9）: `AuditEventRecord::instant()`、`mutate(occurred_at)`、`const fn`、`read_all_audit_shards` の綴り、`jump_refusal` の 17 変種、面の表は `cli/mod.rs` のみ、env の end-to-end は `unsafe` 禁止で組めず単体で固定、中継形は「居ない実行」で踏む、カバレッジ回復のテスト 4 本、park 中の受理を既存テストで表明。

## 積み残し（記録のみ、起票しない）
- **繰延（deviations #7）**: `QUESTION_ANSWERED` / `SUMMARY_CONFIRMATION_RECORDED` の解決集合入り（`aidlc-log answer` / `decision` の配線と同じ Bolt）、同一シャード内の位置による同秒判定（材料が無い）、Bolt の他 7 動詞（slice 2）、own wording 2 本。
- **カバレッジの余裕が薄い**（相対ゲートの bar まで約 0.01pp ≒ 4 行）。次の Bolt で未カバーの新規行が数行出ると相対ゲートが赤になる — 失敗経路のテストを先に書く。
- **到達しにくい防御枝**: `SwitchAutonomyError::Repository` の非 Conflict 腕（`PromotePracticesUseCase` と同じ既存の穴）。
- `AIDLC_SKIP_HUMAN_PRESENCE_GUARD=1` の end-to-end はプロセス内 env 差替に `unsafe` が要るため未固定（集約の単体テストで固定）。子プロセス実行のハーネスができたら足す。

## 次
#7 キュー 7 以降（#71 WorkspaceScanner → #77 → #82 → #53 → クリティカルパス 4〜6）。
