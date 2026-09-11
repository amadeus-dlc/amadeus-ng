# Claude会話履歴の停止判定

## 責任と実装

`harness_claude::StopTranscript::parse(&str)` と `is_conversational()` を追加した。最後の実際の人間発言と、それ以降の作業用ツール呼出しの有無を観測する。ワークフロー状態・自律実行・ファイル選択・ロック・副作用は呼出側の責任として分離した。

メタ発言、ツール結果を含むユーザー欄、停止フック自身の追記を人間発言と数えない。読取り専用問い合わせと進行・更新呼出しの違いを、固定本家の文法で判定する。引用符の意味を独自に解釈して本家の結果を変更していない。

通常のJSON読取りで落ちる孤立サロゲート・巨大な数値を含む行について、先に元JSONの文法をRawValueで検査し、ASCII固定句・語境界・空白・値の種類という分類に必要な性質を保った一時表現へ写す。入力履歴や採取データは書き換えない。無関係な欄が原因でassistantの呼出しを丸ごと見落とす差を解消した。入力は文字列なので、ファイル不読や不正UTF-8バイトの取扱いは呼出側で検証する。

## 固定本家とTDD

採取元は本家2.7.1 `a277af218f0df7f325d3b8be7b6d90fce2c5bd40`。検証済み277ファイルの配布コピーにexport文だけを加え、`transcriptIsConversational` と依存するツール判定の本文を変更せず実行した。入力JSONL全文と判定は `tests/golden/selfhost-stage1/stop-transcript.json` に保存した。

- 最初のコンパイル失敗はテストファイルのcrate文書不足であり、Redには数えていない。
- 文書修正後、肯定の観測を実装していない状態でhuman-textがfalse対trueとなるRedを確認した。
- 通常・合成発言・ツール結果・フック追記・単純/複合コマンド・文字列境界等83観測がGreenになった。
- 孤立サロゲートと巨大数値を追加し、新たなRedを確認した。読取りを是正し、壊れた行の扱いも含む全87観測がGreenになった。

## 最終検証

- `cargo test -p harness-claude`: 全テスト成功。専用の公開API契約テストに87観測を含む。
- `cargo test -p core-infrastructure`: 全テスト成功。追加したserde_jsonのraw_value featureによる既存JSON・ハッシュ処理の回帰を確認。
- `cargo clippy -p harness-claude --all-targets -- -D warnings`: 終了0。
- `cargo lint`、対象のrustfmt確認、git diff確認: 終了0。

ログは `test-evidence/stop-transcript-*.txt` に保存した。停止フック本体への組込みと、状態依存の待機・保存/復旧を含むStep 6全体の検証は開発担当へ引き継ぐ。

## 変更ファイル

- `modules/harness/claude/Cargo.toml`
- `modules/harness/claude/src/lib.rs`
- `modules/harness/claude/src/stop_transcript.rs`
- `modules/harness/claude/src/engine_tool_call.rs`
- `modules/harness/claude/tests/stop_transcript_contract.rs`
- `scripts/goldens/capture-stop-transcript.ts`
- `tests/golden/selfhost-stage1/stop-transcript.json`
