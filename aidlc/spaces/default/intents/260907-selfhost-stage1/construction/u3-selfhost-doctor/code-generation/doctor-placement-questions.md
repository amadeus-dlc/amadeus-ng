# 診断の判定の置き場所と D3 の対象集合（裁定）

2026-09-12。担当 `u3_doctor` の [implementation-verification.md](implementation-verification.md) §5 の 2・12 を裁定に付した。

## Q1: D3.c / D3.d の対象集合

[Question]: 循環・孤児・schema・参照の検査対象は、選定 2 スコープが使うコンパイル済みグラフ全体（33 ステージ、本家採取とバイト一致）か、2 スコープの EXECUTE 部分集合か？

- A. コンパイル済みグラフ全体（本家採取とバイト一致する現状）
- B. 2 スコープの EXECUTE 部分集合
- X. Other (please specify)

[Answer]: A. コンパイル済みグラフ全体 (推奨)

## Q2: 判定（✓ / ✗ を決める計算）の置き場所

[Question]: 承認済み計画は D1–D5 の判定を Query use case に置いたが、`coding-rules/cqrs-boundaries.md` の 2026-09-02 追記（クエリ側ユースケースは DAO で View を読んで返すだけ）と契約 C5 に反する。どこに置くか？

- A. RMU へ移す
- B. 今回は例外として認める
- X. Other (please specify)

[Answer]: X. 「doctor は、コマンド側集約が処理してイベントを吐き出し、RMU がイベントからリードモデルを作り、クエリ側がその結果を表示です。」（利用者原文）

## Q3: 初回（作業記録なし）のときのイベントの扱い

[Question]: C7 は初回にファイル・イベントを一切作らないと定める。Q2 の流れを初回でどう回すか？

- A. メモリ上で同じ流れ（集約 → イベント → RMU → クエリをメモリ上のストアで回して表示だけ行う。記録があるときは既存ストアへ追記し HEALTH_CHECKED も投影）
- B. 初回もストアへ永続化（C7 を改訂）
- X. Other (please specify)

[Answer]: A. メモリ上で同じ流れ (推奨)

## Q4: 診断イベントの置き場所（Q3 の再裁定）

2026-09-12。Q3 = A の括弧書き後半「記録があるときは既存ストアへ追記」が実装できないことが判明したため、再裁定に付した。

[Question]: 記録があるときも診断イベントを永続ストアへ書かない形（毎回プロセス内のメモリ SQLite）で確定してよいか？

判明した事実（親が現物で確認）:

- 契約テスト DC10（`modules/app/aidlc/tests/doctor_contract.rs:488`）は、空間ストアの `journal` に `CREATE TRIGGER fail_doctor_record BEFORE INSERT ON journal … RAISE(ABORT)` を仕込んだうえで、診断出力の全文（`34 passed, 0 failed`）・終了 1・監査シャード不変・**`journal` 行数不変**を要求する。診断集約のイベントを同じ `journal` へ追記すると、報告が組み上がる前に ABORT が飛ぶため両立しない。
- 承認済み契約 C7 の `effects` は `cold_record: no_files_or_events_created` / `initialized_record: HEALTH_CHECKED_via_U2_command_and_RMU` / `automatic_repair: forbidden` と定める。永続する事実は `HEALTH_CHECKED` だけである。
- 残る候補の runtime ストア（`aidlc/.aidlc-runtime.sqlite`）は、不在時に `--doctor` が新しい永続ファイルを作るため `automatic_repair: forbidden` と DC1 に触れる。

したがって Q3 = A の括弧書き後半は、選択肢を書いた時点の誤りである。Q2 の本題（判定はコマンド側集約が持つ）は影響を受けない。

- A. 現状を追認し、未使用の口を撤去
- B. 現状を追認するが `open` は残す
- C. 永続化する形へ変える（C7 の `effects` と DC10 の改訂が必要）
- X. Other (please specify)

[Answer]: A. 現状を追認し、未使用の口を撤去 (推奨)

帰結: 「集約 → イベント → RMU → クエリを毎回プロセス内のメモリ SQLite で回し、永続する事実は `HEALTH_CHECKED` 1 件だけ」を確定仕様とする。呼び手の無くなった `WorkspaceDoctorRepositoryImpl::open(&StorePath)` は `no-backward-compatibility.md` に従って撤去し、その契約テストは `open_ephemeral()` へ移す。
