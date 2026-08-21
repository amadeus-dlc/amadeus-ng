# ADR 0004: 可観測性 — `tracing` 計装と OpenTelemetry エクスポート

- **ステータス**: **Accepted**（2026-08-22 オーナーレビューで確定: トレース境界は **1 ターン = 1 トレース**、エクスポートは flush 方式で開始。敵対的レビュー 1 巡反映済み）
- **日付**: 2026-08-22
- **対応**: `00-policy.md` §3 A10（プロジェクトオーナーの要求により追加: 監査イベント等を OTel に載せてログ出力できるようにする。計装は Rust の `tracing`、出力は OTel）
- **関連**: `01-domain-model.md` §2（横断機構）・§3.3（監査イベント / audit-first）、ADR 0001（正準 JSON シリアライザ — contract-compact プロファイルが Directive の stdout 1 行 JSON を規定）、将来の A4 ADR（プロセス実行基盤 — 内部 spawn のコンテキスト伝播を引き受ける）

## コンテキスト

upstream の外部可観測性経路は **StatsD メトリクスタップのみ**である: `aidlc-metrics.ts` が `AIDLC_METRICS_ENDPOINT` 設定時に限り、監査 append のチョークポイントから発火し、**detached spawn のワーカー**で配送する（「Metric loss is preferable to blocking or breaking the audit write」— `09-cli-tools.md:773-775`、`03-state-audit-runtime.md:116`）。トレース・ログ・呼び出し相関の経路は存在しない。監査台帳は D6 で凍結された契約でありワークフローの真実源だが、ストリーミング・横断検索・他システムとの相関には向かない。

実行形態には可観測性設計を難しくする制約が 3 つある。

1. **プロセスが短命**。engine・ツール・フックはサブ秒で終了する CLI 呼び出しの連鎖で、常駐前提の OTel バッチエクスポートがそのまま使えない。なお OTel のスパンは終了時に各プロセスが個別送信し、トレースはバックエンドが trace-id で漸進的に組み立てる — 「トレース完了時にまとめて送信」という概念はなく、この制約はトレース境界の選択（後述）と独立である。
2. **フックには個別の挙動契約がある**。fail-open は PreToolUse ガード 4 本の性質であって全フックの契約ではなく（`07-hooks.md:83`）、deliver-stage-rules は exit 2/3 の fail-closed 腕を、SESSION_ENDED の帰属は意図的な fail-closed を持つ。Stop フックのエンジンプローブは 10 秒制限。テレメトリは**いずれの終了コード・挙動契約も変えてはならない**。
3. **stdout は契約**。Directive は stdout の 1 行 JSON であり（ADR 0001 の contract-compact）、テレメトリやログが stdout を汚すことは互換破壊になる。

さらに伝播の物理的制約として、フックと engine を spawn するのは**ハーネス（Claude Code 等）と conductor の Bash** であり、amadeus 側から環境変数を注入できない。TRACEPARENT 環境変数による伝播が効くのは amadeus プロセスが自ら spawn する内部ホップ（Stop フック→engine プローブ、センサーワーカー、sibling CLI 等）だけである。

## 決定

1. **計装 API は `tracing` に統一**する。スパン・イベントの発行は application / interface adapter 層で行い、**domain 層は計装しない**（純粋性の維持 — 01 §7）。subscriber の初期化とエクスポータ構成は infrastructure（CLI エントリポイント）に置く。
2. **監査イベントの OTel 搬送は「派生」**とする。監査 append の単一チョークポイントで、**append の成功後にのみ** `tracing` イベントを発行し、イベント種別・space・intent・stage 等を `aidlc.*` 名前空間の属性（例: `aidlc.event_type`, `aidlc.space`, `aidlc.intent`, `aidlc.stage`, `aidlc.session_id`）で載せる。監査台帳が真実源で、テレメトリはその派生という向きを両方向で固定する: **OTel 側は欠落しうるが、台帳に存在しないイベントが OTel に現れることはない**。テレメトリの失敗はワークフローの挙動に一切影響しない（何もスローしない — upstream usage ledger / metrics タップと同じ堅牢性契約）。
3. **既存 StatsD タップとの関係**: `AIDLC_METRICS_ENDPOINT` / `AIDLC_METRICS_PREFIX` の StatsD 挙動は D6 互換対象として**維持**し、OTel は並存する別経路とする。統合（StatsD 廃止・OTel メトリクスへの一本化）は upstream 追従（A7）と改名再判断の際に再訪し、廃止する場合は逸脱台帳に記録する。
4. **トレース境界は「1 ターン = 1 トレース」**（オーナー決定。LLM が 1 ターンの中で aidlc ツール群を駆動した一連の呼び出しが 1 本のトレースになる）。
   - **ターンコンテキストの発行**: prompt-submit フック（HUMAN_TURN を刻む場所）がターン開始時に trace-id ＋親 span context をマシンローカルに発行し、Stop フックがターン終了時に閉じる。**engine は触らない**。コンテキストファイルは session_id でキーイングし、フックは stdin の session_id で自セッションのターンを引く。
   - **ターン内のスパン**: 各 CLI 呼び出し・フック発火はそのターントレース内のトップレベル子スパン（tool ＋サブコマンド名）になる。amadeus 内部 spawn への伝播は W3C `TRACEPARENT` 環境変数で行う（A4 の仕様に含める）。
   - **セッション相関**: 同一セッションのターン同士は `aidlc.session_id` 属性＋ span link でつなぎ、セッション全体はバックエンドの属性検索で束ねる。
   - **入口別キャリア表**（2026-08-22 確定 — PR #2 レビュー反映）: (1) **フック** = stdin の `session_id` で自セッションのターンコンテキストファイルを引く。(2) **amadeus 内部 spawn** = `TRACEPARENT` 環境変数（A4）。(3) **engine（session_id を受け取らない）** = マシンローカルのターンコンテキストのうち、active-directive マーカーの owner セッションと**キーが一致するもののみ**を採用する。
   - **縮退規則**: キーが一致しない・解決不能・セッション外実行の場合は、**自分をルートとする単独トレースに縮退**し `aidlc.*` 属性で相関する。**「最新ターン」へのフォールバックは禁止**（並行セッションで他セッションのターンに誤帰属するため — 誤った因果を作るくらいなら相関を放棄する）。engine のキー照合の詳細（マーカーのどのフィールドで一致を取るか）は orchestration コンテキスト仕様で規定するが、この縮退規則と禁止則は本 ADR で確定とする。
   - この設計によりトレースは常にターンサイズ（秒〜分）に収まり、tail-sampling・バックエンド表示の前提と整合する。
5. **エクスポートは既定で無効、方式は flush で開始**（オーナー決定）。汎用 `OTEL_EXPORTER_OTLP_ENDPOINT` **またはシグナル別**（`OTEL_EXPORTER_OTLP_TRACES_ENDPOINT` / `_LOGS_ENDPOINT` 等）のいずれかが設定されている場合のみ有効化し、未設定時はサブスクライバ登録ごとスキップして起動オーバーヘッドをゼロに近づける。送信は exit 前の明示 flush ＋短いタイムアウト（初期値 数百 ms）で行い、**Stop フックは 10 秒プローブ予算を守るため特に短いタイムアウト（100ms 級）**とする。hot path（毎ターンの engine、毎ツール呼び出しのフック）の実測で痛みが出たら、upstream 流の **detached ワーカー配送**へエスカレートする（較正条項）。
6. **stdout の保護とログ**: テレメトリ・ログは決して stdout に出さない。stdout は契約出力（Directive 等）専用、診断は stderr、テレメトリは OTLP。`tracing-subscriber` の fmt 出力（人間向けログ）は stderr に限定し、既定は quiet、`AIDLC_LOG`（`RUST_LOG` 互換記法）で制御する。upstream 既存の `AIDLC_HOOK_DEBUG`（フックのファイルデバッグログ）は D6 互換対象として維持し、`AIDLC_LOG` は stderr の構造化ログという別役割とする。`AIDLC_LOG` は AIDLC_* 名前空間への独自追加なので逸脱台帳（拡張分類）に記録し、A7 の diff レビューで upstream の同名導入を監視する。
7. **依存とビルド**: `tracing` は常時依存とし、`opentelemetry` / `tracing-opentelemetry` / OTLP エクスポータは Cargo feature で分離する。既定ビルドに含めるか否かはバイナリサイズ実測の上で A1（配布モデル）と合わせて確定する。

## 帰結

- 監査イベントが OTel のイベント/ログとして外部基盤に流れ、**ターン単位の因果**（1 ターン内にどの engine 呼び出し・フック発火・センサー発火が起きたか）がトレース境界そのものとして得られる。セッション全体の見え方は属性検索と span link 越しになる（1 本のセッショントレースは存在しない）。
- **エンドポイント設定時は各プロセスの exit に flush タイムアウト上限分のレイテンシが上乗せされうる**。毎ツール呼び出しで発火するフックと毎ターンの engine 呼び出しで合算されるため、較正（決定 5）で実測し、痛ければ detached ワーカーへ移行する。
- prompt-submit / Stop フックにターンコンテキストの発行・クローズという小さな責務が、A4（spawn 基盤）に `TRACEPARENT` 伝播が、それぞれ要件として加わる。
- OTel 系クレートの分だけビルド時間とバイナリサイズが増える（feature 分離で配布判断に自由度を残す）。

## 検討した代替案

- **`log` クレート**: 不採用。スパン・構造化属性・コンテキスト伝播がなく、トレース相関の要求を満たせない。
- **1 セッション = 1 トレース（案 A）**: 不採用（オーナーレビューで棄却）。送信タイミングは案 B と同じだが、数時間〜数日のセッションで 1 つの trace-id にスパンが積もり続け、tail-sampling の判定窓・バックエンド表示の前提と衝突する。ターンが作業の実態に合う。
- **upstream StatsD タップの拡張**: 不採用（置換はしない）。StatsD はメトリクスのみでトレース・ログを運べないが、互換維持のため経路としては並存させる（決定 3）。その detached spawn 配送方式はエスカレーション先として採用（決定 5 の較正条項）。
- **監査台帳の後段変換のみ**（台帳 → OTLP の一括変換コマンド）: 単独では不採用だが、補完として有用（過去分の投入・flush 欠落の補償）。フェーズ C 以降の追加候補として残す。

## 未確定事項

- OTLP トランスポートの選定（gRPC か http/protobuf か）とエクスポータクレートの構成。
- engine のターンコンテキストのキー照合フィールドの詳細 — orchestration コンテキスト仕様で確定（縮退規則と「最新ターン」フォールバック禁止は決定 4 で確定済み）。
- hot path の flush 予算の較正結果と detached ワーカーへの移行判断。フックごとの有効化範囲。
- メトリクス・シグナル（実行時間、トークン/コストの usage ledger 連携）の範囲 — フェーズ B 以降に再訪。
- スパン属性スキーマ（`aidlc.*` 一覧）は orchestration / workspace コンテキスト仕様の執筆時に確定し、監査イベントスキーマ（Published Language）と対応付ける。
