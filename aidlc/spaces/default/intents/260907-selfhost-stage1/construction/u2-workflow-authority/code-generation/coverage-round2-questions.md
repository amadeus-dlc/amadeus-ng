# カバレッジ 2 回目と担当報告の裁定

2026-09-11。5 担当の着地後、head 98.18%（base 99.15%、相対ゲート未達、残り約 850 行）。各担当が報告した裁定事項をまとめて提示する。

## Q1: 配布 scope ファイルの `keywords:` ブロック列（上流不一致）

[Question]: 配布 `.claude/scopes/aidlc-*.md` の `keywords:` は YAML ブロック列（`- fix`）だが、本 build の frontmatter 読取（`compiled_definition_repository_impl.rs:1150-1215`）はフロー列 `[a, b]` しか受理せず、配布ファイルのままではキーワードによる scope 推論が効かない（`next fix the crash` が compose 提案へ落ちる）。どうしますか？

- A. U2 でブロック列の読取を TDD で追加する（本家の挙動に揃える。推論経路は本家採取で固定）
- B. 観測差として記録し、切替条件 2 の判定材料にする（今回の実地スモークは `--scope selfhost-stage1` の明示指定なので経路を踏まない）
- X. Other (please specify)

[Answer]: A. U2 で読取を追加

## Q2: `pipeline_link_error.rs:89-98` の `Display` 書式に doc コメント行が混入

[Question]: 書式文字列が `"pipeline \n/// 指定されたlink。\nlink: {self:?}"` になっている（コードの誤り。文言の正本は出す側にあり観測互換には影響しない）。U2 で直しますか？

- A. U2 で直す（意図した 1 行の文言に戻し、テストで固定する）
- B. 記録だけ残し、別件にする
- X. Other (please specify)

[Answer]: A. U2 で直す (推奨)

## Q3: dead code 候補の扱い

[Question]: 5 担当が根拠付きで列挙した到達不能コード（harness 8 件、RMU 6 箇所、command-IA の単一変種ゆえの分岐、domain/use-case 5 箇所、app の `resume-menu` 分岐等）を削除しますか？ 削除すれば未カバー行が減り、カバレッジにも寄与する（数十行）。ただし `shell_write_targets.rs:485` の `${PWD}` 単独分岐は本家との同一性が未確認で、削除ではなく本家との突き合わせが要る。

- A. 根拠が「型・不変条件により到達不能」のものは削除する。`${PWD}` 分岐は本家と突き合わせてから決める
- B. 削除せず、すべて記録に留める
- X. Other (please specify)

[Answer]: A. 到達不能のものは削除 (推奨)

## Q4: カバレッジ 2 回目の進め方

[Question]: 残り約 850 行をどう進めますか？ 担当の報告では残りの多くがテストでは到達しにくい行（未使用クロージャ、I/O 失敗時のみの分岐、テストコード自身の `panic!` 腕）であり、2 回目で 99.14% に届く保証はない。

- A. app（651 行）と RMU（343 行）を中心に 2 回目を委譲し、Q3 の削除と合わせて再計測する
- B. ここで独立レビューへ進み、カバレッジは並行で続ける（Unit 完了はゲートを通してから）
- X. Other (please specify)

[Answer]: A. app / RMU 中心に 2 回目 (推奨)
