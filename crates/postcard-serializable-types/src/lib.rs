/// Marker trait indicating that a type has been validated for postcard compatibility at compile time
pub trait PostcardSerializable:
    serde::Serialize + for<'de> serde::Deserialize<'de> + Default
{
    // Marker trait - presence indicates postcard compatibility verified at compile time
}

// Blanket implementations for common wrapper types
impl<T: PostcardSerializable> PostcardSerializable for Option<T> {}
impl<T: PostcardSerializable> PostcardSerializable for Vec<T> {}
impl<T: PostcardSerializable> PostcardSerializable for Box<T> {}
impl PostcardSerializable for [u8; 8] {}
impl PostcardSerializable for [u8; 16] {}
impl PostcardSerializable for [u8; 32] {}
impl<K: PostcardSerializable + Eq + std::hash::Hash, V: PostcardSerializable> PostcardSerializable
    for std::collections::HashMap<K, V>
{
}
impl<K: PostcardSerializable + Ord, V: PostcardSerializable> PostcardSerializable
    for std::collections::BTreeMap<K, V>
{
}
impl<T: PostcardSerializable + Eq + std::hash::Hash> PostcardSerializable
    for std::collections::HashSet<T>
{
}
impl<T: PostcardSerializable + Ord> PostcardSerializable for std::collections::BTreeSet<T> {}
impl<A: PostcardSerializable, B: PostcardSerializable> PostcardSerializable for (A, B) {}
impl<A: PostcardSerializable, B: PostcardSerializable, C: PostcardSerializable> PostcardSerializable
    for (A, B, C)
{
}
impl<
        A: PostcardSerializable,
        B: PostcardSerializable,
        C: PostcardSerializable,
        D: PostcardSerializable,
    > PostcardSerializable for (A, B, C, D)
{
}

// Implementations for primitive types that are postcard-compatible
impl PostcardSerializable for () {}
impl PostcardSerializable for bool {}
impl PostcardSerializable for u8 {}
impl PostcardSerializable for u16 {}
impl PostcardSerializable for u32 {}
impl PostcardSerializable for u64 {}
impl PostcardSerializable for u128 {}
impl PostcardSerializable for usize {}
impl PostcardSerializable for i8 {}
impl PostcardSerializable for i16 {}
impl PostcardSerializable for i32 {}
impl PostcardSerializable for i64 {}
impl PostcardSerializable for i128 {}
impl PostcardSerializable for isize {}
impl PostcardSerializable for f32 {}
impl PostcardSerializable for f64 {}
impl PostcardSerializable for char {}
impl PostcardSerializable for String {}

// Common external types that are "just bytes" and safe for postcard
impl PostcardSerializable for uuid::Uuid {}
impl PostcardSerializable for std::time::Duration {}
impl PostcardSerializable for std::path::PathBuf {}
