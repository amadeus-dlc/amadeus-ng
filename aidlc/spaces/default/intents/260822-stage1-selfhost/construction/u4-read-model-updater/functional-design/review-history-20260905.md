# review-history-20260905 — functional-spec.md 旧 Review 節の退避

> 2026-09-05T11:30:33Z の独立レビュー（iteration 1、READY、R-01〜R-05 全件 Resolved）を、2026-09-07 の再走（Modify、現行コード HEAD `52fce820` への追従）で `functional-spec.md` 末尾から切り離して保存したもの。
> 切り離しの理由: レビュー要求は付録前のバイト列を束縛し、付録は要求ごとに 1 節だけを許すため（stage-protocol-reviewer 手順 1-2）。git 上の原本は `f6726802`（#112）の同ファイル末尾（:226-260）。
> 本文 §7 末尾の「以下の Review 節は過去の設計判定を保存したもの」は、本ファイルへの退避を指すものとして読む。再走の判定は `functional-spec.md` 末尾の新しい `## Review` を参照。

## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-09-05T11:30:33Z
**Iteration:** 1
**Request Challenge:** review:49ec5611dd4be343a8aa8586d33f0942

### Findings

既存IDを保持して現物を再照合した。R-01〜R-05はすべてResolvedで、新規所見はない。解消は機能設計上の判定であり、未実装機構の障害復旧試験に合格したという意味ではない。

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Major | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u4-read-model-updater/functional-design/functional-spec.md > W6、および entities.md > PublicationBatch.target_position・ProjectionCursor.position | 確定済み位置での再生成はmode=rebuildと新しいrequest_id・世代で表現する。100→100、空履歴0→0、committed要求の再送の無操作が、データ制約・BR5.1・W6・受入シナリオで一致する。 | 追加修正なし。定義された再生成シナリオを後続実装で検証する。 | Resolved |
| R-02 | Major | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u4-read-model-updater/functional-design/functional-spec.md > 第4節 blocked 行・別計画への切替、および entities.md > PublicationBatch.id・未確定計画の一意制約 | blockedからsupersededへの終端化と新prepared計画への置換がW7にある。resolution・inherited_blocks・世代によって利用者変更と確認済み出力を引き継ぎ、旧新計画の切替とactive_generation更新は不可分。旧要求と旧実行者の書込拒否も維持されている。 | 追加修正なし。置換前後の停止と旧世代拒否を後続実装で検証する。 | Resolved |
| R-03 | Major | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u4-read-model-updater/functional-design/functional-spec.md > W2手順1・5／W5、および entities.md > ProjectionCursor.projection_id・StructuredProjection | SharedProjectionHeadがspace共有面を管理し、個別カーソルと分離する。B120後のA100は共有120を維持してA100とserved_byを確定する。欠落・破損・規約不一致・旧世代を有効な面として流用しない条件も維持されている。 | 追加修正なし。共有面の公開逆転と修復の受入例を後続実装で検証する。 | Resolved |
| R-04 | Major | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u4-read-model-updater/functional-design/functional-spec.md > W6手順3・4 | 手順3で完全な出力とbefore_identity・expected_content・after_identityを確定し、手順4でW1手順8と同じ排他・再検査を経て、完全な計画・request_id・active_generationを不可分に受理する順へ修正された。計算失敗時はpreparedも世代変更も公開せず、受理直後の停止では保存済みバイトから再開できる。対応する停止点の受入例も追加された。 | 追加修正なし。 | Resolved |
| R-05 | Minor | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u4-read-model-updater/functional-design/rules.md > BR5.3.logic | BR5.3が共有as_ofのtarget未満・同値・超過を分けた。同値では候補と既存行集合の一致時だけ維持し、不一致は破損として停止するため、W8と一致する。受入シナリオにも同値・内容不一致の停止が追加された。 | 追加修正なし。 | Resolved |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| aidlc-sensor-required-sections.ts（--stage functional-design、各 --output-path） | PASS: entities / rules / functional-spec、所見0 | 追記前のH2数は3 / 3 / 7。 |
| aidlc-sensor-upstream-coverage.ts（consumes 5件・deliverables 3件を明示） | PASS: unreferenced 0 | 共有入力への参照は揃う。 |
| aidlc-sensor-traceability.ts | FAIL: missing_from_upstream_ids 36件。他のgaps / orphans / missing_from_table / invalid_entries / invalid_targetsは空 | 36件は共有story-map上のU4担当外。4入力要求と17規則の対応に、対象Unitの欠落や未定義参照はない。失敗結果自体は保持する。 |
| R-01〜R-05の机上トレース | すべて解消 | 同一位置の再生成、利用者変更を保持した置換、共有B120後のA100、共有面破損、旧世代拒否、計画計算前後の停止、同値時の内容不一致を制約・手順・状態表・受入例で照合した。 |
| linter / type-check | 対象外・未実行 | TS/JS/TSXのコード出力や該当スニペットはない。 |
| YAML参照・派生表示 | 手動照合で対応 | 今回の変更は保存順序・比較条件・受入例であり、関係図の型集合と世代管理の制約は維持されている。Mermaidパーサは今回再実行していない。 |
| アプリケーションテスト | 今回は再実行しない | 実装変更がないため、既存29テストを繰り返して新設計の証拠とはしない。過去の監査2→4プローブも現状差の証拠であり、PublicationBatch/OutputPlanの実装合格証ではない。 |

### Summary

未解消のCritical・Major・Minorは0件。R-01〜R-05の修正は整合しており、機能設計としてREADYとする。保存方式・排他の実装・保全期間等の具体化と、実際の永続化境界での障害試験は、本文第7節に記載された後続作業として残る。
