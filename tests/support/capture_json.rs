//! 外部実測の証跡もプロジェクトの正規JSON経路で書く。
#![allow(clippy::unwrap_used)]
use base64::Engine as _;
use core_infrastructure::canon_json::{
    JsonValue, ObjectMembers, SerializationProfile, serialize, to_value,
};
pub(super) fn value<T: serde::Serialize>(value: &T) -> JsonValue {
    to_value(value).unwrap()
}
pub(super) fn object<const N: usize>(entries: [(&str, JsonValue); N]) -> JsonValue {
    let mut fields = ObjectMembers::new();
    for (key, value) in entries {
        fields.insert(key, value);
    }
    JsonValue::Object(fields)
}
pub(super) fn pretty(value: &JsonValue) -> String {
    serialize(value, SerializationProfile::ContractPretty)
}
pub(super) fn literal<T: serde::Serialize>(input: &T) -> String {
    serialize(&value(input), SerializationProfile::ContractCompact)
}
pub(super) fn file_output(output: &std::process::Output, key: &str, bytes: &[u8]) -> JsonValue {
    optional_file_output(output, key, Some(bytes))
}
pub(super) fn optional_file_output(
    output: &std::process::Output,
    key: &str,
    bytes: Option<&[u8]>,
) -> JsonValue {
    object([
        ("exit_code", value(&output.status.code())),
        (
            "stdout_base64",
            value(&base64::engine::general_purpose::STANDARD.encode(&output.stdout)),
        ),
        (
            "stderr_base64",
            value(&base64::engine::general_purpose::STANDARD.encode(&output.stderr)),
        ),
        (
            key,
            value(&bytes.map(|bytes| base64::engine::general_purpose::STANDARD.encode(bytes))),
        ),
    ])
}
