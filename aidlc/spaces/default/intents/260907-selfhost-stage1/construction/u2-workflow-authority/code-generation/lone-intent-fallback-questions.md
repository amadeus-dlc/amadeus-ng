# カーソル先に状態ファイルが無い記録の扱い（F-H1）

2026-09-11。`u2_parity_closeout` の [parity-closeout-verification.md](parity-closeout-verification.md) §3 F-H1。本家 `a277af21` の判定と、U2 で先に着地した投影復元（Step 3）が両立しない 1 点。

## Q1: `active-intent` カーソルが名指すディレクトリは在るが `aidlc-state.md` が無いとき

[Question]: 本家どおりカーソルを無視して唯一記録へ後退しますか、それとも本 build どおり記録として選びジャーナルから状態ファイルを復元しますか？

- 本家 `aidlc-lib.ts` は `existsSync(join(dir, raw, "aidlc-state.md"))` で実在を判定し、無ければカーソルを無視して唯一の記録へ後退する（記録が複数なら「状態なし」）。
- 本 build は状態ファイルを SQLite ジャーナルからの投影として扱い、`next` が失われた状態ファイルを描き直す（`intent_lifecycle::next_restores_missing_projection_files_without_repeating_audit`、承認済み Step 3）。本家の判定をそのまま写すと、状態ファイルを失った記録が「存在しない」扱いになり復元経路へ到達しない。
- 担当は復元側へ倒して着地している（`layout.rs` `shared` はディレクトリの有無で判定、テスト `a_cursor_naming_a_directory_without_a_state_file_is_still_the_record`）。唯一記録の数え方は本家どおり。

- A. 本 build の判定（ディレクトリが在れば記録として選び復元）を保ち、切替条件 2 の判定材料として本家との差を記録する（推奨。復元は承認済み契約で、本家は状態ファイル正本なのでこの分岐自体が無い）
- B. 本家に合わせ `aidlc-state.md` の有無で判定し、`intent_lifecycle` の復元テスト 2 件を見直す（復元経路はカーソル無し・唯一記録の場合に限られる）
- X. Other (please specify)

[Answer]: B. 本家に合わせる
