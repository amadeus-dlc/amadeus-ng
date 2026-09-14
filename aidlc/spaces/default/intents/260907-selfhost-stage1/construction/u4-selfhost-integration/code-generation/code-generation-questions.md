# U4 の実装計画の確認

## Plan Approval

[Question]: Approve this exact Code Generation plan?

[code-generation-plan.md](code-generation-plan.md) と、その中の Testing Contract、および [unit-test-instructions.md](unit-test-instructions.md) を承認対象とする。

配布の入口をこの build の Rust プロセスへ接続する単位である。実測で確かめた現在地は「`.claude/settings.json` のフック登録 16 本がすべて配布 `.ts` を指し、native 面への接続は 1 本も無い」「本 build の `NATIVE_HOOKS` は 14 本で、配布 16 本のうち `plan-approval-guard` と `run-sensors` が native に無い」「ホスト/ターゲットの識別・切替・復帰の資産は現物に無い」。

6 ステップで進める。(1) bugfix 9 ステージが実際に踏む動詞・フック・受領記録を実バイトから列挙、(2) フック接続を native 面へ差し替え、(3) 動詞接続と不足の明示、(4) 読取り専用の再利用（`review-brief` 3 動詞・`testing-posture` 2 動詞）の副作用検証、(5) ホスト識別と切替準備、(6) 統合検証と引継ぎ。

計画は 3 つの前提を置いている。**`plan-approval-guard` と `run-sensors` は配布 `.ts` のまま残す**（前者は native に無く、後者はセンサーとしてスコープ外）。**未配線の動詞は U4 で実装せず、不足として U2 の責任に送る**（C6 の writer / integrator の分担）。**`statusLine` は変更しない**（D2.a の名指しが本家 17 本に対し 16 本のままになるが、U3 が記録済みの既知差）。

[Approval Fingerprint]: sha256:0125d23964950c90608aa2f6e24dc27f861e8b59d44acd0970f1493e3488b33a

- Approve Plan — 計画とテスト境界を承認して U4 を実装する。
- Request Changes — 計画またはテスト手順を修正する。

[Answer]: Approve Plan
