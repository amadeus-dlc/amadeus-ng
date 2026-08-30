# U6 brief-1 — next / continue ユースケース（B15 + B16 の 2 Bolt 分割）

日付: 2026-08-30（オーナー承認: 2 Bolt 分割・本セッション直接実装・hmac + base64 採用）

## 固定裁定（現正典 — 2026-08-30 時点）

- **§2b（use-case-rules）**: 旧 I8 機構（Controller が集約を `&` で渡す）は失効。`Next` は
  **読取専用ポート注入**（`find_by_id` 系のみ）で読取専用を型強制する。execute 引数は
  集約 ID + VO のみ。仕様 10 §3 の旧 I8 文言は歴史記録（specs バナーの優先順位注記どおり）。
- **集約の構築**: genesis / replay / apply_event のみ（aggregate-commands「再構成の形」）。
- **エラー**: `RepositoryError<Id>` 1 本・source 連鎖（裁定 6）。ユースケースの失敗は
  モジュールごとの手実装 enum（材料のみ）。
- **判断はドメイン**: 21 分岐の状態判断は `IntentExecution::next_decision`（実装済み）。
  ユースケースはフロー制御のみ（FR3.3）。
- **綴り写像（逸脱 #1 / ADR 0002 決定 3）**: directive 文言中のコマンド参照は упstream 3 形の
  うち**素のマルチコール形 (2)**（例 `aidlc-utility status`）を正準として 1 モジュール
  （`command_spelling`）に集約する。ディスパッチャ語彙の完全 ROUTES 写しは U7 / A1 の責務で、
  差し替えはこの 1 点で行う。
- **文言**: ラダーの逐語メッセージは公開契約（Published Language B14）そのもの — use-case の
  `wording` サブモジュールに置く（error-handling の「文言を持ち込まない」はエラー型の話で、
  directive payload は対象外。message-catalog 解体後の「出す側が持つ」に従う）。

## B15 — `next` 21 分岐ラダー（本 Bolt）

- **domain**: `Directive` 判別共用体（10 kind、typed variants、placeholder 2 種は構築不能 —
  コンストラクタ検証 E1+E2。serde なし。28KiB 上限は Presenter/U7）。
- **use-case**: `NextUseCase<E, I, D>`（`IntentExecutionRepository` / `IntentRepository` /
  `WorkflowDefinitionRepository` の find 系のみ使用）。入力 `NextTurnInput`（Controller が
  パース済みのフラグ・トークン・env 解決値を運ぶ VO — env 読取・CLI パースは U7）。
  出力 `Directive`。分岐 0〜10 + 前置ガードのフロー制御。
- **run-stage payload**: StageNode（produces/consumes/sensors/reviewer/agents/mode）+
  workspace VO（StorePath / IntentDirName 系）から組む。steering 由来フィールド
  （rules_in_context / load-steering 連鎖）は B16 で搭載。
- **合格（FR3.1）**: 契約マップ（research/orchestration-next-ladder.md §1）の 21 ラベル +
  前置ガード + Branch 10 内部 5 手順の分岐網羅テスト green。逐語文言は契約マップの原文
  （コマンド綴りのみ写像形）。

## B16 — load-steering / continue_token / continue 動詞（次 Bolt）

- チャンク分割（STEERING_TEXT_TARGET_BYTES = 20KiB）、`ContinueToken`（HMAC-SHA256 封筒
  `{p,m}`・18 キー厳密型表・base64url・timingSafeEqual・4 ダイジェスト束縛）、fail-closed 6 形、
  `continue` 動詞。新依存 `hmac` + `base64`（オーナー承認済み）。鍵
  `.aidlc-steering-token-key` はマシンローカル Gateway（advisory — I8 例外 1）。
- ダイジェストは `core_infrastructure::canon_json`（sha256）を使う。

## 未決の申し送り

- ディスパッチャ語彙の完全 ROUTES 写し（30 経路 + SLASH_FLAG_ALIASES）は U7 で表として
  実体化し、`command_spelling` を差し替える。
- run-stage の conductor_persona 焼き込み・active-directive マーカー（I8 例外 2）は
  B16 以降（マーカー Gateway と同時）。

## B15 実施記録（2026-08-30）

- 実装: domain `Directive`（RunStage/Ask/Print/Error/Done/Parked — 構築できる部分集合）、
  use-case `NextUseCase`（読取専用ポート 3 本注入）・`NextTurnInput`・`scope_resolution`・
  `command_spelling`・`wording`。分岐網羅 40+ テスト、全ゲート緑、カバレッジ 98.748%。
- 逐語注記: 契約マップに完全引用が無い分岐（分岐 1 の全文・分岐 8 の質問文など）は意味論
  準拠で組み、FR4.1 の CLI ゴールデン（U7）で最終バイト合わせする。
- 工程事故: B14 が stale origin/main から分岐していた（fetch 漏れ）。remote はマージキューが
  救済。**新 Bolt ブランチは fetch 直後の origin/main から切る**を運用規約に追加すること。
- 次 (B16): load-steering 分割配信・ContinueToken（hmac + base64 承認済み）・continue 動詞・
  run-stage への rules_in_context 搭載・active-directive マーカー Gateway。

## B16 実施記録（2026-08-30）

- 実装: use-case `SteeringPlan`（20KiB 見出し境界分割 + UNSPLITTABLE_SECTION 後退分割）、
  `ContinueToken`/`ContinueTokenBuilder`（18 キー厳密型表・v=1）、`ContinueUseCase`
  （verify → 状態束縛比較 → 定義/ノード解決 → route ハッシュ比較 → rebuild_with_pins →
  bundle/directive ダイジェスト比較 → part 配送、fail-closed 6 形は逐語 wording）、
  `RuleBundleSource` ポート + fs 実装（org→team→project→phases/<phase>、欠落 skip・
  読取不能 blocking）。interface-adapter `ContinueTokenCodecImpl`（HMAC-SHA256 封筒
  {p,m}・base64url NO_PAD・`Mac::verify_slice` timing-safe・鍵不在時 getrandom ミント）。
  next 側は load-steering 連鎖の起点化と run-stage への rules_in_context 台帳搭載。
- 新依存: hmac 0.12 / base64 0.22 / getrandom 0.3（オーナー承認済み。暗号は adapter のみ、
  domain/use-case は封筒に非依存）。
- ゲート: fmt / clippy(-D warnings) / cargo lint / test 934 全緑、カバレッジ 98.79%
  （相対ゲート回復のため、網羅 match テストヘルパの実行補完と continue 異常系テストを追加）。
- 申し送り継続: 完全 ROUTES 表・conductor_persona 焼き込み・active-directive マーカー
  Gateway は U7。

## B16 是正記録（2026-08-30 オーナーレビュー裁定）

3 原則（Domain Primitive 化 / ユースケース層はフロー制御の層 / 貧血補償コード禁止）に
基づく全面是正。実装済み:

- **是正 1（ContinueToken の型化）**: 全フィールドをドメインプリミティブへ —
  `StageSlug`/`ScopeSlug`/`PartIndex`(from_raw は 0 拒否・next() のみ公開)/
  `TokenVersion`(is_supported)/`UnitRef`{UnitName, UnitKind enum(service|spec|ui|
  packaging|library)}/`StageName`/`Bindings`(4 ダイジェスト束縛 VO — BundleDigest/
  DirectiveDigest/RouteDigest 別型 newtype + Option<StateBinding> で aware×hash の不正
  状態を表現不能化)。ワイヤ予約フラグ f/w/z と unit から導出可能な p はフィールド廃止
  （decode は真値を fail-closed 拒否）。`too_many_arguments` allow 撤去（束縛は
  Bindings 1 個で受ける）。ExecutionMode enum 化は不採用: single/per-unit/wave の相互
  排他は slice-2 意味論が未確定で、bool は原則の許容範囲（brief へ理由記録）。
- **是正 2（不法入居 4 ファイル）**: `scope_resolution` → domain（判断ポリシー、
  ResolvedScope.name は ScopeSlug）。`steering` の分割・20KiB パック → adapter
  `rule_bundle_source_impl`（ポートは分割済み `SteeringPlan`(domain VO) を返し、
  Unsplittable は RuleBundleReadError の変種）。`command_spelling` → ドメイン
  `EngineCommand`（概念の閉集合、ConfigField enum）+ use-case `CommandSpelling`
  ポート + adapter `MulticallCommandSpelling`（綴りカタログ、U7 の ROUTES 差し替え
  点）。ReadOnlyVerb 点検: 変種名は操作の意図由来で維持、綴りメソッド subcommand() は
  adapter へ移した。
- **是正 3（steering_chain 解体）**: material 3 関数を廃止 — ダイジェスト対象は名前付き
  VO（`StageRoute` = WorkflowDefinition::stage_route クエリ、`StatePosition` =
  intent_id + seq_nr + StoreVersion newtype）で codec ポートの型付き 4 メソッドへ。
  直列化は adapter の serde 素材構造体（JSON エスケープ — {:?} と `|`/`::` 連結の
  欠陥を根絶）。`rebuild_with_pins`/`clone_with_rules` の 15 フィールド手動移送 ×2 は
  ドメイン `RunStageDirective::with_pins`/`with_rules_in_context`（clone ベースの部分
  更新 — フィールド追加時も黙って欠落しない）へ。emit_part の範囲外部エラーは
  `SteeringPart`（計画クエリのみが構築、範囲外は表現不能）+ LoadSteeringDirective::new
  の無謬化（LoadSteeringError 削除）で消滅。センチネル "-" はワイヤ形式の詳細として
  adapter 内に封じ込め。
- **StoreVersion の適用範囲**: Repository ポート面（store の expected_version /
  RehydratedIntentExecution::version）は本家 event-store-adapter-rs v3 の `usize` の
  まま — newtype 化は Conformist 方針違反として却下済み（オーナー裁定 2026-08-29）。
  StoreVersion は自前 VO `StatePosition` 内だけで使い、両裁定を両立。
- **port/ 集約（オーナー提案）**: ポート契約 trait と契約依存型
  （RehydratedIntentExecution/RepositoryError/GraphReadError/RuleBundleReadError/
  InvalidContinueToken/StatePosition/StoreVersion）を
  `use-case/src/orchestration/port/` へ。公開ファサードは据え置き（消費側パス不変）。
- **Directive::Error{message: String} 点検**: message はエンジン公開契約のペイロード
  （02 §4.4 逐語 6 形・契約マップ逐語）そのもの＝データであり維持。use case 内の文言
  組み立てで残るのは契約逐語のみ（emit_part の `internal:` format! は表現不能化で消滅）。
- ゲート: fmt / clippy(-D warnings, allow 追加なし・too_many_arguments allow 消滅) /
  cargo lint / 954 テスト全緑 / カバレッジ 98.77%。
