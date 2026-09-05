# U4 リードモデル更新の規則

## 正本と出典

[要求](../../../inception/requirements-analysis/requirements.md)、[要求割当](../../../inception/units-generation/unit-of-work-story-map.md)、[Unit定義](../../../inception/units-generation/unit-of-work.md)、[共有契約](../../../inception/contract-design/contract-summary.md)、[構成](../../../inception/domain-design/components.md)、[確認回答](functional-design-questions.md) を根拠とする。cqrs-boundaries は active space の coding-rules/cqrs-boundaries.md を指す。

BR3.1〜BR3.4 は障害後の冪等性を満たすための追加設計。現行の「ファイル追記後にチェックポイントを進める」実装だけでは満たせない。

## 規則の正本

```yaml
rules:
  - id: BR1.1
    category: constraint
    statement: "取得ループと純粋投影核を分離する"
    applies_to: ["JournalRecord","PublicationBatch"]
    trigger: "ジャーナルの取得時"
    logic: "IF 投影を実行する THEN 取得・チェックポイント・書込は取得側が所有し、投影核は受け取ったイベント材料から出力を計算する"
    violation: "責務を越えた依存を設計・依存検査で拒否"
    source: "FR1.1; cqrs-boundaries"
  - id: BR1.2
    category: validation
    statement: "横断通番と集約内通番を混同しない"
    applies_to: ["JournalRecord","ProjectionCursor"]
    trigger: "入力の受理時"
    logic: "IF 読めない行・異なる集約の材料・契約外の値がある THEN 誤った状態を投影せず失敗する。正当な別集約の横断通番の間隔は欠落とみなさない"
    violation: "入力位置と原因を返し確定位置を進めない"
    source: "NFR3"
  - id: BR1.3
    category: constraint
    statement: "すべての出力を同じ採取断面から計画する"
    applies_to: ["PublicationBatch","StructuredProjection"]
    trigger: "投影計画時"
    logic: "IF 差分がある THEN 一つの確定した履歴断面を採取し、その末尾と出力の as_of を一致させる。採取後の追加イベントは次回に回す"
    violation: "断面を一致させられなければ公開を開始しない"
    source: "NFR3"
  - id: BR2.1
    category: constraint
    statement: "監査の語彙・フィールド順・時刻・文言を観測契約に揃える"
    applies_to: ["AuditBlock"]
    trigger: "監査ブロック生成時"
    logic: "IF 対応するイベントを描く THEN 契約で定めた行列を、発生時刻と確定した表示材料から生成する。未対応語彙を成功したことにしない"
    violation: "比較不一致または投影不能として扱う"
    source: "FR1.1; FR5.4（描画側）; NFR1"
  - id: BR2.2
    category: authorization
    statement: "所有対象の管理部分だけを更新する"
    applies_to: ["OutputPlan"]
    trigger: "出力計画と適用時"
    logic: "IF 対象が自クローンの管理対象であり現在内容が計画の前提に一致する THEN 更新する。規則ファイルの所有外部分と他クローンのシャードは保持する"
    violation: "所有または内容が一致しない場合は競合として停止"
    source: "FR1.1; NFR1; NFR3"
  - id: BR2.3
    category: policy
    statement: "構造化面はドメイン判断の結果を保持する"
    applies_to: ["StructuredProjection"]
    trigger: "構造化投影時"
    logic: "IF 読取用の行を生成する THEN 集約の規則を参照して結果を投影し、取得ループやDAOに別の業務判断を実装しない"
    violation: "規則の二重実装を設計・依存検査で拒否"
    source: "FR1.1; cqrs-boundaries"
  - id: BR3.1
    category: constraint
    statement: "出力に先立って再開可能な計画を保持する"
    applies_to: ["PublicationBatch","OutputPlan"]
    trigger: "公開開始時"
    logic: "IF 書込を始める THEN 入力断面・適用前後の同一性・確定出力バイトを耐久的に保持済みでなければならない"
    violation: "計画保存失敗時は出力も確定位置も変更しない"
    source: "NFR3"
  - id: BR3.2
    category: constraint
    statement: "同じ計画の再試行で同じ監査行を重複させない"
    applies_to: ["PublicationBatch","OutputPlan","AuditBlock"]
    trigger: "出力または復旧時"
    logic: "IF 出力が計画適用後と一致する THEN 書かずに反映済みとする。適用前なら計画を適用する。追記途中の厳密な接頭辞なら未完部分だけを補完する。それ以外は競合として止める"
    violation: "無条件の再追記を禁止し、曖昧な出力を上書きしない"
    source: "NFR3; FR1.1"
  - id: BR3.3
    category: constraint
    statement: "必要な出力の確認後に構造化面と確定位置を一緒に確定する"
    applies_to: ["ProjectionCursor","PublicationBatch","StructuredProjection"]
    trigger: "計画完了時"
    logic: "IF すべてのファイル出力が計画どおり反映済み THEN BR5.3に従う共有面の公開または維持・個別位置・served_by・計画完了を不可分に確定する。候補のas_ofはtargetと同一だが、有効な既存共有面が新しければ後退させず利用する"
    violation: "一部書込の失敗で確定位置を進めない。確定処理を再試行する"
    source: "NFR3"
  - id: BR3.4
    category: constraint
    statement: "同じ対象へ競合する計画を適用しない"
    applies_to: ["ProjectionCursor","PublicationBatch","OutputPlan"]
    trigger: "開始・復旧・確定時"
    logic: "IF 未完計画がある THEN 復旧またはBR5.2の置換を先に行う。spaceの共有面、ファイルの正準パス順で排他し、開始位置・active_generation・対象所有を再検査する。supersededや古い世代の書込を拒否する"
    violation: "競合を返し既存計画を保持する"
    source: "NFR3"
  - id: BR4.1
    category: policy
    statement: "初回は構造化面だけを用意できる"
    applies_to: ["ProjectionCursor","StructuredProjection"]
    trigger: "互換ファイルの投影先が未作成の初回起動"
    logic: "IF 初回の構造化のみの経路が要求された THEN 構造化面とその専用確定位置を更新する。後の互換ファイル投影の未処理範囲を既処理扱いにしない"
    violation: "別出力の未反映を完了と認めない"
    source: "FR1.1; NFR3"
  - id: BR4.2
    category: policy
    statement: "規則入力の変更をジャーナル差分の有無から独立して反映する"
    applies_to: ["SteeringProjection"]
    trigger: "参照規則の同期時"
    logic: "IF 読取内容の同一性が保存済みと違う THEN 規則の投影を差し替える。同じなら書込不要。ジャーナルの確定位置は動かさない"
    violation: "存在する入力の読取失敗・整形失敗は成功扱いしない"
    source: "FR1.1; NFR1"
  - id: BR4.3
    category: validation
    statement: "失敗を出力段階と対象付きで伝え、同じ入力から回復する"
    applies_to: ["PublicationBatch","OutputPlan"]
    trigger: "入力・変換・公開・確定の失敗時"
    logic: "IF 処理が失敗する THEN 対象と段階を返し、未完計画を保持する。正常な空差分と破損・読取失敗を区別する"
    violation: "欠落を空入力として継続しない"
    source: "NFR3"
  - id: BR4.4
    category: constraint
    statement: "他シャードを読取専用で合流し表示順と因果関係を区別する"
    applies_to: ["AuditBlock"]
    trigger: "横断読取時"
    logic: "IF 複数シャードを表示する THEN 時刻と位置による再現可能な順序を用いる。同秒の別シャード間に、記録されていない承認等の因果関係を仮定しない"
    violation: "曖昧な因果関係で処理を承認・完了しない"
    source: "FR1.1; NFR3"
  - id: BR5.1
    category: constraint
    statement: "確定済み位置でも新しい世代で再生成できる"
    applies_to: [ProjectionCursor, PublicationBatch, OutputPlan, StructuredProjection]
    trigger: "再生成要求または出力欠落"
    logic: "IF rebuildを受理する THEN 新しい要求IDと計画世代を使い、targetは個別位置と共有面の記録済み位置以上とする。同じ位置の100→100や空履歴の0→0も許す。同じ要求の再送は同じ計画へ解決し、committedは無操作"
    violation: "不足した履歴を正当な再生成として受理せず、位置を下げない"
    source: "NFR3; FR1.1; 確認回答R-01"
  - id: BR5.2
    category: constraint
    statement: "利用者の変更と確認済み出力を保持してblocked計画を置換する"
    applies_to: [PublicationBatch, OutputPlan, AuditBlock, ProjectionCursor]
    trigger: "競合の解決後"
    logic: "IF 現在内容・所有部分・反映済みブロックを一意に照合できる THEN 解決記録を保存し、旧計画のsuperseded・新世代のprepared・active_generationを不可分に更新する。確認済みブロックは引き継ぎ、未反映分だけ適用する"
    violation: "曖昧な反映状態を推測せずblockedを維持。旧計画を削除・再開しない"
    source: "NFR3; NFR1; FR1.1; 確認回答R-02"
  - id: BR5.3
    category: constraint
    statement: "共有構造化面をspace単位で公開し古い断面へ戻さない"
    applies_to: [SharedProjectionHead, StructuredProjection, PublicationBatch, ProjectionCursor]
    trigger: "通常・初回構造化のみ・再生成の確定時"
    logic: "IF 同じ規約版の有効な共有面のas_ofがtarget未満 THEN 候補の行集合とheadを公開する。同値なら候補と既存の行集合の一致を検査し、一致時だけ共有面を維持してserved_byを記録し、不一致は破損として停止する。targetを超える場合は新しい共有面を維持してserved_byを記録する。欠落・破損は記録済み位置以上のrebuildで修復し、規約不一致は受理済みの版へ計画を置換する。いずれもheadの世代を排他下で検査する"
    violation: "古い候補で共有面を後退させない。欠落した新しい面を古い候補で代用せず、再生成指定だけで規約版を変更しない"
    source: "NFR3; NFR1; 確認回答R-03; 仕様11号§4.1"
```

## 規則の要約

| ID | 規則 |
|---|---|
| BR1.1 | 取得ループと純粋投影核を分離する |
| BR1.2 | 横断通番と集約内通番を混同しない |
| BR1.3 | すべての出力を同じ採取断面から計画する |
| BR2.1 | 監査の語彙・フィールド順・時刻・文言を観測契約に揃える |
| BR2.2 | 所有対象の管理部分だけを更新する |
| BR2.3 | 構造化面はドメイン判断の結果を保持する |
| BR3.1 | 出力に先立って再開可能な計画を保持する |
| BR3.2 | 同じ計画の再試行で同じ監査行を重複させない |
| BR3.3 | 必要な出力の確認後に構造化面と確定位置を一緒に確定する |
| BR3.4 | 同じ対象へ競合する計画を適用しない |
| BR4.1 | 初回は構造化面だけを用意できる |
| BR4.2 | 規則入力の変更をジャーナル差分の有無から独立して反映する |
| BR4.3 | 失敗を出力段階と対象付きで伝え、同じ入力から回復する |
| BR4.4 | 他シャードを読取専用で合流し表示順と因果関係を区別する |
| BR5.1 | 確定済み位置でも新しい世代で再生成できる |
| BR5.2 | 利用者の変更と確認済み出力を保持してblocked計画を置換する |
| BR5.3 | 共有構造化面をspace単位で公開し古い断面へ戻さない |
