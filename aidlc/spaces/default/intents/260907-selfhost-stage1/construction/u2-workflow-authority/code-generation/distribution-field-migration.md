# 配布データの保持と受入移行

## 検出した差と修正

承認済み計画Step 8の配布データ比較を、本家2.7.1の `tests/golden/upstream-a277af21/data/` へ移した。固定データでは33ステージ・11スコープ・レビュー宣言13件（adversarial 5件、advisory 8件）。旧版と異なる実行工程数はbugfixが9、refactorが10である。

旧版を読み続けた状態では、bugfixの7件と期待9件の不一致でテストが失敗した。読取り先を2.7.1へ移すと10件成功・1件失敗になり、配布グラフの書戻しで `review_artifact` とセンサー参照の `fire_on` / `default_severity` / `category` が落ちることを検出した。出力81,955バイトに対して原本は91,041バイトだった。期待値や原本を変更せず、ドメイン、取込・書戻しDTO、保存DTO、RMUの読込みDTOで保持するフィールドを加えた。センサーの発火・判定機能は実装していない。

`SensorRef` は追加3フィールドを含む完全コンストラクタで構築し、既存呼出しも明示的な値へ変更した。省略された新しい任意項目は保存時にも省略し、既存行へ不要なnullを増やさない。内容識別子の計算にも追加項目を含める。

## 検証の経過

- 移行前の配布往復テスト: 11件成功。
- 2.7.1の実行工程数を期待したRed: bugfix 7対9で失敗。
- 新配布への切替後のRed: `stage-graph.json` の先頭不一致は `review_artifact` の欠落。10件成功・1件失敗。
- フィールド保持後: 配布往復11件成功。グラフ91,041バイトとグリッドを原本と全文比較した。
- 内容識別子: 追加項目のハッシュ入力を一旦加える前の状態に戻し、項目変更が同じ識別子になるRedを記録してから追加。4項目それぞれの変化を検出するテストを含め、定義関連166件成功。
- 保存DTO: 任意項目を埋めた往復を含む6件成功。
- RMU DTOの初回回帰: 14件成功・1件失敗。新しい未指定項目のnull追加を検出し、省略を維持するよう是正した。後続の再実行は並行実装中の停止フックのコンパイルエラーで未完了。

停止フックの実装中に生じたビルドエラーは、この差分のRedとして数えていない。U2全体やCIが成功したという証拠ではない。

## 最終検証

- `cargo test -p core-command-interface-adapter --test golden_parity_test`: 11件成功。
- `cargo test -p core-command-domain --lib workflow_definition`: 166件成功（ほか546件はフィルタ除外）。
- `cargo test -p core-command-interface-adapter --lib workflow_definition_dto`: 6件成功（ほか94件はフィルタ除外）。
- `cargo test -p core-read-model-updater --lib definition_content_dto`: 15件成功（ほか293件はフィルタ除外）。
- `cargo test -p core-read-model-updater --test read_tables_test`: 39件成功。
- `cargo clippy -p core-command-interface-adapter --all-targets -- -D warnings` と `cargo lint`: 終了0。
- 以下の11ファイルへの `rustfmt --edition 2024 --check` と `git diff --check`: 終了0。

上記検査はいずれも失敗0。証跡は `test-evidence/distribution-field-*.txt` に保存した。引き続き停止フック等のU2実装があるため、最終のworkspace全体・カバレッジ・CIは別途確認する。

## 変更ファイル

- `modules/core/command/domain/src/workflow_definition/stage_node.rs`
- `modules/core/command/domain/src/workflow_definition/stage_node/stage_node_builder.rs`
- `modules/core/command/domain/src/workflow_definition/sensor_ref.rs`
- `modules/core/command/domain/src/workflow_definition/definition_revision.rs`
- `modules/core/command/interface-adapter/src/orchestration/compiled_definition_repository_impl.rs`
- `modules/core/command/interface-adapter/src/orchestration/dto/workflow_definition_dto.rs`
- `modules/core/command/interface-adapter/tests/golden_parity_test.rs`
- `modules/core/read-model-updater/src/orchestration/dto/definition_content_dto.rs`
- `modules/core/read-model-updater/src/orchestration/dto/definition_dto_tests.rs`
- `modules/core/read-model-updater/src/read_tables/json_column.rs`
- `modules/core/read-model-updater/tests/read_tables_test.rs`
