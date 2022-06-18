use serde::Deserialize;
use async_trait::async_trait;
use std::collections::HashMap;
use std::time::Duration;

use crate::searcher::*;

pub struct SauceNaoImageSearcher {
    pub api_key: String,
}

#[derive(Deserialize, Debug)]
pub struct SauceNaoImageSearchResult {
    pub results: Vec<SauceNaoImageSearchResultItem>,
}

#[derive(Deserialize, Debug)]
pub struct SauceNaoImageSearchResultItem {
    pub header: SauceNaoImageSearchResultItemHeader,
    pub data: SauceNaoImageSearchResultItemData,
}

#[derive(Deserialize, Debug)]
pub struct SauceNaoImageSearchResultItemHeader {
    pub similarity: String,
}

#[derive(Deserialize, Debug)]
pub struct SauceNaoImageSearchResultItemData {
    pub ext_urls: Option<Vec<String>>,
    pub title: Option<String>,
    pub author_name: Option<String>,
}

#[async_trait]
impl ImageSearcher for SauceNaoImageSearcher {
    fn get_name(&self) -> &'static str {
        "saucenao"
    }

    async fn search(&self, url: &str) -> ImageSearchResult {
        let client = reqwest::Client::new();
        let result: SauceNaoImageSearchResult = client.get("https://saucenao.com/search.php")
            .query(&[("db", "999"), ("numres", "3"), ("api_key", self.api_key.as_ref()), ("output_type", "2"), ("url", url.as_ref())])
            .timeout(Duration::from_secs(15))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        match &result.results[..] {
            [result, ..] => {
                let similarity = result.header.similarity.parse::<f64>()?;

                if similarity < 0.95 || result.data.ext_urls.is_none() || result.data.ext_urls.as_ref().unwrap().is_empty() {
                    return Ok(None);
                }

                let mut metadata: HashMap<String, String> = HashMap::new();
                if let Some(author_name) = &result.data.author_name {
                    metadata.insert("作者".to_string(), author_name.clone());
                }
                if let Some(title) = &result.data.title {
                    metadata.insert("标题".to_string(), title.clone());
                }

                let url = result.data.ext_urls.as_ref().unwrap().get(0).unwrap().clone();

                return Ok(Some(SourceImage {
                    url,
                    searcher: self.get_name(),
                    metadata,
                }));
            }
            _ => Ok(None)
        }
    }
}
