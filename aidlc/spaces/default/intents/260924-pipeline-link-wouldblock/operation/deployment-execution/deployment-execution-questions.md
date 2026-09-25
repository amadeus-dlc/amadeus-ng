# Deployment Execution — 確認したいこと

PR を出して CI を確かめる前の確認です。改訂 1（再現テストの同期を強めた変更）を PR #154 に載せる 2 回目のために、前提を更新し Question 3 を足しました。

事前の確認のうち、すでに答えが出ているもの（ここでは尋ねない）:

- **事前チェック**: ビルド・fmt・clippy・lint・修正の単体テストは緑。ワークスペース全体のテストとカバレッジは、macOS ローカルの SIGKILL のため未確定のまま、Build and Test で `Accept failure` を選んで先へ進めた（`construction/build-and-test/test-results.md`）。
- **DB マイグレーション**: 無い。表の DDL は変えていない。
- **依存するサービス**: 無い。GitHub（PR・Actions）だけを使う。
- **デプロイの時間帯**: 決まりは無い。~~merge queue へ入れる時期はオーナーが決める。~~ → 2026-09-24 に利用者の指示「CI greenならマージいいよ。タイミング任せる」で改めた。PR の必須チェックが緑になったら、進行役が自分の判断したタイミングで merge queue へ入れる（`deployment-pipeline-questions.md` の Q2）。
- **改訂 1 の届け方**: PR #154 の同じブランチに追加のコミットとして積んで push する。force push はしない（`deployment-pipeline-questions.md` の Q3）。

## Question 1: PR と Issue #134 の結び付け

A. PR 本文に `Closes #134` と書く。マージで #134 が自動で閉じる
B. PR 本文には `Refs #134` とだけ書き、#134 は開いたままにする。main の CI で再発しないことを見てから、オーナーが手で閉じる
X. Other (please specify)

[Answer]: A

## Question 2: PR の CI が、#134 とは別の症状で落ちたとき

たとえば、子プロセスの SIGKILL やタイムアウトで CI が落ちた場合です。

A. 落ちたジョブと失敗の形を報告して止まる。再実行するかはオーナーが決める
B. 失敗の形を記録したうえで、落ちたジョブだけを 1 回だけ再実行し（`gh run rerun --failed`）、結果を報告する
X. Other (please specify)

[Answer]: A

## Question 3: PR #154 に残っているレビューの指摘（CodeRabbit）への対応

CodeRabbit の指摘 1 件（再現テストのホルダが、主スレッドが `replace_pipeline` を呼ぶ前にロックを放しうる）が未解決のままで、CI のレビューのやり取りのゲートはこれが解決されるまで通りません。改訂 1 がこの指摘への対応です。

A. 改訂 1 を push した後、指摘のスレッドに対応したコミットと直し方を返信し、私がスレッドを resolve する
B. 改訂 1 を push した後、指摘のスレッドに返信だけして、CodeRabbit の再レビューを待つ。自動で解決されなければ報告して止まる
X. Other (please specify)

[Answer]: A

## Consolidated Summary Confirmation

- PR と #134 の結び付け（Q1）: PR 本文に `Closes #134` と書き、マージで #134 を自動で閉じる。
- CI が #134 とは別の症状で落ちたとき（Q2）: 落ちたジョブと失敗の形を報告して止まる。再実行するかはオーナーが決める。
- レビューの指摘への対応（Q3）: 改訂 1 を push した後、指摘のスレッドに対応したコミットと直し方を返信し、進行役がスレッドを resolve する。
- 決定済みの前提: DB マイグレーションは無し、依存するサービスは無し、時間帯の決まりは無し。PR に入れるのはコード 1 ファイルだけ。改訂 1 は同じブランチに追加のコミットとして積む。必須チェックが緑になったら、進行役が merge queue へ入れる。

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct
