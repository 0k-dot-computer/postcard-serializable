use postcard_serializable::PostcardSerializable;
use postcard_serializable_types::PostcardSerializable as PostcardSerializableTrait;
use serde::{Deserialize, Serialize};

// This should compile successfully
#[derive(Debug, Clone, Default, Serialize, Deserialize, PostcardSerializable)]
struct GoodEvent {
    id: u64,
    message: String,
    data: Vec<u8>,
}

// #[serde(default)] alone is fine — postcard writes the None discriminant byte
#[derive(Debug, Clone, Default, Serialize, Deserialize, PostcardSerializable)]
struct GoodOptionalFields {
    id: u64,
    #[serde(default)]
    name: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PostcardSerializable)]
struct NestedGoodFields {
    event: GoodEvent,
    optional: Option<GoodOptionalFields>,
}

#[derive(Debug, Clone, Default)]
struct SkippedCache;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PostcardSerializable)]
struct GoodSkippedCache {
    id: u64,
    #[serde(skip)]
    cache: Option<SkippedCache>,
}

/*
Examples of types that WILL FAIL to compile with #[derive(PostcardSerializable)]:

// HashMap is not reliably postcard-compatible
#[derive(Debug, Clone, Default, Serialize, Deserialize, PostcardSerializable)]
struct BadEventHashMap {
    data: HashMap<String, String>,
}

// serde_json::Value fails postcard deserialization
#[derive(Debug, Clone, Default, Serialize, Deserialize, PostcardSerializable)]
struct BadEventJsonValue {
    data: serde_json::Value,
}

// Trait objects are not postcard-compatible
#[derive(Debug, Clone, Default, Serialize, Deserialize, PostcardSerializable)]
struct BadEventTraitObject {
    handler: Box<dyn std::fmt::Display>,
}

// skip_serializing_if is incompatible with postcard (positional format).
// Skipping fields during serialization causes deserialization to read wrong positions.
// Use #[serde(default)] alone for optional fields.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PostcardSerializable)]
struct BadSkipSerializingIf {
    id: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    name: Option<String>,
}
*/

#[test]
fn test_good_event_compiles() {
    let event = GoodEvent::default();

    // Test that postcard serialization works
    let serialized = postcard::to_allocvec(&event).unwrap();
    let _deserialized: GoodEvent = postcard::from_bytes(&serialized).unwrap();
}

#[test]
fn test_good_optional_fields_roundtrip() {
    let event = GoodOptionalFields {
        id: 42,
        name: Some("test".to_string()),
    };

    let serialized = postcard::to_allocvec(&event).unwrap();
    let deserialized: GoodOptionalFields = postcard::from_bytes(&serialized).unwrap();
    assert_eq!(deserialized.id, 42);
    assert_eq!(deserialized.name.as_deref(), Some("test"));

    // None case also roundtrips
    let event_none = GoodOptionalFields { id: 1, name: None };
    let serialized = postcard::to_allocvec(&event_none).unwrap();
    let deserialized: GoodOptionalFields = postcard::from_bytes(&serialized).unwrap();
    assert_eq!(deserialized.id, 1);
    assert_eq!(deserialized.name, None);
}

#[test]
fn test_trait_is_implemented() {
    // Verify the PostcardSerializable trait is properly implemented
    fn requires_postcard_serializable<T: PostcardSerializableTrait>(_: T) {}

    let event = GoodEvent::default();
    requires_postcard_serializable(event);
}

#[test]
fn test_nested_fields_roundtrip() {
    let event = NestedGoodFields {
        event: GoodEvent {
            id: 1,
            message: "nested".to_string(),
            data: vec![1, 2, 3],
        },
        optional: Some(GoodOptionalFields {
            id: 2,
            name: Some("child".to_string()),
        }),
    };

    let serialized = postcard::to_allocvec(&event).unwrap();
    let deserialized: NestedGoodFields = postcard::from_bytes(&serialized).unwrap();
    assert_eq!(deserialized.event.message, "nested");
    assert_eq!(
        deserialized.optional.and_then(|value| value.name),
        Some("child".to_string())
    );
}

#[test]
fn test_skipped_cache_field_is_not_wire_checked() {
    let event = GoodSkippedCache {
        id: 9,
        cache: Some(SkippedCache),
    };

    let serialized = postcard::to_allocvec(&event).unwrap();
    let deserialized: GoodSkippedCache = postcard::from_bytes(&serialized).unwrap();
    assert_eq!(deserialized.id, 9);
    assert!(deserialized.cache.is_none());
}
