use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

/// Manifest envelope version (e.g., "1.0").
const MANIFEST_TENOR_VERSION: &str = "1.0";

/// Compute SHA-256 etag from compact JSON representation.
pub fn compute_etag(bundle: &Value) -> String {
    let canonical = serde_json::to_string(bundle)
        .unwrap_or_else(|e| panic!("serialization error computing etag: {}", e));
    let hash = Sha256::digest(canonical.as_bytes());
    format!("{:x}", hash)
}

/// Wrap an interchange bundle in a TenorManifest envelope.
///
/// Keys are lexicographically sorted for spec compliance (§19).
/// `serde_json::Map` is backed by `BTreeMap` (the default when the
/// `preserve_order` feature is not enabled), so insertion order does
/// not matter — the map itself guarantees sorted output.
pub fn build_manifest(bundle: Value) -> Value {
    let etag = compute_etag(&bundle);
    let mut map = Map::new();
    map.insert("bundle".to_string(), bundle);
    map.insert("etag".to_string(), Value::String(etag));
    map.insert(
        "tenor".to_string(),
        Value::String(MANIFEST_TENOR_VERSION.to_string()),
    );
    Value::Object(map)
}
