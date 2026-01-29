use sha2::{Digest, Sha256};

/// Hash a string with sha256.
/// Then take the first 119 bits, and convert that to base62.
/// Returns a 20-character long string.
pub fn id_hash(name: &str) -> String {
    // Create hash from string.
    let mut hasher = Sha256::new();
    hasher.update(name.as_bytes());
    let hash256 = hasher.finalize();

    // Create 128 bit integer from the first 16 bytes of the hash.
    let mut num128 = u128::from_be_bytes([
        hash256[0],
        hash256[1],
        hash256[2],
        hash256[3],
        hash256[4],
        hash256[5],
        hash256[6],
        hash256[7],
        hash256[8],
        hash256[9],
        hash256[10],
        hash256[11],
        hash256[12],
        hash256[13],
        hash256[14],
        hash256[15],
    ]);

    // Use only the first 119 bits.
    num128 >>= 9;

    // Convert to base62.
    let mut id = String::with_capacity(20);
    for _ in 0..20 {
        let m = (num128 % 62) as u8;
        num128 /= 62;

        let c = if m < 10 {
            m + 48 // '0' to '9'
        } else if m < 36 {
            m + 65 - 10 // 'A' to 'Z'
        } else {
            m + 97 - 36 // 'a' to 'z'
        };
        id.push(c as char);
    }

    id
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_id_hash_deterministic() {
        let input = "test string";
        let hash1 = id_hash(input);
        let hash2 = id_hash(input);
        assert_eq!(hash1, hash2, "Hash should be deterministic");
    }

    #[test]
    fn test_id_hash_length() {
        let hash = id_hash("any string");
        assert_eq!(hash.len(), 20, "Hash should be 20 characters long");
    }

    #[test]
    fn test_id_hash_different_inputs() {
        let hash1 = id_hash("input1");
        let hash2 = id_hash("input2");
        assert_ne!(hash1, hash2, "Different inputs should produce different hashes");
    }

    #[test]
    fn test_id_hash_base62_chars() {
        let hash = id_hash("test");
        for c in hash.chars() {
            assert!(
                c.is_ascii_alphanumeric(),
                "Hash should only contain alphanumeric characters"
            );
        }
    }
}
