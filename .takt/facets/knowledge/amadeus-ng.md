# amadeus-ng 知識

## このリポジトリ

AI-DLC Workflows（`vendor/aidlc-workflows/`）を Rust で再実装したもの。コマンド側・クエリ側・リードモデル更新器（RMU）を
クレートで分けた CQRS+ES の構成で、形式モデル（Quint）を `formal/` に置く。

| 場所 | 中身 |
|------|------|
| `modules/core/command/{domain,use-case,interface-adapter}` | コマンド側。集約・ユースケース・アダプタ |
| `modules/core/query/{use-case,interface-adapter}` | クエリ側。`…Dao` は `find` だけ |
| `modules/core/read-model-updater` | RMU。コマンド側とクエリ側の両方に依存してよい唯一のクレート |
| `modules/core/infrastructure` | 言語拡張（原子的 I/O・時計・ID など）だけ |
| `modules/app/aidlc` | 合成ルートと CLI（`aidlc` バイナリ） |
| `tools/lint` | `cargo lint` の本体（ワークスペース外の独立クレート） |
| `tests/golden/upstream-a277af21/` | upstream 互換のゴールデン |
| `formal/` | Quint モデル（`scripts/quint-gate.sh` で検査） |

## 文書の置き場所

- コーディング規則の正本: `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/`（`README.md` が索引）
- 設計の調査・計画の記録: `aidlc/spaces/default/knowledge/`（例: RMU の DAO 移行計画 `rmu-dao-migration-plan-20260926.md`）
- これらは参照してよい。進み具合の記録（計画の「現状」節など）は、これまでの PR と同じく作業の PR で更新してよい。
- `aidlc/spaces/*/intents/`（AI-DLC の intent 記録）は開発に使わない。

## リードモデル更新器の形（Issue #153）

更新器は「ジャーナルを読む → 投影 → 表の DAO で書く」だけ（`read-model-updater-structure.md`、オーナー裁定 2026-09-26）。

- `ReadModelWriter` は作らない。`JournalReader` はジャーナルの要素（事実と位置）を読むだけ。
- DAO は 1 表の I/O だけ（1 ファイル = 1 表）。trait は `orchestration/port/`、実装は `*_dao_impl.rs`。
- 更新器が IMMEDIATE トランザクションを開き、表の DAO へ `&mut Transaction` で渡す。
- 複数の表と処理済みの番号は同じ IMMEDIATE トランザクションで確定してよい。
- ファイルはトランザクションに入れない。追記するファイルは、ファイルごとの反映済み番号で二重追記を防ぐ。
- 移行の順序・各 PR の範囲・決定事項は `rmu-dao-migration-plan-20260926.md` が正本。
