# セルフホスト切替の要求確認

## 前提と回答方法

依頼原文、承認済み計画、共有設定へ反映された開発規則を引き継ぐ。TDD、直列統合、独立した先行実装を設けないこと、実地スモークと自己診断、CI全ジョブ成功、日本語、配布資産再利用は再質問しない。

回答はこの場で案内し、画面の番号だけでも受け付ける。質問ごとに番号を1から付け直し、回答欄には該当する意味上の選択肢を記録する。

## Q1: 互換性の受入基準をどの版に揃えるか

ゴールデン（比較用に採取した正解データ）の基準を、現在使う配布資産のコミットへ更新しますか。

実測基準: HEADとmainはともに `1dc727e00a26c27258d916cb3a5e0592c6c48c1c`。配布元は `801c570062f67dc8f4952ee5fc601381d09db7ec`、`AIDLC_VERSION` は `2.7.1-j5ik2o.1`。ゴールデンの来歴は `3c3146cfd7cef33020d48e8d48d4e80d0f8c2820`、`v2.6.40` を示す。

| 対象 | stage-graph.json のMD5 |
| --- | --- |
| 本リポジトリのClaude配布資産 | `38310798f41e8173e6e27b9d3078e840` |
| 配布元801c5700のClaude配布資産 | `38310798f41e8173e6e27b9d3078e840` |
| 現在の3c3146cfゴールデン | `3ee59d7a177bd55d2e8392fb9028561d` |

- A. forkの801c5700へ再採取する（推奨） — 配布資産と受入基準を同じ固定コミットに揃える。既存の採取済みバイトは編集せず、新しいコミットのディレクトリへ採取し、差分を全数審査する。観測差を都合よく正規化して消さない。
- B. upstreamの3c3146cfに据え置く — 現行の2.6.40基準を維持する。2.7.1系の配布資産と組み合わせる際の適合条件を、この後に具体化して裁定する。
- X. Other (please specify)

[Answer]: X. 本家2.7.1を前提にする。2.6.40への据え置きとして扱わない。

**User Input**: 独自フォーク版のvendor/aidlc-workflowsからインストールされていますが、kimi-code用のハーネスが入っているだけで本家と違いがありませんので、2で問題ないと思いますよ

**User Correction**: 本家は2.7.1を前提にしていますが。2.6.40とは。。。なんだろそれ

**User Clarification**: 書き換えずにって、2.7.1の仕様に合致させないと意味がないから、そこだけは気を付けてね

**User Confirmation**: はい。意図が合っていますね。ありがとう。つづけて

**Mode**: guided

最初の「2」を2.6.40据え置きとして記録した解釈は、上記の明示的な訂正により撤回する。本家2.7.1を互換対象とし、vendorのforkはその配布経路として扱う。2.6.40は既存ゴールデンの採取元としてのみ記録する。fork独自変更とupstreamの版間差分は別の論点であり、配布経路がforkであることを古いゴールデンへの据え置き理由にしない。

受入データを2.7.1に整合させるため、固定コミットと必要な再採取範囲を実測して要求・実装計画へ含める。既存の採取済みバイトを編集したり、未調査の観測差を許容したりする承認には拡大しない。

本家2.7.1の固定コミットから再採取し、ゴールデンだけでなくテストの参照先・検証内容・実装も2.7.1仕様へ揃える。2.6.40の期待値を現行の受入基準として残さない。「採取済みバイトを編集しない」とは、実装の都合に合わせて本家の採取結果を手修正しないことであり、古い仕様の維持を意味しない。

この回答は基準の選択であり、未調査の観測差を許容する承認ではない。基準に依存する設計・実装・受入判定は回答後に進める。再採取作業も、必要な差分を確定してから実装計画に含める。

## 本家2.7.1と配布資産の照合

fork独自変更の直前にある本家コミットは `a277af218f0df7f325d3b8be7b6d90fce2c5bd40`。その `dist/claude/.claude/tools/aidlc-version.ts` は `AIDLC_VERSION = "2.7.1"` を宣言する。2.7.1の受入基準を固定する候補はこのコミットとし、採取来歴へ完全なSHAを記録する。

この本家コミットとfork `801c5700` の間で、Claudeの `SKILL.md`、ステージ定義、`stage-graph.json`、`scope-grid.json` に差分はなかった。共通ツールにはKimiの識別・探索・診断等の追加と版名接尾辞の変更がある。forkのファイル全体が同一バイトとはせず、Claudeの対象契約を本家2.7.1から採取して検証する。

## bugfix一周の呼出し実測

以下は配布済み `.claude/` の読取りによる手順上の列挙であり、まだバイナリで一周した結果ではない。実地スモークは引き続き必須。パスはリポジトリルート相対、行番号は調査時点のもの。

| 用途 | 呼出し・受領 | 配布手順の根拠 | 現バイナリの入口 |
| --- | --- | --- | --- |
| 作業開始・前進 | `intent-create`、`next`、`continue`、`report` | `.claude/skills/aidlc/SKILL.md:42`、`:44`、`:82`、`:164` | 配線あり。2.7.1との観測一致は再検証が必要 |
| 通常質問・内容確認 | `aidlc-log decision/answer`、`summary-confirmation` | `.claude/aidlc-common/protocols/stage-protocol.md:131`、`:427`、`:436` | `cli/request.rs:160`で`LogNotWired` |
| 解析の引継ぎ | `aidlc-log link`、`PIPELINE_LINK_COMPLETED` | `.claude/aidlc-common/stages/inception/reverse-engineering.md:245`、`:388` | `link`は未配線 |
| コード解析資料の管理 | `codekb-scope-diff`、`codekb-snapshot`、`codekb-path`、`codekb-publish` | 同`:98`、`:165`、`:308`、`:321`、`:338`、`:353` | utility面は`intent-create/init`以外を未提供 |
| 依頼原文 | `aidlc-utility project-description` | `.claude/aidlc-common/stages/inception/requirements-analysis.md:63` | 未提供。`document-input`は外部文書を指定する場合だけの条件付き操作 |
| 実装のテスト方針 | `aidlc-testing-posture render`、`fingerprint --stage-level` | `.claude/aidlc-common/stages/construction/code-generation.md:152`、`:217` | `cli/face.rs`に当該ツール面なし |
| 実装計画の承認 | `decision/answer --checkpoint plan-approval --session ... --stage-level`、保護された応答と実行許可 | 同`:237`、`:252`、`:263` | 通常の`decision/answer`を含め未配線。監査行だけで許可済みにしない |
| レビュー | `aidlc-log review`の要求・完了、`aidlc-review-brief summary/review`（再確認時は`context`） | `.claude/aidlc-common/protocols/stage-protocol-reviewer.md:29`、`:108`、`:205`、共通protocol`:397` | reviewは配線あり。review-brief面なし。2.7.1の受領契約へ適合させる |
| 学びの記録 | `aidlc-learnings surface/persist`、質問と回答 | `.claude/aidlc-common/protocols/stage-protocol.md:1091`、`:1101` | learnings面なし。persistは規則と監査の更新を伴う |
| 停止・再開・修正 | `park`、`next --resume`、`report rejected/revised`、条件付き`reuse-artifact` | `.claude/skills/aidlc/SKILL.md:95`、`:109`、共通protocol`:1161` | park/report等は配線あり、reuse-artifactは未配線。必要な分岐を一周の実測で判定 |
| 自己診断 | `aidlc --doctor`からdoctorへの経路 | 依頼の切替条件4。`modules/app/aidlc/src/cli/request.rs:3`以降に未実装文法の注記 | 実処理未提供。成功の表示だけでなく必要項目の異常検出も受入対象 |
| 主要4フック | Stop=`aidlc-continue-workflow`、人間応答=`aidlc-record-human-turn`、遷移保護=`aidlc-state-transition-guard`、保存監査=`aidlc-write-audit-log` | `.claude/settings.json`のStop/UserPromptSubmit・PostToolUse/PreToolUse設定。Stop実装冒頭は質問待ち・承認待ちの除外を規定 | `modules/harness/claude`と`modules/harness/infrastructure`に動作実装なし |

bugfixは単位分割を省くため、`code-generation.md:99`付近の「no Bolt, walking-skeleton, ladder, per-Unit receipt」に従い、通常のゼロUnit経路ではBolt操作・Unit受領・自律実行を追加しない。失敗・再開などの条件付き経路を正常経路の必須呼出しと混同しない。

`.claude/settings.json`には主要4フック以外にも、実装計画の承認保護、レビュー範囲・確定後の変更保護、開始・終了、状態同期、委任結果の記録などの登録がある。実装計画承認保護は監査も更新するため、単なる表示補助とは扱えない。センサーは明示的な対象外のままとし、他の登録についても、スモークで必要か・更新する正本は何かを確認して切替対象を決める。

## Q2: 配布補助ツールの再利用範囲

状態・監査・承認をRustで管理したうえで、表示や指紋計算などの補助処理に配布済みTypeScriptを再利用してよいですか。

たとえば`review-brief`は確認対象や指摘の表示、`testing-posture`はテスト方針の表示・指紋計算を担当する。一方、`learnings persist`や各種受領・監査を更新する処理は正本を変更するため、表示補助と同じ扱いにはできない。

- A. 正本の更新はRust、更新しない補助処理は再利用する（推奨） — 状態・監査・承認受領はRustに統一する。再利用する補助処理は読取り先と副作用を確認し、一覧と実地スモークで検証する。TypeScriptが独立に同じ正本を書き換える混在構成にはしない。
- B. スモークで呼ぶ補助処理もすべてRustへ移す — 表示・指紋計算・資料管理を含む必要な入口をRust側へ実装する。配布資産はステージ・役割・プロトコル等の定義として再利用する。
- X. Other (please specify)

[Answer]: A. 正本の更新はRust、更新しない補助処理は再利用する

**User Input**: Q2: 1, Q3: 1

一旦スコープを絞ります。

## Q3: 実地スモークでのコード解析資料

bugfix手順に必要なコード解析資料の生成を、実地スモークの範囲で認めますか。

`reverse-engineering.md:98`以降は既存ストアがない場合に初回解析へ進み、`:245`の開発者引継ぎと`:388`の最終引継ぎを記録する。`:259`以降は9種類の解析資料を要求する。現在のコード解析ストアは削除済みのため、「資料を作らず通常のbugfix手順を一周できる」とは扱えない。

- A. スモークで手順が要求する解析資料だけ生成する（推奨） — この開発計画では既存コードの再文書化を行わず、実地スモークに限って必須の解析資料を生成する。主な調査対象は小さなbugfixの関係箇所とし、浅く確認した範囲を深く解析済みとは書かない。
- B. スモークでも解析資料を生成しない — 再文書化禁止を維持する。その場合は通常のbugfix手順との不一致を未解決として残し、変更後のスモーク手順を改めて裁定する。
- X. Other (please specify)

[Answer]: A. スモークで手順が要求する解析資料だけ生成する

**User Input**: Q2: 1, Q3: 1

一旦スコープを絞ります。

範囲を絞る方針は、更新しない補助処理の再利用と、解析資料の生成を実地スモークに必要な分へ限定する選択として適用する。切替条件・完了条件の削除や、承認済み工程の追加・削除は指示されていないため変更しない。

## 調査から適用する範囲の限定

`NextTurnInput`には未接続の観測があるが、5つを一律に今回の対象にしない。Claudeの選定経路で必要なCLI・環境・カーソル観測だけを実装対象にし、Kiro専用ラッチなど他ハーネスの機能は除外する。`--doctor`の実入口が未接続である点は今回の対象。

補助処理の再利用可否はファイル単位ではなく操作単位で判定する。`aidlc-testing-posture.ts:1824`の`render`と`:1827`の`fingerprint`は表示・計算の候補であり、`:1874`の`begin`は実行許可を更新する別操作である。更新操作を同じツール名だからという理由でTypeScriptへ残さない。`review-brief`も依存先を含む副作用とRustの投影結果との読取り互換を検証してから再利用する。

自己診断は、このリポジトリのClaude上で切替条件を検査する範囲に限定する。バイナリの入口と必要フックの配線、配布グラフとbugfix/featureの資産、ワークスペースの状態・監査・永続化との整合を検査し、不足や破損を正常と表示しない。他ハーネス・センサー・プラグインや、スモークに無関係な管理機能の実装へ広げない。

## この後に確認する範囲

- 配布資産のbugfix一周で必要になる呼出し・フック・受領を引用付きで列挙し、バイナリに無い差分を特定する。
- 列挙結果に基づき、バイナリと配布補助処理の担当範囲、自己診断の必要項目、実地スモークで証明する範囲を具体化する。
- コードと上流契約の相違は個別に提示し、独断で読み替えない。

## Sources

- [desc] Initial description: [依頼原文](../../project-description.json)。`project-description`が返す原文を正本として使用。
- [scope] Workflow-selected scope: selfhost-stage1。[承認済み計画](../../initialization/state-init/workflow-plan.json)。
- [承認済み開発規則](../practices-discovery/team-practices.md)。工程承認・共有設定反映は状態と監査記録が正本。
- 配布版: `.claude/tools/aidlc-version.ts:4`、`vendor/aidlc-workflows/dist/claude/.claude/tools/aidlc-version.ts:4`。
- 採取来歴: `tests/golden/upstream-3c3146cf/cli/provenance.json`。
- 再採取規律: `tests/golden/upstream-3c3146cf/README.md` の「バイトを変更してはならない」。
- MD5は2026-09-07に本リポジトリ・配布元・ゴールデンの3ファイルから再計測。

## Consolidated Summary Confirmation

- 本家2.7.1を互換対象とする。本家の固定コミットからゴールデンを再採取し、テストの参照先・検証内容・実装を2.7.1仕様へ揃える。2.6.40の期待値を現行の受入基準として残さず、採取結果を実装に合わせて手修正しない。
- 配布はvendorのforkを継続して使用し、ステージ・役割・プロトコル・コンパイル済みグラフを再利用する。bugfix/featureの必要な資産が揃うことを確認する。
- Rustへの追加は、Claude上でbugfixを一周する際に必要な未実装部分に限る。next/reportの実経路、主要4フック、質問・回答・引継ぎ・レビューと実装計画承認の受領を、状態・監査の互換性を保って動かす。
- 状態・監査・承認受領の更新はRustに統一する。表示や指紋計算など正本を更新しない配布補助処理は、読取り互換と副作用を確認して再利用する。更新を伴う操作や実行許可の発行を、同じ補助ツールの別操作だからという理由でTypeScriptへ残さない。
- 自己診断は、今回使うバイナリ・Claudeフックの配線、bugfix/featureの配布資産、状態・監査・永続化の整合に絞る。不足・破損時の検出も検証し、未実装項目を成功扱いしない。
- 本リポジトリで、小さなbugfix intentをreleaseバイナリにより開始・質問・人間のゲート承認・完了まで通す。実フックと人の応答を使い、手作りの承認記録や保護機構の無効化で代替しない。このスモークに限り、配布手順が要求するコード解析資料を生成する。
- TDD、カバレッジ床90%と相対ゲート、Quint/ITF、既存CI全ジョブ成功を維持する。切替後は安定タグのホストと開発版を使い分ける。
- 独立した先行実装・自律実行、センサー・プラグイン・他ハーネス、配布一般化、OTel、インストーラ、既存コードの再設計・再文書化、スモークで踏まない課題は対象外。コード解析資料の生成は上記の実地スモークだけの例外とする。

Does this all look correct before I generate the requirements artifact?

- Looks correct
- Request changes

[Answer]: Looks correct
