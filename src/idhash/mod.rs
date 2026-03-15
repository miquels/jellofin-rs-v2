use sha2::{Digest, Sha256};

// Top-level root ID, parent ID of all collections
pub const COLLECTION_ROOT_ID: &str = "e9d5075a555c1cbc394eec4cef295274";
// ID of dynamically generated Playlist collection
pub const PLAYLIST_COLLECTION_ID: &str = "2f0340563593c4d98b97c9bfa21ce23c";
// ID of dynamically generated favorites collection
pub const FAVORITES_COLLECTION_ID: &str = "f4a0b1c2d3e5c4b8a9e6f7d8e9a0b1c2";

pub const ITEM_PREFIX_MOVIE: &'static str = "mov_";
pub const ITEM_PREFIX_SHOW: &'static str = "sho_";
pub const ITEM_PREFIX_SEASON: &'static str = "sea_";
pub const ITEM_PREFIX_EPISODE: &'static str = "epi_";

pub const ITEM_PREFIX_GENRE: &'static str = "gen_";
pub const ITEM_PREFIX_STUDIO: &'static str = "stu_";
pub const ITEM_PREFIX_PERSON: &'static str = "per_";
pub const ITEM_PREFIX_COLLECTION: &'static str = "col_";
pub const ITEM_PREFIX_PLAYLIST: &'static str = "pla_";
pub const ITEM_PREFIX_DISPLAY_PREFERENCES: &'static str = "dsp_";

pub fn make_jf_display_preferences_id(id: &str) -> String {
    let b_id = id.as_bytes();
    if b_id.len() < 4 || b_id[3] != b'_' {
        format!("{}{}", ITEM_PREFIX_DISPLAY_PREFERENCES, id)
    } else {
        let id = str::from_utf8(&b_id[4..]).unwrap();
        format!("{}{}", ITEM_PREFIX_DISPLAY_PREFERENCES, id)
    }
}

pub fn is_jf_root_id(id: &str) -> bool {
    id == COLLECTION_ROOT_ID
}

pub fn is_jf_collection_favorites_id(id: &str) -> bool {
    id == FAVORITES_COLLECTION_ID
}

pub fn is_jf_collection_playlist_id(id: &str) -> bool {
    id == PLAYLIST_COLLECTION_ID
}

pub fn is_jf_collection_id(id: &str) -> bool {
    is_jf_root_id(id)
        || is_jf_collection_favorites_id(id)
        || is_jf_collection_playlist_id(id)
        || id.starts_with(ITEM_PREFIX_COLLECTION)
}

pub fn is_jf_playlist_id(id: &str) -> bool {
    id.starts_with(ITEM_PREFIX_PLAYLIST)
}

pub fn is_jf_genre_id(id: &str) -> bool {
    id.starts_with(ITEM_PREFIX_GENRE)
}

pub fn is_jf_studio_id(id: &str) -> bool {
    id.starts_with(ITEM_PREFIX_STUDIO)
}

#[allow(dead_code)]
pub fn is_jf_movie_id(id: &str) -> bool {
    id.starts_with(ITEM_PREFIX_MOVIE)
}

#[allow(dead_code)]
pub fn is_jf_show_id(id: &str) -> bool {
    id.starts_with(ITEM_PREFIX_SHOW)
}

#[allow(dead_code)]
pub fn is_jf_season_id(id: &str) -> bool {
    id.starts_with(ITEM_PREFIX_SEASON)
}

#[allow(dead_code)]
pub fn is_jf_episode_id(id: &str) -> bool {
    id.starts_with(ITEM_PREFIX_EPISODE)
}

#[allow(dead_code)]
pub fn is_jf_person_id(id: &str) -> bool {
    id.starts_with(ITEM_PREFIX_PERSON)
}

/// Hash bytes with SHA256 and return hex string (for ETags).
pub fn hash_bytes(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

/// Hash a string with sha256.
/// Then take the first 120 bits, and convert that to base32.
/// Returns a 24-character long string.
pub fn id_hash(name: &str) -> String {
    id_hash_prefix("", name)
}

/// Hash a string with sha256.
/// Then take the first 120 bits, and convert that to base32.
/// Returns a prefix + 24-character long string.
pub fn id_hash_prefix(prefix: &str, name: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(name);
    let hash256 = hasher.finalize();

    // Create 128 bit integer from the first 16 bytes of the hash.
    let num128 = u128::from_be_bytes([
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

    base32(prefix, num128)
}

/// Generate a new random id, prefix it with 'prefix'.
pub fn id_new_prefix(prefix: &str) -> String {
    let num128: u128 = rand::random();
    base32(prefix, num128)
}

fn base32(prefix: &str, mut num128: u128) -> String {
    // Use only the first 120 bits.
    num128 >>= 8;

    // Convert to base32.
    let mut id = String::with_capacity(prefix.len() + 20);
    id.push_str(prefix);

    for _ in 0..20 {
        let m = (num128 % 32) as u8;
        num128 /= 32;

        let c = if m < 10 {
            m + 48 // '0' to '9'
        } else {
            m + 97 - 10 // 'a' to 'z'
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
