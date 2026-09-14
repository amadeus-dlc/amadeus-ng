# code-generation セッション引継ぎ（2026-09-13）

> **この文書は U1 の成果物ではない。** 承認ガードにより U4 の記録・工程日誌 `construction/code-generation/memory.md`・記憶へ書き込めなかったため、唯一書き込めた U1 の記録ディレクトリへ一時的に置いた。再開後、内容を U4 の記録と工程日誌へ移し、このファイルは削除する。**U1 のレビューには渡さない。**

## 1. native フック接続の切り戻し（原因は未特定）

- 2026-09-12 20:54 JST: U4 で `.claude/settings.json` のフック 14 本を `"$CLAUDE_PROJECT_DIR/target/release/aidlc" hook <name>` へ切り替えた（`plan-approval-guard` / `run-sensors` は配布 `.ts` のまま）。
- 以後、監査に `HUMAN_TURN` が **0 件**。最後の記録は切替前の 2026-09-12T10:52Z。利用者の発言も構造化質問の回答も記録されていない。
- 影響: 計画承認の受領が `Plan Approval requires the actual offered choice from this prompt and session` で拒否された。承認ガードが `--doctor`・`aidlc-sync --check`・記憶への書き込みまで止める循環になった。
- 2026-09-13: 利用者が `! git checkout -- .claude/settings.json scripts/aidlc-sync/installed.json` で切り戻した。`settings.json` の native 登録が 0 本になったことを確認済み。`scripts/aidlc-sync/patches/selfhost-native-hook-bindings.patch` は未適用の成果として残してある。
- 原因の見立て（未検証）: `harness_binding_contract` は「native 登録からバイナリが起動し stdin を受ける」ことしか検査しておらず、実ハーネス上で `record-human-turn` が監査行を生むかを見ていない。接続の形は正しく、振る舞いが欠けていた。

## 2. 再開時の確認（Claude Code を完全に再起動した後）

1. `./target/release/aidlc --doctor` の D2.e が `binding mismatch (declared native, registered distributed: fold-usage)` へ戻る（切り戻しが効いた証拠）
2. `bun scripts/aidlc-sync.ts --check` が成功する
3. 構造化質問の回答が `HUMAN_TURN` として監査に記録される

## 3. このセッションの利用者裁定のうち、記録に残っていないもの

- **未配線 8 件をこの試行で実装する**（U2 の責任）。対象は `aidlc-state lookup`、`aidlc-utility project-description` / `scope-table` / `stage-table` / `codekb-scope-diff` / `codekb-snapshot` / `codekb-path` / `codekb-publish`。差し戻し枠を使い切っているため、code-generation を閉じた後には入れ直せない。
- **構造化質問は 1 回に 1 問ずつ**聞く。

## 4. 工程の状態

- `GATE_REJECTED` を記録済み。理由は「U4 の D2.e 追従で U3 の終端レビュー受領が失効し、回復レビューがエンジンに拒否されたため再レビューする。併せて U4 の R-01 を是正する」。試行は U1 からリセットされ、状態は `Revising Code Generation (revision 3 of 3)`。
- **U1**: `UNIT_STARTED` 記録済み。計画承認は `code-generation-questions.md` に `[Answer]: Approve Plan`・指紋 `sha256:81e0dd9c…`・`DECISION_RECORDED` まで済んでいるが、**受領（`aidlc-log.ts answer`）が上記の理由で記録できていない**。再開後は `[Answer]:` を空へ戻し、指紋を再計算して取り直す。
- **U4**: 独立レビュー iteration 1 は READY で受領済み。Major R-01 は `scripts/aidlc-selfhost/host-binding.json` の `binding_selected: "distributed-typescript"` が live の native 接続と矛盾する、というもの。**切り戻し後の現在は実態と一致に戻っている**が、native を再適用すると再び矛盾する。Minor R-02 は契約テストの独立実行が一部未完。
- **検証（切替前の作業ツリー）**: workspace 全通し 119 テストバイナリ・3,805 件成功・0 失敗、line 98.0641%（絶対床 90.0% PASS）。fmt / clippy / `cargo lint` / `tools/lint` / `cargo audit` / `aidlc-sync --check` / bun テスト 12 ファイル / `verify-corpus` / Quint ゲート / release の `doctor_contract` はすべて成功。

## 5. 再開後の段取り

1. 上記 2 の確認
2. native `record-human-turn` が監査行を生まない原因の特定と、監査行まで検査する契約の追加（U4 の是正）
3. U1 の計画承認を取り直し → Unit 完了 → 独立レビュー
4. U2 の計画を改訂し、未配線 8 件を TDD で実装 → 独立レビュー
5. U3・U4 を再受領（U4 は R-01 の是正と `required-surface.md` の分類更新を含む）
6. 本ファイルの内容を U4 の記録と工程日誌へ移し、本ファイルを削除する
