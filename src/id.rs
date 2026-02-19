use uuid::Uuid;

/// Generate a new UUID v7 identifier.
///
/// UUID v7 is time-ordered: the first 48 bits encode a Unix timestamp in
/// milliseconds, so IDs sort chronologically.
pub fn generate_id() -> Uuid {
    Uuid::now_v7()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_id_is_v7() {
        let id = generate_id();
        assert_eq!(id.get_version_num(), 7);
    }

    #[test]
    fn generated_ids_are_unique() {
        let a = generate_id();
        let b = generate_id();
        assert_ne!(a, b);
    }

    #[test]
    fn generated_ids_are_time_ordered() {
        let a = generate_id();
        let b = generate_id();
        assert!(a < b, "UUID v7 should be time-ordered: {a} < {b}");
    }
}
