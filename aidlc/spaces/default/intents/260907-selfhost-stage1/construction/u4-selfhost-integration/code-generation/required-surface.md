# U4: bugfix 一周が踏む入口の必要集合

配布資産の**実バイト**から列挙した、bugfix スコープ 1 周 (初期化 3 + reverse-engineering + requirements-analysis + code-generation + build-and-test + deployment-pipeline + deployment-execution の 9 ステージ) が踏む動詞・フック・受領記録である。機械可読の正本は [`scripts/aidlc-selfhost/required-surface.json`](../../../../../../../../scripts/aidlc-selfhost/required-surface.json)、検査は `cargo test -p aidlc --test required_surface_contract` が行う。

列挙の対象ファイル (18 本) は JSON の `sources` に固定してある。SKILL・conductor・共通プロトコル 6 本・bugfix スコープ定義・9 ステージ本文である。

## 分類の定義

| 分類 | 意味 |
| --- | --- |
| 配線済み (`wired`) | この build の入口 (`aidlc::cli::parse`) が受理する。native 面へ接続できる。 |
| 未配線 (`not-wired`) | この build が拒否する。native が担うべき動詞であり、実装は **U2 の責任**である (C6: writer = U2 / integrator = U4)。U4 では実装しない。 |
| 配布 TS のまま (`distributed-ts`) | C6 の `reader_candidates` として配布 TypeScript のまま再利用する。この build に対応する入口は無い。 |

「必ず踏む」は、計画したゼロ Unit の bugfix スモーク 1 周で確実に通る入口を指す。条件付きの入口 (ジャンプ・park・Unit 経路・recovery など) は `—` とし、条件を併記した。

## 件数

| 区分 | 配線済み | 未配線 | 配布 TS のまま | 計 |
| --- | ---: | ---: | ---: | ---: |
| 列挙した全動詞 | 16 | 27 | 3 | 46 |
| うち bugfix 一周で必ず踏む | 11 | 8 | 3 | 22 |

フックは配布登録 16 本のうち native 面にあるのが 14 本、配布 TypeScript のまま残すのが 2 本 (`plan-approval-guard` / `run-sensors`) である。2026-09-12 の裁定を受けて `.claude/settings.json` はこの分類どおりへ書き換え済みで、自己診断の D2.e はこの宣言を正として照合する。分類の正本は [`scripts/aidlc-selfhost/hook-binding.json`](../../../../../../../../scripts/aidlc-selfhost/hook-binding.json)。

## 未配線のまま残る必須入口 (U2 への申し送り)

bugfix 一周が必ず踏むのに、この build が拒否する入口は **8 件**である。C6 の分担に従い、U4 では実装しない。

| 入口 | 用途 | 出典 |
| --- | --- | --- |
| `aidlc-state lookup` | ステージの phase / 次ステージ / 担当を引く読取り。 | `.claude/aidlc-common/protocols/stage-protocol.md:681` |
| `aidlc-utility codekb-path` | reverse-engineering / requirements-analysis が保管先を解決する。 | `.claude/aidlc-common/stages/inception/reverse-engineering.md:321` |
| `aidlc-utility codekb-publish` | reverse-engineering が走査結果を保管する。 | `.claude/aidlc-common/stages/inception/reverse-engineering.md:353` |
| `aidlc-utility codekb-scope-diff` | reverse-engineering が対象リポジトリごとに読取り検査を行う。 | `.claude/aidlc-common/stages/inception/reverse-engineering.md:308` |
| `aidlc-utility codekb-snapshot` | reverse-engineering が走査前に compare-and-swap の写しを取る。 | `.claude/aidlc-common/stages/inception/reverse-engineering.md:165` |
| `aidlc-utility project-description` | requirements-analysis が依頼原文を逐語で取るための固定コマンド。 | `.claude/aidlc-common/stages/inception/requirements-analysis.md:63` |
| `aidlc-utility scope-table` | state-init がスコープ別の実行数を引く。 | `.claude/aidlc-common/stages/initialization/state-init.md:58` |
| `aidlc-utility stage-table` | state-init がコンパイル済みステージ一覧を引く。 | `.claude/aidlc-common/stages/initialization/state-init.md:59` |

この 8 件が埋まるまで、bugfix 一周を native エンジンだけで通すことはできない。

内訳は 2 種類ある。`aidlc-utility codekb-snapshot` と `codekb-publish` は codekb の保管先へ**書く**
操作であり、native と配布 TypeScript の両方が同じ保管先を更新する形は U4 の境界条件
(「同じ正本を二重更新しない」) に反する。残る `aidlc-state lookup`・`aidlc-utility scope-table`・
`stage-table`・`project-description`・`codekb-scope-diff` は表示・読取りに見えるが、C6 が再利用を
認めた候補 (`reader_candidates`) には入っていない。C6 は「ファイル名だけで読取り専用と扱わない」
と定めており、依存先まで含む副作用を操作単位で実測しない限り再利用と判断できない。

どちらの場合も、接続の失敗を別の更新実装へ切り替えて隠すことは C6 が禁じている。実装は U2 の
責任であり、再利用へ回すなら C6 の再利用条件の判定を先に受ける必要がある。

## 全動詞

| 入口 | 分類 | 必ず踏む | 条件 | 出典 (代表 1 行) |
| --- | --- | :-: | --- | --- |
| `aidlc-audit append` | 未配線 | — | 禁止 — 通常経路では呼ばない。権限を持つ受領は本家側でも拒否される。 | `.claude/aidlc-common/protocols/stage-protocol.md:130` |
| `aidlc-bolt set-autonomy` | 配線済み | — | ladder の回答を記録するとき。bugfix は skeleton: off で踏まない。 | `.claude/aidlc-common/protocols/stage-protocol-construction.md:70` |
| `aidlc-graph compile` | 未配線 | — | ステージグラフを再コンパイルするときだけ。 | `.claude/skills/aidlc/SKILL.md:178` |
| `aidlc-jump execute` | 配線済み | — | engine が print directive で綴ったジャンプを逐語で実行するときだけ。 | `.claude/aidlc-common/protocols/stage-protocol-construction.md:148` |
| `aidlc-learnings persist` | 配線済み | ✓ | 学びを確定したとき。候補ゼロでも儀式自体は走る。 | `.claude/aidlc-common/protocols/stage-protocol.md:1101` |
| `aidlc-learnings surface` | 配線済み | ✓ | 各ステージ末の学びの儀式 (§13)。 | `.claude/aidlc-common/protocols/stage-protocol.md:1091` |
| `aidlc-log answer` | 配線済み | ✓ | ゲート以外の回答を受けた直後。 | `.claude/aidlc-common/protocols/stage-protocol.md:131` |
| `aidlc-log decision` | 配線済み | ✓ | ゲート以外の質問を提示する前。 | `.claude/aidlc-common/protocols/stage-protocol.md:131` |
| `aidlc-log link` | 配線済み | ✓ | reverse-engineering の委譲引継ぎ完了。 | `.claude/aidlc-common/protocols/stage-protocol-ensemble.md:126` |
| `aidlc-log review` | 配線済み | ✓ | requirements-analysis と code-generation のレビュー受領。 | `.claude/aidlc-common/protocols/stage-protocol-reviewer.md:108` |
| `aidlc-orchestrate --doctor` | 配線済み | — | 利用者が自己診断を求めたときだけ (C7)。 | `.claude/aidlc-common/protocols/stage-protocol.md:65` |
| `aidlc-orchestrate continue` | 配線済み | — | 継続トークンで同じターンを続けるときだけ。 | `.claude/skills/aidlc/SKILL.md:82` |
| `aidlc-orchestrate next` | 配線済み | ✓ | すべてのステージ境界で次の手を引く。 | `.claude/aidlc-common/protocols/stage-protocol-construction.md:146` |
| `aidlc-orchestrate park` | 配線済み | — | 作業を中断するときだけ。 | `.claude/skills/aidlc/SKILL.md:95` |
| `aidlc-orchestrate report` | 配線済み | ✓ | ゲート提示・承認・完了のたび。 | `.claude/aidlc-common/protocols/stage-protocol.md:246` |
| `aidlc-review-brief context` | 配布 TS のまま | ✓ | レビューアへ読取り範囲を渡す。C6 の読取り専用候補。 | `.claude/aidlc-common/protocols/stage-protocol-reviewer.md:61` |
| `aidlc-review-brief review` | 配布 TS のまま | ✓ | レビュー結果の整形。C6 の読取り専用候補。 | `.claude/aidlc-common/protocols/stage-protocol-reviewer.md:205` |
| `aidlc-review-brief summary` | 配布 TS のまま | ✓ | 要約確認の受領前に決定要旨を出す。C6 の読取り専用候補。 | `.claude/aidlc-common/protocols/stage-protocol.md:397` |
| `aidlc-state lookup` | 未配線 | ✓ | ステージの phase / 次ステージ / 担当を引く読取り。 | `.claude/aidlc-common/protocols/stage-protocol.md:681` |
| `aidlc-state practices-event` | 未配線 | — | practices-discovery の override のみ。bugfix はこのステージを持たない。 | `.claude/aidlc-common/conductor.md:130` |
| `aidlc-state reuse-artifact` | 未配線 | — | 既存の走査結果を再利用するときだけ。 | `.claude/aidlc-common/protocols/stage-protocol.md:1161` |
| `aidlc-state set` | 未配線 | — | 禁止 — engine 内部用。ステージ本文は report / scope-change を使う。 | `.claude/aidlc-common/protocols/stage-protocol.md:638` |
| `aidlc-state set-construction-iteration` | 未配線 | — | Unit を持つ Construction のみ。bugfix のゼロ Unit 経路では踏まない。 | `.claude/aidlc-common/protocols/stage-protocol-construction.md:310` |
| `aidlc-state skip` | 未配線 | — | recovery 経路のみ。 | `.claude/aidlc-common/protocols/stage-protocol-recovery.md:149` |
| `aidlc-state unit` | 未配線 | — | Unit を持つ Construction のみ。bugfix のゼロ Unit 経路では踏まない。 | `.claude/aidlc-common/protocols/stage-protocol-construction.md:302` |
| `aidlc-state unpark` | 未配線 | — | 委譲エージェントへの禁止記述としてだけ現れる。 | `.claude/aidlc-common/protocols/stage-protocol.md:821` |
| `aidlc-swarm prepare` | 未配線 | — | 並行レビュー/並行 Construction のみ。bugfix では踏まない。 | `.claude/aidlc-common/protocols/stage-protocol-reviewer.md:287` |
| `aidlc-testing-posture fingerprint` | 配線済み | ✓ | code-generation が Testing Contract の指紋を取る。 | `.claude/aidlc-common/stages/construction/code-generation.md:210` |
| `aidlc-testing-posture render` | 配線済み | ✓ | code-generation が Testing Contract を描く。 | `.claude/aidlc-common/stages/construction/code-generation.md:152` |
| `aidlc-unit claim` | 未配線 | — | Team Unit 経路のみ。bugfix のゼロ Unit 経路では踏まない。 | `.claude/aidlc-common/protocols/stage-protocol-construction.md:342` |
| `aidlc-unit participate` | 未配線 | — | 同上。 | `.claude/aidlc-common/protocols/stage-protocol-construction.md:348` |
| `aidlc-unit release` | 未配線 | — | 同上。 | `.claude/aidlc-common/protocols/stage-protocol-construction.md:351` |
| `aidlc-utility codekb-path` | 未配線 | ✓ | reverse-engineering / requirements-analysis が保管先を解決する。 | `.claude/aidlc-common/stages/inception/reverse-engineering.md:321` |
| `aidlc-utility codekb-publish` | 未配線 | ✓ | reverse-engineering が走査結果を保管する。 | `.claude/aidlc-common/stages/inception/reverse-engineering.md:353` |
| `aidlc-utility codekb-scope-diff` | 未配線 | ✓ | reverse-engineering が対象リポジトリごとに読取り検査を行う。 | `.claude/aidlc-common/stages/inception/reverse-engineering.md:308` |
| `aidlc-utility codekb-snapshot` | 未配線 | ✓ | reverse-engineering が走査前に compare-and-swap の写しを取る。 | `.claude/aidlc-common/stages/inception/reverse-engineering.md:165` |
| `aidlc-utility document-input` | 未配線 | — | 依頼が外部文書を指したときだけ。 | `.claude/aidlc-common/stages/inception/requirements-analysis.md:86` |
| `aidlc-utility help` | 未配線 | — | 利用者がスコープ一覧を照会したときだけ。 | `.claude/skills/aidlc/SKILL.md:7` |
| `aidlc-utility intent` | 未配線 | — | 作業記録の一覧を照会したときだけ。 | `.claude/skills/aidlc/SKILL.md:145` |
| `aidlc-utility intent-create` | 配線済み | ✓ | 新しい作業記録を作るとき。計画したスモークは新規 intent で始める。 | `.claude/aidlc-common/stages/initialization/workspace-scaffold.md:102` |
| `aidlc-utility project-description` | 未配線 | ✓ | requirements-analysis が依頼原文を逐語で取るための固定コマンド。 | `.claude/aidlc-common/stages/inception/requirements-analysis.md:63` |
| `aidlc-utility recompose` | 未配線 | — | 計画を組み直すときだけ。 | `.claude/aidlc-common/stages/inception/requirements-analysis.md:229` |
| `aidlc-utility scope-change` | 未配線 | — | スコープを変えるときだけ。 | `.claude/aidlc-common/protocols/stage-protocol.md:639` |
| `aidlc-utility scope-table` | 未配線 | ✓ | state-init がスコープ別の実行数を引く。 | `.claude/aidlc-common/stages/initialization/state-init.md:58` |
| `aidlc-utility stage-table` | 未配線 | ✓ | state-init がコンパイル済みステージ一覧を引く。 | `.claude/aidlc-common/stages/initialization/state-init.md:59` |
| `aidlc-worktree info` | 未配線 | — | Bolt worktree を使うときだけ。bugfix では踏まない。 | `.claude/aidlc-common/protocols/stage-protocol-construction.md:93` |

## 受領記録

bugfix 一周が作る受領記録と、その所有者は次のとおりである。

| 受領記録 | 所有する入口 | この build |
| --- | --- | --- |
| ゲート提示・承認・却下 (`STAGE_AWAITING_APPROVAL` / `GATE_APPROVED` / `GATE_REJECTED`) | `aidlc-orchestrate report` | 配線済み |
| ゲート以外の質問と回答 (`QUESTION_ANSWERED`) | `aidlc-log decision` / `answer` | 配線済み |
| 要約確認 (`--checkpoint summary-confirmation`) | `aidlc-log decision` / `answer` | 配線済み |
| レビュー受領 (`REVIEW_REQUESTED` / `REVIEW_COMPLETED`) | `aidlc-log review` | 配線済み |
| 引継ぎ受領 (`PIPELINE_LINK_COMPLETED`) | `aidlc-log link` | 配線済み |
| 人間の応答 (`HUMAN_TURN`) | フック `record-human-turn` | native 面にある |
| 成果物の作成・更新 (`ARTIFACT_CREATED` / `ARTIFACT_UPDATED`) | フック `write-audit-log` | native 面にある |
| 成果物の再利用 (`ARTIFACT_REUSED`) | `aidlc-state reuse-artifact` | 未配線 |
| 計画承認 (Plan Approval) | フック `plan-approval-guard` と `aidlc-testing-posture begin` | フックは配布 TS のまま。`begin` は native にあるが、ステージ本文は配布 TS を呼ぶ |

`aidlc-audit append` は列挙に現れるが、4 箇所とも**禁止の記述**であり呼出しではない (「Do NOT call `aidlc-audit.ts append` separately」ほか)。権限を持つ受領は本家側でも拒否される。禁止を呼出しと数えないことは `required_surface_contract` の `usage` 欄で固定した。

## 限界

- ここで確かめたのは**入口の受理/拒否**であって、受理された動詞が本家と同じ観測を出すことではない。それは既存の本家契約テストの担当である。
- 条件付きの入口は、計画したスモークの経路に基づく判断である。実際のスモークで別の分岐へ入れば、踏む集合は変わる。
- この列挙の成功は、実地スモーク (FR7)・切替 (FR8) の達成証拠にならない。
