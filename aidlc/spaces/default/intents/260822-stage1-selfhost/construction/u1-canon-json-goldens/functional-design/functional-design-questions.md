# functional-design-questions — U1 canon-json とゴールデン（`u1-canon-json-goldens`）

> Functional Design（Construction 3.1）の質問票（Unit: U1）。出典: `../../../inception/units-generation/unit-of-work.md`
> （U1 の責務・境界・合格）、`../../../inception/units-generation/unit-of-work-story-map.md`（FR7.1〜7.3、NFR1）、
> `../../../inception/requirements-analysis/requirements.md`（FR7、前提 A3）、`../../../inception/domain-design/components.md`
> （CanonJson）、`../../../inception/contract-design/contract-summary.md`（C7 ゴールデン、C1 continue_token の正準化）、
> `docs/adr/0001-canonical-json-serializer.md`（3 プロファイル・受入条件 (a)〜(e)）。
>
> 構築フェーズの質問は「先行ステージで決まっていない本当の空白」だけに絞る。ADR 0001 で決まっている
> 内容（3 プロファイル・キー順・数値表記・直接呼び出し禁止・ゴールデン先行）は問い直さない。

## Q1. ゴールデン（正解データ）の非決定フィールドの扱い

CLI 実行出力・状態ファイル差分・監査行には、採取のたびに変わる値（ISO タイムスタンプ、`<host>-<clone>` の
シャード名、絶対パス、セッション ID）が含まれる。

- A. **固定できるものは採取時に固定し、残りはプレースホルダに正規化して比較**（推奨）— 例: タイムスタンプは
  `<TS>`、clone id は `<CLONE>`、絶対パスは `<ROOT>` に置換した上でバイト比較。置換規則はゴールデン表の
  一部として固定し、再採取スクリプトにも同じ規則を使う
- B. 採取環境を完全に固定して逐語比較のみ（固定クロック・固定 id・固定パスを upstream ツール側で注入できる
  前提が必要）
- X. Other (please specify)

[Answer]: A

## Q2. CLI ゴールデン（FR7.2）のシナリオ範囲

- A. **主要遷移 + フック代表ケース**（推奨）— `next` / `report` / `continue` / `park` の主要遷移
  （開始・awaiting-approval・approve・reject・revise・skip・jump・park/unpark・recompose・set-autonomy）と、
  フック 4 本の代表ケース（許可 / 拒否 / 無視 の 2〜3 件ずつ）。後続 Bolt で必要になった経路は追加採取
- B. 網羅 — 全 ROUTES 動詞・全分岐（21 分岐ラダーの全経路を含む）を最初から採取
- X. Other (please specify)

[Answer]: A

## 以前に確認済みのまとめ

- ゴールデンの非決定値（Q1 = A）: 固定できるものは採取時に固定し、残り（タイムスタンプ `<TS>`、clone id `<CLONE>`、絶対パス `<ROOT>`、セッション ID `<SESSION>`）はプレースホルダに正規化してバイト比較。置換規則はゴールデン表の一部として固定し、再採取スクリプトも同じ規則を使う
- CLI ゴールデンの範囲（Q2 = A）: next / report / continue / park の主要遷移（開始・awaiting-approval・approve・reject・revise・skip・jump・park/unpark・recompose・set-autonomy）+ フック 4 本の代表ケース（許可 / 拒否 / 無視 を 2〜3 件ずつ）。後続 Bolt で必要な経路は追加採取
- 設計判断（質問なし）: canon-json の公開 API はプロファイル enum（contract-pretty / contract-compact / hash-canonical）を引数に取る 1 組の関数（serialize / hash）とし、ADR 0001 の受入条件 (a)〜(e) を入力クラス別ゴールデン表で固定する

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct

## Consolidated Summary Confirmation

2026-09-05の不整合是正。Q1/Q2の確定方針と採取済みゴールデンは維持する。

- すべてのプロファイルで整数形式キーを数値昇順で先頭に配置する。残りのキーは正準ハッシュ用のみUTF-16順、それ以外は宣言順・挿入順とする。
- 整数の保持型と出力時のJS互換丸めを区別する。2^53を超える整数も採取済みの出力・ハッシュに合わせる。
- UTF-8で表せない孤立サロゲートは読取で拒否する。任意の外部JSONとの完全互換を根拠なく主張しない。
- to_valueの変換失敗、用途ごとのハッシュ族、現行モジュールの依存境界を設計本文へ反映する。
- 変更と根拠は `../correction-report.md`、実測対象は `core-infrastructure::canon_json` と採取済み32行のコーパス。過去のReview節は今回の承認として扱わない。

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]:
