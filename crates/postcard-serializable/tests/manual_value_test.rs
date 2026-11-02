use serde_json::Value;

#[test]
fn test_value_postcard_compatibility() {
    let value = Value::String("test".to_string());

    // This should fail according to your assertion
    let result = postcard::to_allocvec(&value);
    println!("Value serialization result: {:?}", result);

    if let Ok(bytes) = result {
        let deser_result: Result<Value, _> = postcard::from_bytes(&bytes);
        println!("Value deserialization result: {:?}", deser_result);
    }
}
