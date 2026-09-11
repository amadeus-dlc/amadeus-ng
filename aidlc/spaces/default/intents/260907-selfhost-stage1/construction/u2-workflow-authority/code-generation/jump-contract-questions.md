# jump公開契約と既存ドメイン規則の確認

## 確認対象

通常のbugfix・単一rootで、`resolve`が導いた方向と同じ`execute`を実行する経路は実装・検証を進めている。以下はその経路以外で見つかった契約差であり、未回答のまま新しい拒否仕様を確定しない。

固定本家は`a277af218f0df7f325d3b8be7b6d90fce2c5bd40`。`/tmp/amadeus-u1-upstream/dist/claude`を`verifySource`で照合し、隔離した一時workspaceで本家とnativeを別々に実行した。[生観測](jump-logs/questions-observations.json)に入力・stdout・stderr・exitを保存した。初期条件はBrownfield、bugfix、現在工程reverse-engineeringで共通。

## Q1: 直接executeの引数をどこまで本家どおり受け入れるか

| 条件・入力 | 本家2.7.1 | 現在のnative |
|---|---|---|
| 方向不一致: `execute --target requirements-analysis --direction backward` | backwardとして成功。`stages_skipped=[]` | forwardを導出して成功。`stages_skipped=["reverse-engineering"]`。渡された方向と違う動作になっており、未解決の不一致 |
| 別scope: `execute --target domain-design --direction forward --scope classic` | classicの実効プランで成功。5工程をskip | bugfixの集約でtarget index 15を拒否、exit 1 |
| 初期化への直接移動: `execute --target workspace-scaffold --direction backward` | 成功し初期化3工程とreverse-engineeringをreset、completed=0 | 初期化target index 0を拒否、exit 1 |

本家の根拠（固定コミットの`core/tools/aidlc-jump.ts`）:

- 257–296行: `execute`は渡されたdirectionを検証してそのまま使い、scope指定があればそのscopeで検証する。
- 308–375行: 実際のdirectionに対応してskip/resetする。targetの初期化フェーズを拒否する条件はない。
- 454–539行: 全方向のSource Baseline、backwardの3配列、開始監査を記録する。
- `resolve`は210–251行で現在位置とtargetからdirectionを導出する。`execute`と同じ制約ではない。

現在の設計根拠:

- `modules/core/command/domain/src/orchestration/jump_direction.rs`は「方向は宣言ではなく導出」「矛盾した方向指定は表現不能」と定めている。
- `IntentExecution::jump_resolve`はinitializationと実行のscope外のtargetを拒否する。
- `Jumped`と`StageSlot::apply_jump`はtargetと移動前cursorから方向を導出し、明示directionや別scopeを保存しない。

候補は以下のとおり。

- A: 本家の直接execute契約を優先する。resolveの現在の制約とexecuteの制約を分離し、必要な指定・観測を保存する。既存の「方向は常に導出」の規則と関連テストも正式に更新する。
- B: 現在のドメイン規則を優先し、本家と異なる直接executeの条件を明示的な非互換として承認する。拒否文言・終了状態と受入対象を別途確定する。
- C: jumpの直接execute対応全体を今回から外す。既存projection goldenを除外して一致済みにはせず、未達として残す。
- X: その他。

[Answer]: A. 本家の直接execute契約を優先する

利用者の回答原文: 「推奨」。直前に提示した「3点すべて推奨案で再開する」を選択した回答として記録する。

## 既存goldenとの関係

`projection_golden_test`の`jump/execute-backward`は、domain-designからworkspace-scaffoldへの直接backwardを含む。これは今回の全数比較から消せない。投影関数に保存済みJumpedを渡せば監査を描画できることと、公開CLIが同じイベントを受理・保存できることは別の達成条件である。

この質問は真正な人間回答を記録するための未回答資料であり、承認・拒否・仕様変更を代行した記録ではない。
