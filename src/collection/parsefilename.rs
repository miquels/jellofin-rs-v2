use regex::Regex;
use std::sync::OnceLock;

/// Parse episode information from filename
/// Returns (season_no, episode_no, double_episode, name) if successful
pub fn parse_episode_name(filename: &str, season_hint: i32) -> Option<(i32, i32, bool, String)> {
    // Pattern: ___.s03e04.___
    static PAT1: OnceLock<Regex> = OnceLock::new();
    let pat1 = PAT1.get_or_init(|| Regex::new(r"^.*[ ._][sS]([0-9]+)[eE]([0-9]+)[ ._].*$").unwrap());

    // Pattern: ___.s03e04e05.___ or ___.s03e04-e05.___
    static PAT2: OnceLock<Regex> = OnceLock::new();
    let pat2 = PAT2.get_or_init(|| Regex::new(r"^.*[ ._][sS]([0-9]+)[eE]([0-9]+)-?[eE]([0-9]+)[ ._].*$").unwrap());

    // Pattern: ___.2015.03.08.___
    static PAT3: OnceLock<Regex> = OnceLock::new();
    let pat3 = PAT3.get_or_init(|| Regex::new(r"^.*[ .]([0-9]{4})[.-]([0-9]{2})[.-]([0-9]{2})[ .].*$").unwrap());

    // Pattern: ___.308.___  (or 3x08) where first number is season.
    static PAT4: OnceLock<Regex> = OnceLock::new();
    let pat4 = PAT4.get_or_init(|| Regex::new(r"^.*[ .]([0-9]{1,2})x?([0-9]{2})[ .].*$").unwrap());

    // Try pattern 1: s01e04
    if let Some(caps) = pat1.captures(filename) {
        let season = caps.get(1)?.as_str().parse().ok()?;
        let episode = caps.get(2)?.as_str().parse().ok()?;
        let name = format!("{}x{}", caps.get(1)?.as_str(), caps.get(2)?.as_str());
        return Some((season, episode, false, name));
    }

    // Try pattern 2: s01e04e05 (double episode)
    if let Some(caps) = pat2.captures(filename) {
        let season = caps.get(1)?.as_str().parse().ok()?;
        let episode = caps.get(2)?.as_str().parse().ok()?;
        let name = format!(
            "{}x{}-{}",
            caps.get(1)?.as_str(),
            caps.get(2)?.as_str(),
            caps.get(3)?.as_str()
        );
        return Some((season, episode, true, name));
    }

    // Try pattern 3: 2015.03.08 (date-based)
    if let Some(caps) = pat3.captures(filename) {
        let year = caps.get(1)?.as_str();
        let month = caps.get(2)?.as_str();
        let day = caps.get(3)?.as_str();
        let episode_str = format!("{}{}{}", year, month, day);
        let episode = episode_str.parse().ok()?;
        let name = format!("{}.{}.{}", year, month, day);
        return Some((season_hint, episode, false, name));
    }

    // Try pattern 4: 308 or 3x08
    if let Some(caps) = pat4.captures(filename) {
        let season: i32 = caps.get(1)?.as_str().parse().ok()?;
        let episode: i32 = caps.get(2)?.as_str().parse().ok()?;
        
        if season_hint < 0 || season_hint == season {
            let name = format!("{:02}x{}", season, caps.get(2)?.as_str());
            return Some((season, episode, false, name));
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_s01e04() {
        let result = parse_episode_name("show.s01e04.mkv", -1);
        assert_eq!(result, Some((1, 4, false, "01x04".to_string())));
    }

    #[test]
    fn test_parse_s01e04_uppercase() {
        let result = parse_episode_name("show.S02E10.mkv", -1);
        assert_eq!(result, Some((2, 10, false, "02x10".to_string())));
    }

    #[test]
    fn test_parse_double_episode() {
        let result = parse_episode_name("show.s01e04e05.mkv", -1);
        assert_eq!(result, Some((1, 4, true, "01x04-05".to_string())));
    }

    #[test]
    fn test_parse_date_format() {
        let result = parse_episode_name("show.2015.03.08.mkv", 1);
        assert_eq!(result, Some((1, 20150308, false, "2015.03.08".to_string())));
    }

    #[test]
    fn test_parse_3x08_format() {
        let result = parse_episode_name("show.3x08.mkv", -1);
        assert_eq!(result, Some((3, 8, false, "03x08".to_string())));
    }

    #[test]
    fn test_parse_308_format() {
        let result = parse_episode_name("show.308.mkv", 3);
        assert_eq!(result, Some((3, 8, false, "03x08".to_string())));
    }

    #[test]
    fn test_parse_invalid() {
        let result = parse_episode_name("random_file.mkv", -1);
        assert_eq!(result, None);
    }
}
