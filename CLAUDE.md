@AGENTS.md

# Claude Code（指揮役）向けの指針

共通の規則は上の `AGENTS.md` にある。ここには、Claude Code のメインセッション（指揮役）だけが従うことを書く。
`AGENTS.md` と食い違う記述をここに書かない。

## 指揮役と作業者の分担

- 指揮役は Opus 5.5 のメインセッションが務める。要件の明確化、計画、作業の分割、ユーザーの承認の取り付け、作業どうしの調停、
  差分全体のレビュー、検証の確認、受け入れの判断、ユーザーへの報告を担う。
- 実作業は指揮役がせず、takt の作業者に任せる。委譲の手間が節約を上回る小さな作業だけは、指揮役が行ってよい。
- takt は Orca のオーケストレーションから起動する。Orca から作業者のエージェントを直接起動しない
  （`orca orchestration worker-start --agent` は使わない）。監督は指揮役が行う。
- 書き込み範囲が重ならない作業は、takt を並列で動かす。順番待ちにしない。作業ごとに Orca の worktree と端末を分ける。
- 作業どうしの調停は指揮役が行う。書き込み範囲の割り振り、マージの順番の決定、先にマージされた変更への載せ直し
  （同じ worktree で takt を起動し直させる）、衝突の解消の指示を担う。

## モデルの使い分け

takt の中のモデルは `.takt/runtime.yaml` に従う。今は、実装・修正が Claude Sonnet 5.5、レビューが別の提供元の GPT-6.1-Sol で、
作業者に Opus と Fable は使わない。指揮役の判断で `--provider`・`--model` や provider の options（effort など）を上書きしない。
使い分けを変えるときは、`.takt/runtime.yaml` の変更としてユーザーに諮る。

## ワークフロー

- 標準の `default` と、それを呼ぶ `backend-cqrs` などは、最大 51 ステップでピアレビューの収束と要件の最終確認まで回すため、
  範囲の決まった作業には重い。
- 範囲と受け入れの条件が決まった作業は、カスタムの `amadeus-small`（実装 → 独立レビュー、最大 4 ステップ）を使う。
- 計画から要る大きめの作業は、標準の `simple-mini`（計画 → 実装 → レビュー ⇄ 修正）を検討する。
  どちらも合わなければ、カスタムのワークフローを `.takt/workflows/` に作る。
- claude を読み取り専用のレビュー役にすると、cargo などの検査を実行できず `blocked` になることがある。
  その場合は指揮役が検査を実行して確かめる。

## 起動と監督の手順

1. `orca orchestration run-create` と `task-create` で、作業を Run と Task として記録する。
2. 指示書を GitHub の issue にする。git の操作（ブランチ・commit・push・PR の作成）は takt が `--pipeline --auto-pr` で行い、
   `-t` で渡した本文はそのまま PR のタイトルになる。長い指示書だと GitHub の上限（256 文字）を超えて PR を作れないので、
   `-i <issue 番号>` で渡す（PR のタイトルは issue のタイトルになる）。
3. 作業ごとに worktree を作り、その端末で takt を起動する。

   ```sh
   orca worktree create --repo id:<repo の id> --name <作業名> --base-branch origin/main --no-parent --json
   orca terminal create --worktree id:<worktree の id> --title "<作業名>" --command \
     "mise trust && scripts/run-takt.sh --config-dir <claude のアカウント> --trust-workspace -- --pipeline -q -b takt/<作業名> --auto-pr -w amadeus-small -i <issue 番号>" --json
   ```

   - `--pipeline` は今の作業ツリーでブランチを作るので、主のチェックアウトでは起動しない。`-b` に既存のブランチ名を使わない。
   - `--trust-workspace` は、新しい worktree を takt 側のアカウントで信頼済みにする。無いと、takt が起動する Claude が
     `.claude/settings.json` の `permissions.allow` を無視して失敗する。
   - `scripts/run-takt.sh` は、takt が起動する Claude を指定したアカウントで動かし、`.takt/` の読み取り制限を掛ける。
     素の `takt` を直接起動しない。アカウントの設定ディレクトリはマシンごとの事情なので、リポジトリに書かない。

4. `orca terminal wait --terminal <handle> --for exit` と `orca terminal read` で進み具合と終了を見る。途中でも、書き込み範囲を
   外れていないかを確かめる。並列で動かしている作業は、終わったものから順に確かめる。
5. 終了後、`gh pr view` と `gh pr diff` で差分を見て、worktree で `AGENTS.md` の「検査」を実行する。takt の記録（`.takt/runs/` の
   各ステップの結果・構造化出力）と、差分・ログを、指示書の受け入れの条件と照らし合わせて自分で確かめる。
6. 満たしていれば、ボットのレビューの未解決のスレッドを扱ったうえで（正しい指摘は直させ、退ける指摘は根拠を返信して解決済みにする）、
   マージキューに入れる（GraphQL の `enqueuePullRequest`）。マージは、ユーザーが承認した範囲で行う（`/goal` などで事前に承認された
   範囲を含む）。先にマージされた変更と衝突したら、同じ worktree で takt に載せ直させる。満たしていなければ、PR に指摘を書いて
   同じ worktree で takt を起動し直すか、取り込まずにユーザーに報告する。
7. 結果を `orca orchestration task-update` で Task に記録する。PR がマージ済みで、未コミットの変更と未 push のコミットが無いことを
   確かめてから、端末を `orca terminal close` で閉じ、worktree を `orca worktree rm` で消す。

## 指示書

- 指示書には、対象、変えるもの、制約、書き込み範囲、受け入れの条件、検証の手順、時間の上限と打ち切りの条件を書く。
  書き込み範囲は作業どうしで重ねない。
- takt は実行中に指揮役へ問い合わせられない。範囲の外の操作が要るときは作業者が `blocked` で止まるので、指揮役がユーザーに確認し、
  指示書を出し直す。

## マイルストーン

マイルストーンは Claude Code の `/goal` に渡す。4,000 文字以内にし、完了の条件は「セッションに何が表示されれば達成か」で書く
（達成の判定は、セッションの出力だけを見て行われる）。
