# 規則整合監査 2026-08-24

**監査実施**: 2026-08-26T12:16Z（JST 21:16）/ 独立レビュア（aidlc-architecture-reviewer-agent）
**対象**: `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/` の 15 ファイル全文
**注記**: 監査中に対象ファイルが更新されている（`README.md` 21:13、`factory-naming.md` /
`good-examples.md` 21:06、`abstract-data-type.md` / `field-visibility.md` 20:47、
`tell-dont-ask.md` 20:41）。本監査は **21:16 時点のディスク内容**を正とする。

---

## 1. 読んだ範囲（何本・何行を通読したか）

15 ファイル・計 1,227 行（`wc -l` 実測、CONSISTENCY-AUDIT 自身を除く）。全ファイルを全文通読した。

| ファイル | 行 | ファイル | 行 |
| --- | ---: | --- | ---: |
| `README.md`（21:13 改訂後） | 73 | `interior-mutability.md` | 140 |
| `abstract-data-type.md` | 105 | `module-visibility.md` | 31 |
| `tell-dont-ask.md` | 62 | `no-backward-compatibility.md` | 38 |
| `field-visibility.md` | 60 | `cqrs-boundaries.md` | 96 |
| `domain-equality.md` | 17 | `gateway-taxonomy.md` | 121 |
| `factory-naming.md` | 233 | `use-case-rules.md` | 40 |
| `ubiquitous-language.md` | 92 | `error-handling.md` | 24 |
| `command-query-separation.md` | 51 | `good-examples.md` | 86 |

規則文書のみでは真偽が決まらない主張については、次の実測で裏を取った（コードの設計評価は
行っていない。**規則文面の主張が事実か**の確認に限る）:

- `tools/lint/src/check.rs` — 実装済みルール ID、抑制コメントの実装、既存の緑テスト
- `modules/core/use-case/src/**` — `pub trait` の宣言箇所とメソッド名
- `modules/**` — `pub` / `pub(crate)` フィールドの残存数
- 全 `.md` の相対リンク 11 本の到達性

---

## 2. 矛盾（深刻度つき）

| # | 深刻度 | 規則 A | 規則 B | どう衝突するか | 機械的に読むとどちらに従うか判断できるか | 推奨 |
| --- | --- | --- | --- | --- | --- | --- |
| C1 | **Critical** | `gateway-taxonomy.md` §3「ポート造語（Store / Reader / Writer / Source / Provider）は禁止」＋ §機械強制の候補 1「use-case 層の `pub trait` 名が `Store`/`Reader`… で終わったら拒否」 | `cqrs-boundaries.md` 配置表・§原則（`EventStore` / `JournalReader` を正規ポートとして記載）、`command-query-separation.md` §適用例（`EventStore::persist_event` / `JournalReader::advance_checkpoint` を**適合例**として掲示） | 実測: `modules/core/use-case/src/orchestration/event_store.rs` と `journal_reader.rs` に `pub trait EventStore<AID, A, E>` / `pub trait JournalReader` が実在する。§機械強制の候補 1 を実装した瞬間、この 2 本が落ちる。禁止規則側に例外条項が無く、`cqrs-boundaries.md` 側にも「造語禁止の例外である」との明示が無い | **できない**。禁止側は無条件、採用側は無言。片方を消す以外の読み分けが文面から導けない | どちらかに裁定を書く。(a) ES/CQRS の 2 ポートを §3 の明示的例外として `gateway-taxonomy.md` に列挙し理由（event-store-adapter-rs の語彙、ADR-006）を付す、(b) 改名する、のいずれか。候補 1 の lint 定義も同時に更新する |
| C2 | **Critical** | `use-case-rules.md` §4「CQRS を導入せずに（[gateway-taxonomy.md] — CQRS 不採用）」 | `cqrs-boundaries.md` 全体（「本プロジェクトは CQRS + イベントソーシングを採用している」）／`gateway-taxonomy.md` §4 の 2026-08-24 改訂（「前提が失効した」と自ら宣言） | 同一の正本の中に「CQRS 不採用」と「CQRS 採用」が同時に現れる。しかも `use-case-rules.md` は**その根拠として `gateway-taxonomy.md` を名指ししている**が、名指しされた側は既に差し替え済み | **できない**。日付だけでは決められない（`use-case-rules.md` に改訂日が無く、裁定日は 2026-08-22 のまま） | `use-case-rules.md` §4 の括弧書きを削除し、`cqrs-boundaries.md` を参照に差し替える。コンダクタが `gateway-taxonomy.md` §4 で行った失効処理と**同じ処理が未実施**の箇所 |
| C3 | **Critical** | `field-visibility.md` §ルール 5 番目（line 15）「`pub(crate)` は同一クレート内の実装詳細共有にのみ許す（既定はやはり private）」 | 同ファイル §改訂 2026-08-24「**`pub` も `pub(crate)` も、理由なしに使ってはいけない。そして本ルールに例外は認めない**」 | **同一ファイル内の正面衝突**。前者は「許す」、後者は「例外なし」。前者は削除も失効注記もされずに `## ルール` 本体に残っている | **できない**。`## ルール` を読んで実装する読み手（規則の一次読解経路）は前者しか見ない | line 15 を削除する。`no-backward-compatibility.md` §対象外 が「履歴記述は消さずに**失効した旨を追記**する」と定めているので、削除ではなく打ち消し線＋失効注記でもよい。いずれにせよ現状は**その規則が自分自身に適用されていない** |
| M1 | Major | `gateway-taxonomy.md` §1「インターフェイスアダプタ層の Gateway が担うのは次の **2 種類に限る**」 | `cqrs-boundaries.md`（`EventStore` / `JournalReader` を ES 基盤のポートとして導入） | `EventStore` は Repository でも外部システムクライアントでもない（集約の永続化を担うのは `WorkflowExecutionRepository` で、`EventStore` はその下請け）。閉じた 2 分類に第三の実在が収まらない。**帰結**: `EventStore` のメソッド語彙にどの規則も届かない — §2b の「`load` / `get` / `fetch` は使わない」は Repository 限定なので、実測の `get_latest_snapshot_by_id` / `get_events_by_id_since_seq_nr` は規則の射程外 | 判断できない（分類できないので、どの語彙規則が適用されるかも決まらない） | §1 に第三の責務「**永続化基盤ポート**（ES ジャーナル／スナップショット／投影チェックポイント）」を追加し、その語彙は本家ライブラリに従うと明記する（§2b の ES 拡張語彙と同じ扱い） |
| M2 | Major | `command-query-separation.md` §許容される違反「Builder のメソッドチェーン ＝ **オーナー許可が前提**の違反」 | `factory-naming.md` §「ビルダーの鎖メソッドは setter ではない — ファクトリメソッドである」「本ルールが排除するのは前者であって後者ではない」 | `ScopeMetadata::with_depth(mut self) -> Self` を書くのに**オーナー許可が要るのか要らないのか**が 2 文書で逆。`good-examples.md` は同じメソッドを無条件の良い例として掲示している | **できない**。CQS 側は「許可が前提」と書き、factory-naming 側と good-examples 側は許可に一切触れない | CQS の表から「Builder のメソッドチェーン」行を削除し、`self`（値受け）レシーバは CQS の対象外であると §対象外 に書く（下記 M3 と同時に直る） |
| M3 | Major | `command-query-separation.md` §判定フロー（`&self` / `&mut self` の 2 分岐） | `factory-naming.md`（`mut self -> Self`）、同 §対象外（`build(self) -> T`）、`abstract-data-type.md`（`into_*`） | 判定フローが**値受け（`self`）レシーバを一切扱っていない**。`build()` / `into_*` / `with_*` は「状態を変更するか？」でも `&self`/`&mut self` でもないので、フローに入れると必ず「許容される違反」へ落ちる。M2 の構造的原因 | 判断できない（フローに該当分岐が無い） | 判定フローの手前に「レシーバが `self`（消費）なら本ルールの対象外 — ファクトリ／変換である」を置く |
| M4 | Major | `field-visibility.md` §ルール「読み取りは**アクセサメソッド**で公開する」（無条件）＋ `good-examples.md`（`StageSlug::as_str` を無条件の良い例として掲示） | `abstract-data-type.md` §境界での変換「暴露と変換の違いは名前ではなく**必要性**にある」「**1 件ずつ見る必要がある**」＋ `tell-dont-ask.md` §追記の 3 段階（3 に落ちるのは「プリミティブへ降りることが本当に必要な境界」だけ） | 土台側は「アクセサは既定で疑え、必要性を 1 件ずつ検証せよ」、field-visibility 側は「フィールドを private にしたらアクセサで公開せよ」。機械的に適用すると、field-visibility に従った瞬間に土台の禁止パターン（「内部型を返すだけのアクセサ」）を踏む | **できない**。しかも**「必要性」の判定を誰が行い、どこに記録するかが唯一無規定** — `ubiquitous-language.md` と `factory-naming.md` は「doc コメントに理由一行」を要求するのに、`abstract-data-type.md` §境界での変換 は記録先を定めていない | (a) field-visibility §ルールの当該箇所に「アクセサを置く前に `tell-dont-ask.md` の 3 段階を通す」を差し込む、(b) 境界変換アクセサにも「doc コメントに一行」の作法を課して他規則と揃える。**コンダクタが危ぶんだ tell-dont-ask × factory-naming × abstract-data-type の三角は、読む限り整合している**（§3 参照）。危ういのはこちらの組み合わせ |
| M5 | Major | `README.md` §機械化ロードマップ（21:13 新設）「個々の規則に『予定』と書き足すのをやめる（規則側は『レビュー基準』か『`cargo lint`（ルール名）』のどちらかだけを書く）」 | 8 本の規則ファイルの `**機械強制**` ヘッダ（実測、21:16 時点）: `command-query-separation`「ルール化予定」／`interior-mutability`「ルール化予定」／`module-visibility`「ルール化予定」／`use-case-rules`「ルール候補」／`gateway-taxonomy`「将来 `cargo lint` ルール候補」／`error-handling`「ルール候補」／`factory-naming`「ルール化候補」／`no-backward-compatibility`「ルール化候補」 | README が定めた新方針が、方針の対象である 8 本に未反映。README 21:13 と規則ファイル 20:41〜21:06 の時系列どおり | README が新しいので README が勝つ、と読むのは**推測**（優先順の明文が無い — §3） | 8 本のヘッダを機械的に置換する。ロードマップに載った 2 本（`field-visibility` / `factory-naming`）は README 表と本文の表現も揃える |
| M6 | Major | `field-visibility.md` §検出境界の拡張「**抑制コメントは受け付けない** — `// amadeus-lint: allow(no-public-fields)` を書いても抑制しない実装にする」 | `tools/lint/src/check.rs`（実測）— `is_suppressed()` は rule-id 一致＋理由ありで**全ルール一律に抑制**する。緑テスト `r3_is_suppressed_by_a_matching_allow_comment`（check.rs:782-790）が `// amadeus-lint: allow(no-public-fields) — serde の外部表現 (境界の DTO)` を**ドメイン層のパス**で抑制することを固定している | 規則は「例外は無い」、実装は「境界 DTO を例外として通す」。さらに `abstract-data-type.md` §対象外 は「ワイヤ表現の型は対象外」と書くのに、`field-visibility.md` には対象外節が**無い**（下記 §6） | **できない**。規則を実装すると既存の緑テストが赤になる。テストを消すのか、規則に対象外を書くのかが未決 | field-visibility に §対象外 を新設し、`abstract-data-type.md` §対象外 と同じ範囲（ワイヤ表現型・`modules/shared/`）を明記する。そのうえで「その範囲の外に例外は無い」と書けば実装と規則が一致する |
| M7 | Major | 例外の作法が **5 通り**（下記 §6 の表） | 同左 | 「理由を書けばよい」「オーナー許可が要る」「例外なし」「例外規定なし」が規則ごとにばらばらで、共通語彙が無い。読み手は毎回そのファイルを開かないと、自分が何をすれば例外を通せるのか分からない | 個別には読めるが、**規則をまたぐと読めない** | 4 段階（`例外なし` / `理由を doc に記述` / `lint 抑制＋理由` / `オーナー許可＋理由`）を README に定義し、各規則のヘッダに `**例外**: <段階>` の 1 行を足す |
| M8 | Major | `interior-mutability.md` §機械化の候補 1「例外を許すため **`#[allow]` + 理由コメント**を要求」 | `tell-dont-ask.md` §集約所有の前提集合／`ubiquitous-language.md` ヘッダ／`factory-naming.md` §広いルールを機械化する道／`check.rs` 実装 — いずれも `// amadeus-lint: allow(<rule>) <理由>` | 抑制構文が 2 つ並立している。`#[allow]` 形式は `check.rs` に実装が無い（実測: 抑制判定はコメント行の前方一致のみ） | できない（両方が「規則」として書かれている） | `interior-mutability.md` の該当箇所を `// amadeus-lint: allow(...)` へ統一する |
| m1 | Minor | `README.md` 索引表（13 行） | `abstract-data-type.md` / `good-examples.md` | 土台と良い例カタログが**表に無い**（本文の散文でのみ言及）。`abstract-data-type.md` は §禁止パターン を自前で持つ実効規則であり、表だけを見る読み手はそれを見落とす | — | 表に 2 行足す（土台には「機械強制: 部分的」と既にヘッダがある） |
| m2 | Minor | `README.md` / `abstract-data-type.md` §ここから導かれる規則（6 本を主張） | 実測（grep）— `abstract-data-type` へ戻るリンクを持つのは `field-visibility.md` と `factory-naming.md` の **2 本だけ** | 導出が片方向。`tell-dont-ask` / `command-query-separation` / `domain-equality` / `ubiquitous-language` は土台の存在を知らない。土台を改訂しても 4 本は追随しない | — | 4 本のヘッダ `**関連**` に土台を足す（§4 で導出自体の妥当性も評価した） |
| m3 | Minor | `gateway-taxonomy.md` §適用の帰結 表 | `interior-mutability.md` 適用例／ADR-007 | 表が `FsWorkspaceLock`（ADR-007 で退役）を現在形で参照し、`FsStateFileStore` を「`pub(crate)` へ降格した」と肯定的に記す。後者は C3 の争点そのもの | — | 表に「2026-08-22 時点の記録。`FsWorkspaceLock` は ADR-007 で退役」の注を付す（§1b では既に退役処理済みなので、表だけ取り残されている） |
| m4 | Minor | `gateway-taxonomy.md` §2b 許容動詞 `save` / `remove` | `cqrs-boundaries.md`（ES 採用）／実測（`WorkflowExecutionRepository` は `store`/`find_by_id`、`WorkflowDefinitionRepository` は読取専用） | `save` / `remove` に対応する実在メソッドが無い。ES 一本化後に残った死語かどうかが文面から読めない | — | 「ステートソーシング Repository 向けの語彙。現在の実装には該当が無い」と注記するか削除する |
| m5 | Minor | `cqrs-boundaries.md` 配置表「`EventStoreImpl` が `EventStore` と `JournalReader` を実装」 | `gateway-taxonomy.md` §5「**1 trait 1 Impl**」 | 1 つの実装型が 2 つのポートを実装している。「1 trait 1 Impl」を「1 実装型 1 trait」と読むと違反、「1 trait につき本物の実装は 1 つ」と読むと適合。文面が両読みを許す | 読み分けが必要だが根拠が無い | §5 の文言を「1 trait につき本物の実装は 1 つ（`Impl` 型が複数の trait を実装するのは可）」と明確化する |
| m6 | Minor | `abstract-data-type.md` §帰結「**`pub(crate)` は「カプセル化の単位をクレートと取った」という告白である**」（主語なし＝全アイテム） | `module-visibility.md` §運用ループ「再輸出しない内部共有は **`pub(crate)` / `pub(super)` へ明示的に降格する**」 | 土台がアイテム全般を指しているのかフィールド限定なのかが書かれていない。全般と読むと、module-visibility が指示する降格が土台違反になる | **できない**（土台側に射程の明示が無い） | 土台の当該バレットに「フィールドについて」と射程を書く。アイテム（`fn` / `struct` / `mod`）の `pub(crate)` は module-visibility の運用ループが正である旨も併記する |

---

## 3. 優先順位について

**現状: 衝突解決の規則は 1 つも無い。** grep で「優先」「勝つ」を全ファイルから拾うと、出てくるのは
規則**内部**の優先（`factory-naming` の「正確なドメイン語が勝つ」、`domain-equality` の「ドメイン側が
勝つ」）と、機械化手段の優先順（型 → 既存 lint → `cargo lint`）だけである。**規則 A と規則 B が
違う方向を指したときにどちらが勝つか**は、どこにも書かれていない。

`abstract-data-type.md` は「土台」を名乗り 6 本を導いていると主張するが、**土台であることが
衝突時の優先を意味するとは書いていない**。実際 §2 の C3 / M4 / M6 はいずれも「土台の言うことと
派生規則の言うことが違う」形をしており、土台が勝つと決まっていれば機械的に解ける。

### 置くべきか

置くべきである。理由は 2 つ。

1. 規則が 13 本になり、1 人の読み手が全文を頭に入れて衝突を裁定する前提が成立しなくなった
   （本監査で **Critical 3 件・Major 8 件**の衝突が出た）。
2. この正本は**エージェントも読む**（`project.md` の Mandated に「コード・仕様・レビューを書く前に
   読んで従う」とある）。機械的な読み手は「どちらか一方を選ぶ」ができず、片方を無視するか停止する。

### 置くならどういう形が良いか

`README.md` に **1 節・10 行以内**で置く。長い調停規則は誰も読まないので、**順序の列挙**にとどめる。

```markdown
## 規則が衝突したら（優先順）

1. **観測互換**（`docs/specs/` の upstream 契約）— これだけは設計規則より上。
   Published Language の逐語は常に勝つ。
2. **「例外を認めない」と明記した規則** — 現在は field-visibility のみ。
3. **土台**（abstract-data-type）の**目的**（呼び手が表現を知らずに済むこと）。
   派生規則の文面がこの目的を裏切るなら、派生規則の文面が誤りである。
4. **各規則の本文** — 日付が新しいほうが勝つ（裁定日ヘッダで比較する）。
5. **表・語彙の既定**（factory-naming の対応表など）— 最も弱い。
   より正確なドメイン語があればそちらが勝つ、と規則自身が既に書いている。

衝突を見つけたら、読み替えて進まずに**その場で正本を直す**（`project.md` の
Corrections「上流成果物の矛盾は読み替えず裁定を求める」の適用）。
```

3 番目の「文面ではなく**目的**が勝つ」が要点である。土台の文面を優先すると M4 のように
派生規則の運用が止まるが、目的を優先すれば「アクセサを置く前に必要性を問う」という
実行可能な指示になる。

---

## 4. 導出の検証 — abstract-data-type からの 6 本

各規則を全文読み、「抽象データ型（操作で定義され表現では定義されない）」から**実際に導けるか**を
判定した。判定は 3 段階（**成立** / **半分** / **こじつけ**）。

| 規則 | 導出は成り立つか | 根拠 / こじつけならその理由 |
| --- | --- | --- |
| `field-visibility.md` | **成立** | 「表現をフィールドで見せない」は抽象データ型の定義そのもの。派生側も §改訂 2026-08-24 で土台を引用しており、**双方向にリンクしている唯一の組**。ただし §ルール line 15 が土台と矛盾したまま残っている（C3）ので、**導出の主張は正しいが文面が追随していない** |
| `tell-dont-ask.md` | **半分** | 成立するのは **§追記 2026-08-24（アクセサで内部型を意識させない）だけ**。この節は「呼び手がその型を容器として扱いはじめる」＝表現への依存を問題にしており、土台から直に出る。一方、本体（2026-08-22 の §ルール）は「**判断を状態の所有者へ移す**」＝振る舞いの配置の話であり、表現の隠蔽とは別の原理（責務配置／情報エキスパート）から来る。`CheckboxState` の分類述語を呼出側で再実装するな、という規範は、フィールドが全部 private でも成立するし、逆に全部 public でも「判断は所有者へ」は言える。**土台からは出てこない**。しかも実測でこのファイルは土台へのリンクを 1 本も持たない |
| `factory-naming.md` | **半分（かつ土台側の記述が誤り）** | 導出が成立するのは「**構築経路を 1 本に集約する／不正な表現を構成させない**」の部分だけ。ファイルの大半（対応表 `new`/`of`/`from`/`parse`/`create`/`generate`/`open`、`valueOf`・`getInstance`・`newInstance` を採らない理由、ビルダーと setter の区別、`with_*` 命名、機械化の可否）は **Java 由来の命名表と Rust API ガイドラインから来ており、抽象データ型とは無関係**である。<br>さらに **土台側の記述が事実と食い違う**: `abstract-data-type.md` §ここから導かれる規則 の表は factory-naming を「**`parse` が唯一の入口**、完全コンストラクタ」と要約するが、factory-naming §基本コンストラクタが `new` とは限らない は「どれが基本かは**型による**。**『1 つに定まっている』ことが要件**であって、名前ではない」と明言し、`new` / `parse` / `open` の 3 通りを挙げている。**土台の要約だけを読んだ機械的な読み手は「全部 `parse` にせよ」と誤読する** |
| `command-query-separation.md` | **こじつけ** | 土台の表は「表現を書き換えさせない → CQS（Command は型自身の `&mut self` メソッド）」と書くが、**CQS が主張しているのはそこではない**。CQS の中身は「**変更するメソッドは値を返すな、読むメソッドは変更するな**」という**メソッド契約の形**の規律であり、変更が型自身のメソッド経由であること（＝カプセル化）は CQS の前提であって帰結ではない。「変更は型自身のメソッド経由」を導いているのは field-visibility であり、CQS ではない。実際 CQS ファイルは裁定の出典に fraktor-rs の `cqs-principle.md` を挙げ、土台には触れていない。公開フィールドを全面禁止しても CQS は自動的には従わないし、CQS を守っても表現は隠れない — **両者は独立** |
| `domain-equality.md` | **成立（ただし片方向）** | 「同値を**表現の構造**ではなく**ドメインの意味**で決める」は、「型は操作で定義され表現では定義されない」の直接の帰結として読める。`derive` の構造的等価より手実装が勝つ、同一性に含まれないフィールドは `eq` から除外する、はどちらも「表現 ≠ 契約」の言い換えである。**導出は妥当**。ただし裁定日が土台より 2 日早く（2026-08-22 / 08-24）、リンクも一切無いので、土台が後から取り込んだ形になっている |
| `ubiquitous-language.md` | **こじつけ** | 土台の表は「表現の語で名乗らない → ubiquitous-language」とするが、UL ファイルの中身は **Published Language と Ubiquitous Language の区別**、**upstream の語彙をどこまで写すか**、**例外に理由を書く作法**であり、DDD の言語論から来ている。抽象データ型と重なるのは §禁止パターン の 1 行（`set_` / `get_` / `update_` をドメインメソッド名にしない）だけで、これは規則全体のごく一部である。UL の `**関連**` ヘッダも `factory-naming` / `gateway-taxonomy` / `tell-dont-ask` を挙げ、**土台を挙げていない** — 書いた本人も導出とは思っていないように読める |

### まとめ

主張されている 6 本のうち、**素直に成立するのは 2 本**（field-visibility / domain-equality）、
**半分が 2 本**（tell-dont-ask / factory-naming）、**こじつけが 2 本**（CQS / ubiquitous-language）。

「6 本がここから導かれる」という README と土台の記述は**過大主張**である。実害は 2 つ:

1. 土台の要約表が派生規則を**誤って要約している**（factory-naming の `parse` 唯一入口。上表参照）。
   要約が原典と食い違うのは、要約が使われるほど危険になる。
2. 「土台だから優先」という運用を始めた場合、CQS と ubiquitous-language は土台と無関係なので
   **土台を根拠に上書きされうる**。§3 の優先順を「文面ではなく目的が勝つ」形にすれば
   この事故は起きない。

**推奨**: 土台の §ここから導かれる規則 を 2 つに割る。「**この土台から導かれる**」（field-visibility /
domain-equality / tell-dont-ask §追記 / factory-naming の構築経路 1 本）と「**土台と両立するが
別の原理から来る**」（CQS / ubiquitous-language / tell-dont-ask 本体 / factory-naming の命名表）。
後者は「導出」ではなく「関連」と呼べばよく、それだけで過大主張は消える。

---

## 5. 重複

同じことを 2 箇所以上で言っている箇所。**片方が古くなる危険**の順に並べた。

| # | 何が重複しているか | どこ | 危険度 |
| --- | --- | --- | --- |
| D1 | **「アクセサ／getter は I/O 境界では正当」** | `tell-dont-ask.md` §適用領域（「I/O 境界での getter 使用は正当であり対象外」）／`field-visibility.md` §根拠 第 2 段落（「アクセサの**存在**は I/O 境界のためにある」）／`abstract-data-type.md` §境界での変換（「境界での変換は暴露ではない」） | **高**。3 箇所とも微妙に違う言い方をしており、3 番目だけが「必要性を 1 件ずつ見る」という**追加条件**を課している（M4 の原因）。1 箇所に集約し他は参照にすべき |
| D2 | **`get_` 接頭辞の禁止** | `factory-naming.md` §禁止パターン（C-GETTER 違反）／同 §機械化の候補 2 ／`field-visibility.md` §ルール（「`get_` 接頭辞は付けない」）／`ubiquitous-language.md` §禁止パターン（`set_`/`get_`/`update_`）／`gateway-taxonomy.md` §2b（Repository で `load`/`get`/`fetch` を使わない） | **高**。5 箇所・**4 通りの射程**（ファクトリ／アクセサ／ドメインメソッド／Repository 動詞）・**3 通りの理由**（C-GETTER / house style / ドメイン語）。実測の `EventStore::get_latest_snapshot_by_id` はこの 5 つのどれにも該当しない（M1）— 重複しているのに穴が開いている典型 |
| D3 | **機械化の優先順（型 → 既存 lint → `cargo lint`）＋ 赤例テスト必須** | `README.md` 冒頭／`README.md` §機械化ロードマップ／`factory-naming.md` §機械化の候補／`interior-mutability.md` §機械化の候補／`gateway-taxonomy.md` §機械強制の候補 | 中。5 箇所に同文が散る。README §機械化ロードマップ が新設されたので、**規則側の 3 箇所は「README の方針に従う」の 1 行で足りる** |
| D4 | **「同じ用途に複数の入口を残さない」** | `factory-naming.md` §禁止パターン 2・6 ／同 §原則（`try_new` を並立させない）／同 §機械化の候補 3 ／`no-backward-compatibility.md` §ルール全体 | 中。factory-naming 側は毎回 no-backward-compatibility へリンクしているので**運用は成立している**が、同一趣旨が 4 回書かれている |
| D5 | **ビルダー／`with_*` / `to_builder()` の説明** | `factory-naming.md` §既存の値から 1 つだけ変える（10 行）／`good-examples.md` §不変な値を 1 つだけ変える（12 行、同じコード例と同じ理由付け「`depth(x)` だと depth を返すに読める」） | 中。good-examples は「スニペットを書き写さずファイルを指す」と自ら宣言しているのに、ここだけ**説明ごと写している** |
| D6 | **`find()` の廃止（C4 改訂 2026-08-23 / ADR-008）** | `gateway-taxonomy.md` §2b ／`use-case-rules.md` §4 | 低。両方に同じ日付・同じ根拠が書かれている |
| D7 | **`pub(crate)` は「単位をクレートと取った告白」** | `abstract-data-type.md` §カプセル化の単位 ／`field-visibility.md` §改訂（引用ブロックで再掲） | **低（許容）**。後者は引用と出典明示なので、正しい重複の作法。他の重複もこの形にすればよい |

---

## 6. 欠落・例外の扱いの不統一

### 6-1. 例外の作法が 5 通り（コンダクタの見立ては 3 通りだが、実際は 5 通り）

| 段階 | 規則 | 文面 |
| --- | --- | --- |
| **例外なし・抑制不可** | `field-visibility.md` | 「本ルールに例外は認めない」「抑制コメントは受け付けない」（※実装と食い違う — M6） |
| **オーナー許可＋コード上のコメント** | `interior-mutability.md` | 「採る側が強い理由を示し、**オーナーの許可を得て**、その理由をコード上のコメントに残す」 |
| **オーナー許可＋理由コメント** | `command-query-separation.md` | 「**オーナーの許可を得て**違反を許容し、理由をコメントに書く」 |
| **doc コメントに理由（許可不要）** | `ubiquitous-language.md` / `factory-naming.md` | 「例外はある。ただし理由が要る」「なぜ表に載せなかったかを doc に一行書く」 |
| **lint 抑制コメントに理由** | `tell-dont-ask.md` / `check.rs` 実装 | `// amadeus-lint: allow(<rule>) — 理由` |
| **規定なし** | `gateway-taxonomy.md` / `module-visibility.md` / `use-case-rules.md` / `error-handling.md` / `cqrs-boundaries.md` | 例外条項が**無い**。禁止だけがある |

**問題**: 「規定なし」の 5 本のうち少なくとも 2 本には**実在する例外**がある — `gateway-taxonomy` の
Store/Reader 禁止に対する `EventStore` / `JournalReader`（C1）、`cqrs-boundaries` の
「アダプタ層は両契約を実装してよい」は §対象外 にあるが**例外の申請方法は無い**。規定が無い規則に
例外が現れると、**規則を破ったのか例外なのかが記録として残らない**。

**推奨**: 段階を 4 つに正規化し（`例外なし` / `理由を doc に記述` / `lint 抑制＋理由` /
`オーナー許可＋理由`）、13 本すべてのヘッダに `**例外**: <段階>` を 1 行足す。ヘッダ 1 行なので
コストは低く、「この規則の例外はどう通すのか」が毎回そのファイルの冒頭で分かる。

### 6-2. 射程（対象／対象外）を宣言していない規則が 4 本

`## 対象` / `## 対象外` / `## 適用領域` のいずれも持たないのは
**`field-visibility.md` / `module-visibility.md` / `gateway-taxonomy.md` / `use-case-rules.md`** の 4 本
（実測 grep）。うち `field-visibility` は「例外なし」を宣言している**にもかかわらず**、
`abstract-data-type.md` §対象外 はワイヤ表現型と `modules/shared/` を除外している。
**土台の対象外が派生規則に伝播していない**（M6 の実装との食い違いはここから来ている）。

### 6-3. 規則が扱うと宣言した範囲の中で、抜けている場面

| # | 抜けている場面 | どの規則が扱うべきか |
| --- | --- | --- |
| G1 | **値受け（`self`）レシーバのメソッド** — `build()` / `into_*` / `with_*` | `command-query-separation.md`（M3） |
| G2 | **ES 基盤ポートのメソッド語彙** — `EventStore` / `JournalReader` の動詞。実測の `persist_event` / `get_latest_snapshot_by_id` / `events_after` / `advance_checkpoint` はどの語彙規則の射程にも入らない | `gateway-taxonomy.md`（M1） |
| G3 | **`async` trait の扱い** — 実測ではポート 4 本すべてが `async fn` を持つが、非同期の可否・`Send` 境界・戻り値契約について規則が 1 行も無い | 新規、または `use-case-rules.md` |
| G4 | **テスト関数の命名** — 実測では `the_snapshot_write_makes_the_aggregate_readable_again` のような**文型の長い名前**が全域で一貫している。明らかに規約だが、正本に記述が無い。新規参加者（人・エージェント）は既存コードを読んで推測するしかない | 新規（`ubiquitous-language.md` の対象外にテスト名が入るのかも不明） |
| G5 | **doc コメントの言語** — 実測では日本語。`error-handling.md` は `# Errors` セクションを要求するが、その中身を何語で書くかは無規定 | `error-handling.md` または新規 |
| G6 | **「必要性」の判定者と記録先** — `abstract-data-type.md` は「1 件ずつ見る必要がある」と言うが、誰が見て、結論をどこに残すかが無い。他の規則（UL / factory-naming）は doc コメントを要求しているので、ここだけ作法が欠けている | `abstract-data-type.md`（M4） |
| G7 | **`Impl` が複数 trait を実装してよいか** | `gateway-taxonomy.md` §5（m5） |

---

## 7. 失効した記述

`gateway-taxonomy.md` §4 と同種の失効を全ファイルで探した。**5 件見つかった**（うち 2 件が Critical）。

| # | 場所 | 失効している前提 | 何によって崩れたか | 深刻度 |
| --- | --- | --- | --- | --- |
| S1 | `use-case-rules.md` §4「CQRS を導入せずに（gateway-taxonomy.md — CQRS 不採用）」 | CQRS 不採用 | ADR-001 / ADR-003 / ADR-004 / ADR-009、`cqrs-boundaries.md`（2026-08-24）、`gateway-taxonomy.md` §4 の改訂（2026-08-24）。**改訂した本人が、その改訂を引用している側を直していない** | **Critical**（C2） |
| S2 | `field-visibility.md` §ルール line 15「`pub(crate)` は同一クレート内の実装詳細共有にのみ許す」 | `pub(crate)` の条件付き許可 | 同ファイル §改訂 2026-08-24（例外を認めない） | **Critical**（C3） |
| S3 | `gateway-taxonomy.md` §1「Gateway 責務は 2 つだけ」／§3「Store / Reader 造語は禁止」 | ポートの分類が 2 つで閉じている／`Store`・`Reader` で終わる use-case 層 trait が存在しない | `cqrs-boundaries.md`（`EventStore` / `JournalReader` を正規ポートとして導入、2026-08-24） | **Critical**（C1・M1） |
| S4 | `gateway-taxonomy.md` §適用の帰結 表 | `FsWorkspaceLock` が現役であること／`pub(crate)` への降格が望ましいこと | ADR-007（ロック退役。同ファイル §1b では処理済み）／`field-visibility.md` §改訂 | Minor（m3） |
| S5 | 8 本の規則ヘッダの「ルール化予定 / 候補」 | 「規則側に予定を書く」運用 | `README.md` §機械化ロードマップ（2026-08-26 21:13 新設。「個々の規則に『予定』と書き足すのをやめる」） | Major（M5） |

### 参考: 正本の外にある失効記述（本監査の対象外だが、この正本を説明している）

`aidlc/spaces/default/memory/team.md` §Code Style は coding-rules を
「**6 規則 + README**——ルール 3 本が既に機械強制、赤例テスト 31 本」と説明している。
実測は **13 規則 + README + good-examples + 土台**、機械強制は **2 本**（`checkbox-vocabulary` /
`no-public-fields`）。`team.md` は practices-discovery の affirmation gate 経由でしか編集しない
運用なので直し方は別問題だが、**正本を参照する側の説明が失効している**ことは記録しておく。

### 失効していなかったもの（確認済み）

- `tell-dont-ask.md` の `reap_eligible` 参照 — 「退役済み — ADR-007 / Bolt B5。以後は履歴としての例」と
  **明示済み**。`domain-equality.md` の `OwnerStamp`、`use-case-rules.md` §3 の `reap_eligible` も同様に
  処理されている。**この処理の作法自体は正しく機能している**（S1・S2 でだけ適用されていない）。
- **相対リンク 11 本すべて到達可能**（実測。`docs/specs/` 4 本、`modules/` 6 本、
  `naming-audit-report.md` 1 本）。`good-examples.md` が自ら「リンク切れは所見」と定めた基準で、
  現時点の所見は **0 件**。

---

## 8. 機械強制の記述の正確さ

### 実測（2026-08-26T12:16Z 時点）

`tools/lint/src/check.rs` に実装されているルール ID は **2 本のみ**:
`checkbox-vocabulary`（check.rs:18）/ `no-public-fields`（check.rs:27）。他のルール ID は
ソース中に存在しない。抑制は `is_suppressed()`（check.rs:91）が
`// amadeus-lint: allow(<rule-id>) <理由>` を直前行に要求し、**理由の実質的な文字が無ければ抑制しない**
（`has_reason()`、check.rs:106）。

`modules/` 配下の実測: 無制限 `pub` フィールド **0 件**、`pub(crate)` フィールド **16 件**。

### README §機械化ロードマップ（21:13 新設）の正確さ

**正確である。** 「実装済み 2 本」「予定/候補 8 本」「予定が実装の 4 倍」はいずれも実測と一致する。
着手条件 3 つ・優先順 4 本・実装しないと決めた 2 件・未定 6 件の切り分けも、
`factory-naming.md` §やってはいけない機械化 の判断と整合している。**この節の新設は本監査の
指摘事項を先回りで潰しており、§8 で挙げるべきだった問題の大半は既に解決済みである。**

### それでも残る不正確さ

| # | 場所 | 記述 | 実態 | 深刻度 |
| --- | --- | --- | --- | --- |
| E1 | 8 本の規則ヘッダ | 「`cargo lint` ルール化予定 / 候補」 | README が「規則側に予定を書かない」と定めた（21:13）のに未反映。**README と規則本文が食い違っている** | Major（M5） |
| E2 | `README.md` 表・tell-dont-ask 行 | 機械強制「`cargo lint`（checkbox-vocabulary）」 | `checkbox-vocabulary` が見るのは**`CheckboxState` の変種を呼出側で `matches!` 列挙している場合だけ**（check.rs:126 周辺）。Tell-Don't-Ask 規則の本体（判断の持ち出し全般）は機械強制されていない。**規則が機械強制されていると読める** | Major |
| E3 | `README.md` 表・field-visibility 行 | 機械強制「`cargo lint`（no-public-fields。境界拡張は機械化ロードマップ 2）」 | 記述は正確。ただし**現状の検出範囲では `pub(crate)` 16 件が野放し**であり、規則本文は「例外なし」と宣言している。**宣言と強制の差が 16 件**あることは表からは読めない | Minor |
| E4 | `field-visibility.md` §検出境界の拡張 | 「抑制コメントは受け付けない実装にする」 | 現行実装は受け付ける。しかも `no-public-fields` の抑制を**緑テストで固定している**（check.rs:782-790、境界 DTO の serde 表現）。規則どおり実装すると既存テストが赤になる | Major（M6） |
| E5 | `README.md` §機械化ロードマップ 優先順 表 | factory-naming の機械化を「ロードマップ 1・3」と対応づけ（1 = 構造体リテラル 1 箇所、3 = inherent `fn from(`） | `factory-naming.md` §機械化の候補 が挙げるのは **inherent `fn from(` / `get_` 接頭辞 / `new` と `try_new` の共存** の 3 本で、**「構造体リテラル 1 箇所」は含まれていない**（別節 §検査可能な性質 にある）。逆に候補 2（`get_` 接頭辞）と候補 3（`new`/`try_new`）は、ロードマップの優先順・実装しない・未定の**どのリストにも現れず消えている** | Major |
| E6 | `use-case-rules.md` ヘッダ | 機械強制「Cargo のクレート分離（実装依存 = E0432）」 | 正確。`cqrs-boundaries.md` の「クレート分離＝ビルドで落ちる」も同様に正確で、**この 2 本だけが本当に機械強制されている規則**である（lint ではなく型システム／ビルドによる強制）。README 表の該当セルもこれを正しく反映している | — |

### 「予定が多すぎないか」への回答

**多すぎる、という判断は正しい。** ただし **21:13 の README 改訂で構造的には解決している** —
予定の管理を 1 箇所に集め、着手条件 3 つを課し、4 本に絞り、2 本は「実装しない」と明記し、
6 本は「未定」に落とした。**残っているのは実行（E1: 8 本のヘッダ書き換え）だけ**である。

補足すると、実質的な問題は「予定が 8 本ある」ことではなく **「規則が機械強制されていると
読めるのに、実際に見ているのは規則のごく一部」**（E2）のほうである。`checkbox-vocabulary` は
Tell-Don't-Ask の 1 事例しか見ないし、`no-public-fields` は宣言された禁止範囲の半分
（無制限 `pub` のみ）しか見ない。README 表の「機械強制」欄は **`cargo lint`（ルール名）** ではなく
**`cargo lint`（ルール名 — 検出範囲）** と書くほうが正確である。

---

## 9. 総括 — 規則セットとして使える状態か

### 判定: **現状のままでは使えない。Critical 3 件を直せば使える。**

規則の**内容**は概ね良い。射程を誤って広げている箇所（§4 のこじつけ 2 本）はあるが、
1 本ずつ読めば意味が通り、実在ファイルへのリンクは全部生きており、失効処理の作法
（`reap_eligible` / `OwnerStamp` / `WorkspaceLock` の「退役済み」注記）も**確立している**。
問題は作法が確立していることではなく、**その作法が最新の 3 箇所に適用されていない**ことである。

使えない理由は 3 つに絞られる。

1. **同じ正本の中で「CQRS 不採用」と「CQRS 採用」が両立している**（C2）。機械的な読み手は停止する。
2. **`gateway-taxonomy` が禁止した造語を、`cqrs-boundaries` と `command-query-separation` が
   正規のポート名・適合例として掲示している**（C1）。しかも実在する（`modules/core/use-case/src/
   orchestration/{event_store,journal_reader}.rs`）。`gateway-taxonomy` §機械強制の候補 1 を
   実装したら CI が赤になる。
3. **`field-visibility` の `## ルール` が、同ファイルの改訂と正反対のことを書いている**（C3）。
   `## ルール` だけ読む読み手（＝一次読解経路）は古いほうに従う。

### 規則を増やす前にやるべきこと（優先順）

**1. Critical 3 件を直す（30 分程度）。** いずれも 1〜3 行の編集で済む。
   - `use-case-rules.md` §4 の「CQRS を導入せずに（gateway-taxonomy.md — CQRS 不採用）」を削除し
     `cqrs-boundaries.md` を参照に差し替える
   - `field-visibility.md` §ルール line 15 を削除（または失効注記）
   - `gateway-taxonomy.md` §1 に第三の責務（永続化基盤ポート）を追加し、§3 の造語禁止に
     `EventStore` / `JournalReader` の例外を理由付きで明記する

**2. 衝突解決の優先順を 1 節書く（§3 の 10 行）。** 13 本を全部覚えている読み手を前提にできなく
   なった以上、これが無いと同種の Critical が再生産される。**規則を 1 本増やすより効果が大きい。**

**3. 例外の作法を 4 段階に正規化し、13 本のヘッダに `**例外**:` を 1 行足す（§6-1）。**
   現状 5 通りあり、5 本は無規定。無規定の規則に実在の例外があると（C1 がまさにそれ）、
   破ったのか例外なのかが記録に残らない。

**4. README §機械化ロードマップ の新方針を 8 本のヘッダへ反映する（E1）。** 方針を書いた
   ファイルと、方針の対象ファイルが食い違っている状態を残さない。E5（factory-naming の候補
   2・3 が消えている）も同時に処理する。

### 率直な所見

- **土台の主張は 6 本中 2 本しか素直に成立しない**（§4）。「土台」という言葉は優先順を含意するが、
  優先順は明文化されていない（§3）。含意だけがあって規則が無いのが今いちばん危うい。
  土台の要約表が原典を誤って要約している箇所（factory-naming = 「`parse` が唯一の入口」）は、
  **要約を信じた読み手が原典と逆のことをする**形の誤りなので、優先して直したほうがよい。
- **コンダクタが危ぶんだ三角（tell-dont-ask × factory-naming × abstract-data-type）は、
  読む限り整合している。** `factory-naming.md` は §対象外 で変換メソッド（`as_*`/`to_*`/`into_*`）を
  自ら射程外とし C-CONV に委ねているので、`to_usize()` の是非については何も主張していない。
  危ういのは **field-visibility / good-examples（アクセサは既定で置く）× abstract-data-type /
  tell-dont-ask（アクセサは既定で疑う）** のほうである（M4）。**書いた本人の見立てと、
  独立に読んだ結果がずれた唯一の箇所**なので、ここは特に確認をおすすめする。
- 規則の**数**は問題ではない。13 本は多いが、1 ルール 1 ファイルで各 20〜230 行、射程も概ね
  分かれている。問題は**規則間の関係（優先順・導出・例外・重複）が規則と同じ密度で書かれていない**
  ことであり、これは規則を増やしても減っても残る。**次に書くべきは 14 本目の規則ではなく、
  13 本の関係を定義する 1 節である。**

---

## Review

**Verdict:** NOT-READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-08-26T12:16:46Z
**Iteration:** 1

### Findings

| # | Severity | Location | Finding | Recommendation |
|---|---|---|---|---|
| 1 | Critical | `gateway-taxonomy.md` §1/§3 ↔ `cqrs-boundaries.md` / `command-query-separation.md` | 禁止された `Store`/`Reader` 造語のポートが実在（`use-case/src/orchestration/{event_store,journal_reader}.rs`）し、他 2 規則が正規例として掲示 | §1 に第三の責務を追加、§3 に例外を理由付きで明記 |
| 2 | Critical | `use-case-rules.md` §4 | 「CQRS 不採用」の失効前提が残存。`cqrs-boundaries.md` と正面衝突 | 該当括弧を削除し `cqrs-boundaries.md` を参照 |
| 3 | Critical | `field-visibility.md` line 15 vs §改訂 2026-08-24 | 同一ファイル内で `pub(crate)` を「許す」と「例外なし」が併存 | line 15 を削除または失効注記 |
| 4 | Major | 正本全体 | 規則衝突時の優先順が 1 つも明文化されていない | README に 10 行の「規則が衝突したら」を新設 |
| 5 | Major | `abstract-data-type.md` §ここから導かれる規則 | 6 本の導出主張のうち成立 2・半分 2・こじつけ 2。factory-naming の要約が原典と食い違う | 「導出」と「関連」に分割し、要約を原典に合わせる |
| 6 | Major | `command-query-separation.md` | 値受け（`self`）レシーバが判定フローに無く、ビルダーの扱いが factory-naming と逆 | 「`self` レシーバは対象外」を明記し許容違反表から Builder 行を削除 |
| 7 | Major | `field-visibility.md` §検出境界の拡張 ↔ `check.rs` | 「抑制不可」と宣言する一方、実装は境界 DTO の抑制を緑テストで固定 | field-visibility に §対象外 を新設し土台と範囲を揃える |
| 8 | Major | 8 規則のヘッダ ↔ `README.md` §機械化ロードマップ | 「規則側に予定を書かない」新方針が対象 8 本に未反映。factory-naming の候補 2・3 がロードマップから消失 | 8 本のヘッダ置換と候補の突合 |
| 9 | Major | 例外条項（13 本） | 作法が 5 通り、5 本は無規定。実在の例外が記録されない | 4 段階に正規化しヘッダに `**例外**:` を 1 行 |
| 10 | Minor | 索引・重複・射程（m1〜m6 / D1〜D7 / G1〜G7） | 索引表に土台と good-examples が無い、導出リンクが片方向（6 本中 2 本）、`get_` 禁止が 5 箇所 4 射程、`self` レシーバ・ES 語彙・async・テスト命名が無規定 | 本文 §5 / §6 の各推奨 |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| 相対リンク到達性（11 本） | PASS（0 件切れ） | `good-examples.md` の自己基準で所見なし |
| `tools/lint/src/check.rs` ルール ID 抽出 | 実装 2 本（`checkbox-vocabulary` / `no-public-fields`） | README §機械化ロードマップ の「実装済み 2 本」は正確 |
| `pub` / `pub(crate)` フィールド計数 | `pub` 0 件 / `pub(crate)` 16 件 | field-visibility の「例外なし」宣言に対し 16 件が未強制 |
| `pub trait` 宣言（use-case 層） | `EventStore` / `JournalReader` / `WorkflowExecutionRepository` / `WorkflowDefinitionRepository` | 前 2 本が gateway-taxonomy §3 の禁止語で終わる（Finding 1 の直接証拠） |

### Summary

規則の内容と失効処理の作法は確立しているが、その作法が直近 3 箇所（CQRS 不採用の失効前提、
`pub(crate)` の自己矛盾、Store/Reader 造語の実在例外）に適用されておらず、機械的な読み手が
停止する。加えて 13 本になった規則の**関係**（優先順・導出・例外・重複）が規則と同じ密度で
書かれていないため、同種の矛盾が再生産される構造にある。次に書くべきは 14 本目の規則ではなく、
13 本の関係を定義する 1 節である。
