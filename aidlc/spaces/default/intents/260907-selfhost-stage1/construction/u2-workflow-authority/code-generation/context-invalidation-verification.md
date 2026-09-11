# PreCompactによる実装計画承認の文脈失効

## 対象と根拠

承認済みU2計画Step 5・7、FR2–FR4、C2–C4のうち、PreCompactの承認失効だけを扱う。Testing Contractは `sha256:d904f82d20fc4ba2d0045d5697ecae08fac96371ae5ab4ab6ec4913ae045ab55`。

固定本家 `a277af218f0df7f325d3b8be7b6d90fce2c5bd40` の `core/tools/aidlc-lib.ts` にある `invalidateActiveDirectiveContext` は、空でないsession、markerの所有session、project、intent、stateの完全一致を条件にする。一致時だけrevisionとcontext_epochを1増やし、kindをerror、deliveryをsuperseded、needs_rehydrateをtrueにする。継続トークンを取り除き、共有の計画承認をリセットする。元の `code_generation_authority_revision` は変えない。

同じ本家の `freshActiveDirectiveMarker` は、通常Claudeの初期所有者を `sessionless:<project hashの先頭16文字>` とする。今回、Copilot用の所有権取得処理をClaudeへ追加していない。通常のsessionがこの所有者と異なるPreCompactは無変更である。

## 実装

- `ActiveDirective` にowner_session、owner_epoch、context_epoch、issuance_revisionを完全な構築材料として保持した。書込みDTOとRMU専用DTOはそれぞれ独立して保存・復元する。
- `CodeGenerationAuthority` は固定の0ではなく、保存されたowner/context世代を指紋へ含める。失効後のerror指示は承認を発行できない。
- `DirectiveContextInvalidation` は観測した対象と所有者だけを運ぶ値オブジェクトである。`IntentExecution::invalidate_directive_context` が一致を検査し、単一の `DirectiveContextInvalidated` イベントを生成する。公開監査イベントは追加していない。
- `InvalidateDirectiveContextUseCase` は成功時にunitを返す。不一致では両ストアへ保存しない。一致時は既存の共有失効準備、元実行の保存、共有失効の確定を使う。中断時は既存の `RecoverPlanInvalidationUseCase` が、元実行に操作ID付きの失効事実があるかに応じて回復する。
- RMUは失効イベントから公開markerを生成し、共有側のRMUは操作結果を既存の読取り表へ投影する。操作ID指定のQueryで結果を取得する。
- `runtime/plan_approval.rs` の `invalidate_context(layout, execution, intent, session, state)` が接続口。hook-healthだけの初期ストアから計画承認を新設しない。sessionフック本体は別担当の変更で接続する。

## Red → Green

構文・型・ビルド競合による失敗はRedに数えていない。以下はアサートまで実行された失敗である。

| 振る舞い | Redの実行と観測 | Green |
| --- | --- | --- |
| 所有者と文脈世代の保存 | `cargo test -p core-command-interface-adapter --test directive_context_contract`。復元後のowner_sessionが `Null`、期待は `owner-session`。1件失敗、exit 101 | 完全な状態と両DTOへフィールドを追加。1件成功 |
| 文脈・所有世代の承認指紋への反映 | 同コマンド。context_epoch 0と1で同一の `sha256:5aa1f138d5b0fcf7b91af87b92fcfd13418479e8efb0bed11d11ac01e5bf82bf` となった。1件成功・1件失敗、exit 101 | 固定0を保存済み世代に置換。2件成功 |
| 失効後も元の発行回を保持 | `cargo test -p aidlc --test directive_context_contract`。SQLiteから読んだ失効イベントのRMU投影でauthority revisionが2、期待は元の1。1件失敗、exit 101 | issuance_revisionを独立保持し、失効後の投影で維持 |

## 検証結果

- `cargo test -p aidlc --test directive_context_contract`：3件成功。実SQLite→RMU→操作IDQuery、別session・空session・別project・別intent・古いstate・指示不在の6条件の無変更、失効事実の保存前後の2回復分岐。
- `cargo test -p core-command-interface-adapter --test directive_context_contract`：4件成功。保存、指紋、継続トークン消去と反復失効・再発行、revision/context世代枯渇時の無変更。
- `cargo test -p core-command-domain --test plan_authority_contract`：16件成功。固定本家のchallenge指紋を維持。
- `cargo test -p core-command-domain --lib orchestration::intent_execution_event::tests::`：7件成功。26変種の網羅性とイベントIDを検証。
- `cargo lint`：成功。途中で指摘されたuse-caseからのoperation_id getter取得は、操作IDを独立したexecute引数へ分けて是正した。

Clippyで新規試験のJSON直列化・添字アクセスも是正した。契約JSONはcanon_json経路、アクセスはget/insertを使用する。共有テスト補助のexpectは既存のテスト規約に合わせて許容する。全体Clippyは並行作業中の別領域の指摘を含むため、最終統合で再確認する。

## 残る検証

session担当の公開CLI契約と、B1全体のfmt・Clippy・workspace・90%床・相対ゲートは親作業で統合する。今回の試験で本物のユーザー回答・承認・監査を作成していない。実地Claudeスモークと切替の成功を意味しない。
