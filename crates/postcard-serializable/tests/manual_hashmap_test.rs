use std::collections::HashMap;

#[test]
fn test_hashmap_postcard_compatibility() {
    let mut map = HashMap::new();
    map.insert("key".to_string(), "value".to_string());

    // This should work or fail - let's see what happens
    let result = postcard::to_allocvec(&map);
    println!("HashMap serialization result: {:?}", result);

    if let Ok(bytes) = result {
        let deser_result: Result<HashMap<String, String>, _> = postcard::from_bytes(&bytes);
        println!("HashMap deserialization result: {:?}", deser_result);
    }
}
