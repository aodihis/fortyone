use uuid::Uuid;

pub fn generate_short_uuid() -> String {
    // Generate a random UUID
    let uuid = Uuid::new_v4();

    let uuid_bytes = uuid.as_bytes();
    let first_8_bytes: u64 = u64::from_be_bytes(uuid_bytes[0..8].try_into().unwrap());
    base62::encode(first_8_bytes)
}