<!-- INVARIANT: examples are single-line HTML comments so a fresh template parses to total=0 (MEMORY_EMPTY). Do NOT un-comment or split across lines. t100 guards this. -->
> This file is kept up to date automatically while the stage runs. Add observations at the review step, not by editing here directly.

## Interpretations
<!-- example: 2026-05-29T10:14:32Z — chose REST over GraphQL; the consuming team only needs CRUD, revisit if subscriptions land -->
- 2026-08-22T10:44:29Z — ステージ定義の契約形式（OpenAPI/AsyncAPI/shared-schema）はプロセス内 CLI に直接は合わないため、CLI 面は yaml の自前表、trait は fenced rust、イベントは AsyncAPI 風 yaml、スキーマは SQL DDL とした; 正本の所在（upstream 仕様・ゴールデン・trait）を契約ごとに明記して形式の差を補った
- 2026-08-22T10:44:29Z — Q1 = A『CLI 面を唯一の外部契約』を、ハーネスがバイナリ実行から観測できるもの全体（stdout/終了コード + 動詞が書く upstream 互換ファイルの形式）と解釈した; NFR1 の D6 範囲（ワークスペースレイアウト・監査語彙）と一致させるため。SQLite ファイルと内部ポートは外部契約に含めない

## Deviations
<!-- example: 2026-05-29T10:14:32Z — skipped the optional caching layer the stage prose suggested; the dataset is small enough that it adds risk -->
- 2026-08-22T10:40:29Z — Q7 の選択肢ラベルを「U2 / U3 / U4 / U7 / DIP」と記号だけで書いてオーナーから差し戻された（「記号だけ書かれても意味不明。括弧書き付けろ。モバイルだと不明なのだ」）; 選択肢の description はモバイルでは見えないため、ラベル自体に各記号の意味を括弧書きで添えて再提示した。既存の『術語に平易な言い換えを添える』ルールは ID・記号（Unit ID、略語）にも適用し、かつラベル側に書く必要がある

## Tradeoffs
<!-- example: 2026-05-29T10:14:32Z — picked TDD over BDD this run; the team is unit-first and the domain is well-understood -->
- 2026-08-22T10:44:29Z — JournalReader（投影の差分読取・チェックポイント）を EventStore とは別 trait にした; event-store-adapter-rs 同形 trait に読取側を混ぜると本家同形性が崩れるため。所有は U3

## Open questions
<!-- example: 2026-05-29T10:14:32Z — confirm the retention window with compliance before the next stage hardens the schema -->
- 2026-08-22T10:44:29Z — contract-summary §4 の 7 項目（ストアファイル配置・競合文言・フック stdin スキーマ・EventStore ジェネリクス・Started/GateApproved の行順・Jumped ペイロード・projection 名）は functional-design（3.1）で各 Unit が確定する
