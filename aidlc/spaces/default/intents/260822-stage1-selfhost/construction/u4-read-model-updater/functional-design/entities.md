# U4 リードモデル更新のデータモデル

## 正本の範囲

以下の YAML は本 Unit の入出力と処理管理記録の論理モデルである。PublicationBatch / OutputPlan は障害復旧のための追加設計であり、既存コードに実装済みとは扱わない。ドメインイベントや集約の所有は U2 に残す。

出典: [Unit定義](../../../inception/units-generation/unit-of-work.md)、[要求割当](../../../inception/units-generation/unit-of-work-story-map.md)、[要求](../../../inception/requirements-analysis/requirements.md)、[構成](../../../inception/domain-design/components.md)、[契約](../../../inception/contract-design/contract-summary.md)、[確認回答](functional-design-questions.md)。

## エンティティと値の正本

```yaml
entities:
  - name: JournalRecord
    description: "既存のジャーナル行を受け渡す値。集約内通番と横断通番を区別する。"
    attributes:
      - { name: global_position, type: "integer", required: true, constraints: "同じストア内で一意、正数" }
      - { name: event_id, type: "identifier", required: true, constraints: "イベント自身の識別子" }
      - { name: aggregate_id, type: "identifier", required: true, constraints: "発生元集約の識別子" }
      - { name: aggregate_sequence, type: "integer", required: true, constraints: "同じ集約内の正数通番" }
      - { name: occurred_at, type: "timestamp", required: true, constraints: "イベントの発生時刻" }
      - { name: payload, type: "record", required: true, constraints: "対応するイベント契約の材料" }
  - name: ProjectionCursor
    description: "投影先ごとに確定した処理位置。新規ドメイン集約ではなく処理の管理記録。"
    attributes:
      - { name: projection_id, type: "identifier", required: true, constraints: "ストアと投影対象を一意に特定" }
      - { name: position, type: "integer", required: true, constraints: "初期値0、確定時に単調非減少。再生成は同じ位置でも確定できる" }
      - { name: active_generation, type: "integer", required: true, constraints: "新規・再生成・置換の計画を受理するたびに増加。古い世代の書込を拒否する" }
  - name: PublicationBatch
    description: "障害後に同じ出力を再開する管理記録。本設計で追加する契約であり現実装にはない。"
    attributes:
      - { name: id, type: "identifier", required: true, constraints: "投影IDと世代の組。同じ履歴位置と規約版でも別計画を識別できる" }
      - { name: generation, type: "integer", required: true, constraints: "ProjectionCursor.active_generation と一致する世代だけが書ける" }
      - { name: request_id, type: "identifier", required: true, constraints: "要求の再送は同じ計画へ解決。意図的な再生成・置換には新しい要求IDを用いる" }
      - { name: mode, type: "enum", required: true, allowed_values: [incremental, rebuild] }
      - { name: predecessor_id, type: "optional<identifier>", required: false, constraints: "置換元の計画ID。履歴は削除しない" }
      - { name: replacement_id, type: "optional<identifier>", required: false, constraints: "superseded のとき置換先を示す。旧要求の再送にはこの状態を返すだけで書かない" }
      - { name: projection_id, type: "identifier", required: true, constraints: "ProjectionCursorへの参照" }
      - { name: start_position, type: "integer", required: true, constraints: "開始時の確定位置" }
      - { name: target_position, type: "integer", required: true, constraints: "入力断面の末尾。incremental は開始位置より大きく、rebuild は開始位置以上。空履歴の0→0も許す" }
      - { name: transform_revision, type: "identifier", required: true, constraints: "出力計画を生成した規約版" }
      - { name: state, type: "enum", required: true, allowed_values: [prepared, publishing, committed, blocked, superseded] }
      - { name: plans, type: "list<OutputPlan>", required: true, constraints: "ファイル対象ごとの順序付き計画。構造化面だけの場合は空" }
      - { name: structured_projection, type: "StructuredProjection", required: true, constraints: "確定時に公開する構造化面" }
      - { name: served_by, type: "optional<record>", required: false, constraints: "確定時の有効な共有面のspace・世代・as_of・規約版を記録。計画候補より新しい面を維持した場合も特定する" }
      - { name: resolution, type: "optional<record>", required: false, constraints: "置換時の解決判断・現在内容の同一性・保持する利用者部分・反映済み監査ブロックの対応を記録" }
  - name: OutputPlan
    description: "対象への書込前後を照合する値。再試行時も最初に保存した計画を用いる。"
    attributes:
      - { name: target, type: "path", required: true, constraints: "所有対象と管理部分を特定" }
      - { name: mode, type: "enum", required: true, constraints: "append / replace / sections" }
      - { name: before_identity, type: "record", required: true, constraints: "適用前バイトの長さ・同一性と管理境界" }
      - { name: expected_content, type: "bytes", required: true, constraints: "適用する確定バイトまたは耐久的な参照" }
      - { name: after_identity, type: "record", required: true, constraints: "適用後バイトの長さ・同一性" }
      - { name: state, type: "enum", required: true, constraints: "pending / published" }
      - { name: audit_blocks, type: "list<AuditBlock>", required: true, constraints: "監査対象の場合のみ、他の出力は空" }
      - { name: inherited_blocks, type: "list<record>", required: true, constraints: "置換元から引き継ぐ反映済みブロックのイベントID・順序・出力範囲・バイト同一性。対応を一意に確認できないものを反映済みにしない" }
  - name: SharedProjectionHead
    description: "同じspaceのジャーナル由来の共有構造化面の公開位置。個別カーソルと独立して所有する。SteeringProjectionの版は含めない。"
    attributes:
      - { name: space_id, type: "identifier", required: true, unique: true }
      - { name: as_of, type: "integer", required: true, constraints: "公開済みの履歴位置。再生成でも後退させず、面の欠落時もこの位置を保持する" }
      - { name: generation, type: "integer", required: true, constraints: "同じ位置での再生成を含め、共有面を書き替えるたびに増加" }
      - { name: transform_revision, type: "identifier", required: true }
      - { name: content_identity, type: "identifier", required: true, constraints: "行集合の同一性。欠落・破損した面を有効な既存面として流用しない" }
  - name: AuditBlock
    description: "1イベントから0個以上生成する監査ブロック。再試行の識別子は互換ファイルへ新たに印字しない。"
    attributes:
      - { name: event_id, type: "identifier", required: true, constraints: "JournalRecordへの参照" }
      - { name: ordinal, type: "integer", required: true, constraints: "同イベント内の出力順、0以上" }
      - { name: heading, type: "string", required: true, constraints: "採用された監査語彙" }
      - { name: fields, type: "ordered_record", required: true, constraints: "契約に定めたフィールド順" }
      - { name: rendered, type: "bytes", required: true, constraints: "時刻と表示材料を含む決定的な出力" }
  - name: StructuredProjection
    description: "同じ入力断面から計算した読取用の行集合。行の実スキーマは仕様11号の構造化面を参照する。"
    attributes:
      - { name: as_of, type: "integer", required: true, constraints: "採取した履歴断面の末尾" }
      - { name: rows, type: "list<record>", required: true, constraints: "キーと外部キーを持つ読取用の行、ドメイン判断の結果" }
  - name: SteeringProjection
    description: "ジャーナルとは別に変更される参照規則の投影。イベントを捏造せず更新する。"
    attributes:
      - { name: source_identity, type: "identifier", required: true, constraints: "読んだ規則内容の同一性" }
      - { name: rows, type: "list<record>", required: true, constraints: "配信用に整形した規則" }
      - { name: source_paths, type: "list<path>", required: true, constraints: "出典と適用順序" }
relationships:
  - { from: ProjectionCursor, to: PublicationBatch, cardinality: one-to-many, direction: forward, description: "同じ投影の世代を更新し、確定位置は単調非減少" }
  - { from: PublicationBatch, to: OutputPlan, cardinality: one-to-many, direction: forward, description: "ファイル出力が不要なら空" }
  - { from: PublicationBatch, to: JournalRecord, cardinality: many-to-many, direction: forward, description: "単一の入力断面を採用する。空履歴の再生成は0件" }
  - { from: JournalRecord, to: AuditBlock, cardinality: one-to-many, direction: forward, description: "出力なしのイベントもある" }
  - { from: OutputPlan, to: AuditBlock, cardinality: one-to-many, direction: forward, description: "監査対象以外は空" }
  - { from: PublicationBatch, to: StructuredProjection, cardinality: one-to-one, direction: forward, description: "同じ到達位置の候補を持つ。公開または新しい共有面の維持を確定時に判定する" }
  - { from: SharedProjectionHead, to: PublicationBatch, cardinality: one-to-many, direction: forward, description: "複数投影が同じspaceの共有面へ公開する。確定時のserved_byで利用した共有世代を特定する" }
constraints:
  - "prepared / publishing / blocked の計画は投影対象ごとに高々1件。superseded は終端で再開不可"
  - "AuditBlock の (event_id, ordinal) は出力計画内で一意。これは出力ファイルの新フィールドではない"
  - "候補の StructuredProjection.as_of は計画のtarget_positionと同一。確定時の有効な共有面は同じ規約版でtarget_position以上、served_byが利用した共有世代を特定する"
  - "共有面の公開はspace単位、ファイル公開は対象集合単位で直列化する。共有面へ古い履歴位置を上書きしない"
  - "expected_content は書込開始前に耐久的に保持し、復旧中に現在の規則から作り直さない"
  - "他クローンの監査シャードと所有外部分を計画対象に含めない"
```

## 派生表示と実装境界

関係図と処理順序は [functional-spec.md](functional-spec.md)、判断条件は [rules.md](rules.md) を参照する。管理記録の格納方式・接続・排他の具体化は後続設計の対象であり、新しいストア製品や新しいロック方式をここで採用していない。
