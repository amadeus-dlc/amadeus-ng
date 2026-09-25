# Requirements Analysis — 確認したいこと

Issue #134（並行した pipeline link の完了報告で、記録に成功した側も WouldBlock で失敗として返る）の要件を確定するための質問です。期待する振る舞いと受入条件は Issue 本文にすでにあるので、そこから決まらない 3 点だけを尋ねます。

背景（Reverse Engineering の結果）:

- 記録に成功した起動は、その後の投影（リードモデルの更新）で SQLite のロック競合に当たり、終了コード 1 で失敗を返している。
- 最有力の原因は `journal_reader_impl.rs:1105` の `replace_pipeline`。同じファイルのほかの書き込みはすべて最初から書込ロックを取る（IMMEDIATE）が、ここだけは読んでから書込へ切り替える（DEFERRED）。SQLite はこの切り替えで競合すると待たずに即失敗するので、5 秒の待ち時間が効かない（SQLite 単体では実測済み。アプリ本体での再現はまだ）。
- 恒常的な投影失敗では「記録済みでも失敗を返し、受領は残す」ことを既存の契約テスト 2 本が固定している。直すのは一時的なロック競合の扱いだけに限る必要がある。

## Question 1: 修正の方針

記録に成功した起動が、投影の一時的なロック競合で失敗を返さないようにする方法です。どこまで手を入れますか？

A. `replace_pipeline` をほかの書き込みと同じ IMMEDIATE に揃えるだけにする（最小の変更。5 秒の待ち時間が効くようになる。5 秒を超えてロックが握られた場合は従来どおり失敗）
B. 投影がロック競合（WouldBlock）で失敗したときだけ、投影を再試行する仕組みを足す（`replace_pipeline` は DEFERRED のまま）
C. A と B の両方（IMMEDIATE に揃えたうえで、それでも残る一時的な競合を再試行で吸収する）
X. Other (please specify)

[Answer]: A

## Question 2: 同じ形の箇所をどこまで直すか

RMU には `replace_pipeline` と同じ「読んでから書込へ切り替える」トランザクションが、あと 2 か所あります（`hook_health_reader.rs:181` と `workspace_doctor_read_model_updater.rs:183`）。どちらも link の経路には乗りませんが、ロック競合を「壊れている」（Corrupt）と誤って分類します。また、投影の失敗文言はどの段で失敗したかを運びません（`read: io: WouldBlock at <path>` だけ）。

A. `replace_pipeline` だけを直す。残りは Issue に記録して別に扱う（bugfix の射程を広げない）
B. `replace_pipeline` に加え、同じ形の 2 か所も IMMEDIATE に揃える
C. B に加え、投影の失敗文言にどの段で失敗したかを含める
X. Other (please specify)

[Answer]: A

## Question 3: 再現テストの置き場

受入条件は「ロック競合を確実に起こす再現テストを加え、修正前に失敗し、修正後に通ること」です。プロセスを 2 つ走らせる既存の契約テストはタイミング次第でしか落ちないので、確実に起こす形が要ります。

A. RMU 層の単体テストで再現する（別の接続が書込ロックを握っている間に `replace_pipeline` を呼び、修正前は即 WouldBlock、修正後は待って成功することを確かめる。既存の `a_write_lock_held_by_another_connection_is_reported_as_would_block` と同じ作り）
B. A に加え、CLI の起動を跨ぐ契約テストでも、ロックを外から握って確実に再現する形を足す
C. CLI の起動を跨ぐ契約テストだけで再現する
X. Other (please specify)

[Answer]: A

## Consolidated Summary Confirmation

- 修正の方針（Q1）: `replace_pipeline` をほかの書き込みと同じ IMMEDIATE に揃えるだけにする。5 秒を超えてロックが握られた場合は従来どおり失敗を返す。
- 直す範囲（Q2）: `replace_pipeline` だけを直す。同じ形の 2 か所（`hook_health_reader.rs:181`、`workspace_doctor_read_model_updater.rs:183`）と、失敗文言に段を含める改善は Issue に記録して別に扱う。
- 再現テスト（Q3）: RMU 層の単体テストで、別の接続が書込ロックを握っている間に `replace_pipeline` を呼び、修正前は即 WouldBlock、修正後は待って成功することを確かめる。

Does this all look correct before I generate the requirements artifact?

- Looks correct
- Request changes

[Answer]: Looks correct
