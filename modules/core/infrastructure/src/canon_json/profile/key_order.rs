//! `SerializationProfile` が選ぶオブジェクトキーの並べ方 (BR1.1)。

/// オブジェクトキーの並べ方 (BR1.1)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyOrder {
    /// 型のフィールド宣言順 / 動的マップの挿入順 (integer-like キーの先頭寄せは別途 BR1.2)。
    DeclaredOrInsertion,
    /// 全オブジェクトキーを UTF-16 コード単位順で再帰的に整列する。
    RecursiveSorted,
}
