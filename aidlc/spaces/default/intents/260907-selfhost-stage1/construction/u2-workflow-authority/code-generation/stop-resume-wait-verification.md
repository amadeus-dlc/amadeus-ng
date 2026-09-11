# 共有の再開待ちmarkerの読取り

## 責任と実装

`harness_claude::StopResumeWait::parse(&str)` と `matches_state_hash(&str)` を追加した。本家のv2 marker構造を検査したうえで、`owner_session` が `sessionless:` で始まり、kindがask、resume.statusがwaitingで、現在の状態SHA-256に一致する場合だけ待機を認める。

必須項目、非負整数、型、状態/コマンドのハッシュ、任意のUnit一覧・実行対象・試行情報、継続トークンのバイト上限と本文ハッシュを確認する。本家が許す型変換と、文字列でなければならない欄を区別した。ここではファイルIO、対象project/intentの選択、ロックと読取順、自律実行の方針は処理しない。呼出側がその条件を成立させたうえで観測を使う。

## 固定本家との照合

固定コミット `a277af218f0df7f325d3b8be7b6d90fce2c5bd40` の `parseActiveDirectiveMarker` と `stateContentSha256` を、本文を変更せずexportだけ追加して実行した。`hasCurrentSharedResumeWait` の待機式も原文から抽出して実行し、状態は非自律に固定した。IOやロックを検証したとは扱わない。

127入力のmarker全文、状態本文、そのSHA-256、本家の判定は `tests/golden/selfhost-stage1/stop-resume-wait.json` に保存した。入力には正常・欠落・型不正・境界値・別状態・単位名・任意情報・トークンの長さ/ハッシュ不一致を含む。

構造検査の初期状態ではvalid-shared-waitがfalse対trueとなるRedを確認し、実装後に127入力すべてが一致した。

## 最終検証

- `cargo test -p harness-claude --test stop_resume_wait_contract`: 127観測を含む契約テスト成功。
- `cargo test -p harness-claude`: 全テスト成功。
- `cargo clippy -p harness-claude --all-targets -- -D warnings`: 終了0。
- `cargo lint`、対象rustfmt、git diff検査: 終了0。

生ログは `test-evidence/stop-resume-wait-*.txt`。本体へ組み込んだ場合の保存・投影・同一状態の読取りと、Step 6全体の完了は開発担当の結合検証へ引き継ぐ。

## 変更ファイル

- `modules/harness/claude/src/lib.rs`
- `modules/harness/claude/src/stop_resume_wait.rs`
- `modules/harness/claude/tests/stop_resume_wait_contract.rs`
- `scripts/goldens/capture-stop-resume-wait.ts`
- `tests/golden/selfhost-stage1/stop-resume-wait.json`
