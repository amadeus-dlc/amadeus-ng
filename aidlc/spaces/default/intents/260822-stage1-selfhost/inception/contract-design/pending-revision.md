# pending-revision — contract-design（ステージゲート / 契約改訂で処理）

1. C3 `EventStore<AID, A, E>` の数値パラメータ `usize` → **`u64`**（実ドメイン型 `seq_nr` / `version` = u64、Bolt B3 実装）。C3 は「Rust trait が正本」と宣言しており、
   U3（Bolt B5）がユースケース層に置く trait が u64 で確定する。U5 / U6 はその trait を実装・消費するだけで別定義を持たない（型不一致は起きない）。
   contract-summary.md の C3 本文を次の契約改訂機会に u64 へ同期する（U3 FD BR1.1、nfr-design レビュー所見 3）。
