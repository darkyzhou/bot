use anyhow::{bail, Result};
use futures_util::StreamExt;
use lazy_static::lazy_static;
use log::{info, warn};
use regex::Regex;
use reqwest::Response;
use std::collections::HashMap;
use std::str::FromStr;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use url::Url;

pub static DEFAULT_HEADER: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/102.0.0.0 Safari/537.36";

pub fn extract_filename_from_url(url: &url::Url) -> Option<(&str, Option<&str>)> {
    url.path()
        .split('/')
        .rev()
        .find(|item| !item.is_empty())
        .map(
            |item| match item.splitn(2, '.').collect::<Vec<&str>>()[..] {
                [name, extension] => (name, Some(extension)),
                _ => (item, None),
            },
        )
}

pub async fn download_file_if_not_exists(response: Response, path: &str) -> Result<u64> {
    let size = response.content_length().unwrap_or(0);

    if let Ok(metadata) = tokio::fs::metadata(&path).await {
        let len = metadata.len();
        if len == size {
            info!(
                "file already exists at {}, size: {}, skip downloading",
                path, len
            );
            return Ok(len);
        }
        warn!(
            "file {} exists, but size unmatched, expected {} actual {}",
            path, size, len
        )
    }

    info!(
        "downloading file from {} to {}, size: {}",
        response.url().as_str(),
        path,
        size
    );
    let mut file = File::create(&path).await?;
    let mut stream = response.bytes_stream();
    while let Some(item) = stream.next().await {
        file.write_all(&item?).await?;
    }

    Ok(size)
}

pub fn serialize_hashmap(map: &HashMap<String, String>) -> String {
    let mut items: Vec<(&String, &String)> = map.iter().collect();
    items.sort();
    items.iter().fold("".to_string(), |acc, (key, val)| {
        format!("{}\n{}：{}", acc, key, val)
    })
}

pub fn extract_pixiv_artwork_id(url: &str) -> Option<String> {
    lazy_static! {
        static ref NUMBER_REGEX: Regex = Regex::new(r"^\d+$").unwrap();
    }

    if !url.contains("pixiv.net") {
        return None;
    }

    let url = url::Url::from_str(url).ok()?;

    if let Some((_, id)) = url
        .query_pairs()
        .into_iter()
        .find(|(id, _)| id == "illust_id")
    {
        return Some(id.to_string());
    }

    if let Some(id) = url.path_segments().and_then(|mut split| split.next_back()) {
        if NUMBER_REGEX.is_match(id) {
            return Some(id.to_string());
        }
    }

    None
}

pub fn get_file_name(url: &Url) -> anyhow::Result<String> {
    let result = url.path().split('/').into_iter().rev().next().map(|name| {
        if name.contains('.') {
            name.to_string()
        } else {
            format!("{}.mp4", name)
        }
    });
    match result {
        Some(name) => Ok(name),
        None => bail!("failed to infer a file name from {}", url.as_str()),
    }
}

#[cfg(test)]
mod tests {
    use crate::utils::{extract_filename_from_url, extract_pixiv_artwork_id};
    use std::str::FromStr;

    #[test]
    fn extract_filename_from_url_test_1() {
        assert_eq!(
            extract_filename_from_url(
                &url::Url::from_str("https://abs-0.twimg.com/emoji/v2/72x72/1f60d.png").unwrap()
            ),
            Some(("1f60d", Some("png")))
        )
    }

    #[test]
    fn extract_filename_from_url_test_2() {
        assert_eq!(
            extract_filename_from_url(
                &url::Url::from_str("https://abs-0.twimg.com/emoji/v2/72x72/1f60d").unwrap()
            ),
            Some(("1f60d", None))
        )
    }

    #[test]
    fn extract_filename_from_url_test_3() {
        assert_eq!(
            extract_filename_from_url(&url::Url::from_str("https://abs-0.twimg.com/").unwrap()),
            None
        )
    }

    #[test]
    fn extract_pixiv_artwork_id_test_1() {
        assert_eq!(
            extract_pixiv_artwork_id("https://www.pixiv.net/artworks/99118150?xx"),
            Some("99118150".to_string())
        );
    }

    #[test]
    fn extract_pixiv_artwork_id_test_2() {
        assert_eq!(
            extract_pixiv_artwork_id(
                "https://www.pixiv.net/member_illust.php?mode=medium&illust_id=99118150"
            ),
            Some("99118150".to_string())
        );
    }

    #[test]
    fn extract_pixiv_artwork_id_test_3() {
        assert_eq!(
            extract_pixiv_artwork_id(
                "https://www.pixiv.net/member_illust.php?mode=medium&illust_id=99118150&foo=bar"
            ),
            Some("99118150".to_string())
        );
    }

    #[test]
    fn extract_pixiv_artwork_id_test_4() {
        assert_eq!(
            extract_pixiv_artwork_id("https://www.pixiv.net/member_illust.php"),
            None
        );
    }

    #[test]
    fn extract_pixiv_artwork_id_test_5() {
        assert_eq!(
            extract_pixiv_artwork_id("https://www.pixiv.net/member_illust.php?mode=medium&foo=bar"),
            None
        );
    }
}
