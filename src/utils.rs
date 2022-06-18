use std::collections::HashMap;
use std::str::FromStr;
use lazy_static::lazy_static;
use regex::Regex;

pub fn serialize_hashmap(map: &HashMap<String, String>) -> String {
    let mut items: Vec<(&String, &String)> = map.iter().collect();
    items.sort();
    items.iter().fold("".to_string(), |acc, (key, val)| format!("{}\n{}：{}", acc, key, val))
}

pub fn extract_pixiv_artwork_id(url: &str) -> Option<String> {
    lazy_static! {
        static ref NUMBER_REGEX: Regex = Regex::new(r"^\d+$").unwrap();
    }

    if !url.contains("pixiv.net") {
        return None;
    }

    let url = url::Url::from_str(url).ok()?;

    if let Some((_, id)) = url.query_pairs().into_iter().find(|(id, _)| id == "illust_id") {
        return Some(id.to_string());
    }

    if let Some(id) = url.path_segments().and_then(|mut split| split.next_back()) {
        if NUMBER_REGEX.is_match(id) {
            return Some(id.to_string());
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use crate::utils::extract_pixiv_artwork_id;

    #[test]
    fn extract_pixiv_artwork_id_test_1() {
        assert_eq!(extract_pixiv_artwork_id("https://www.pixiv.net/artworks/99118150?xx"), Some("99118150".to_string()));
    }

    #[test]
    fn extract_pixiv_artwork_id_test_2() {
        assert_eq!(extract_pixiv_artwork_id("https://www.pixiv.net/member_illust.php?mode=medium&illust_id=99118150"), Some("99118150".to_string()));
    }

    #[test]
    fn extract_pixiv_artwork_id_test_3() {
        assert_eq!(extract_pixiv_artwork_id("https://www.pixiv.net/member_illust.php?mode=medium&illust_id=99118150&foo=bar"), Some("99118150".to_string()));
    }

    #[test]
    fn extract_pixiv_artwork_id_test_4() {
        assert_eq!(extract_pixiv_artwork_id("https://www.pixiv.net/member_illust.php"), None);
    }

    #[test]
    fn extract_pixiv_artwork_id_test_5() {
        assert_eq!(extract_pixiv_artwork_id("https://www.pixiv.net/member_illust.php?mode=medium&foo=bar"), None);
    }
}
