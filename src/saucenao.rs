use crate::client::CLIENT;
use crate::searcher::*;
use async_trait::async_trait;
use serde::Deserialize;
use std::collections::HashMap;

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
        let result: SauceNaoImageSearchResult = CLIENT
            .get("https://saucenao.com/search.php")
            .query(&[
                ("db", "999"),
                ("numres", "3"),
                ("api_key", self.api_key.as_ref()),
                ("output_type", "2"),
                ("url", url.as_ref()),
            ])
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        match &result.results[..] {
            [result, ..] => {
                let similarity = result.header.similarity.parse::<f64>()?;

                if similarity < 0.95
                    || result.data.ext_urls.is_none()
                    || result.data.ext_urls.as_ref().unwrap().is_empty()
                {
                    return Ok(None);
                }

                let mut metadata: HashMap<String, String> = HashMap::new();
                if let Some(author_name) = &result.data.author_name {
                    metadata.insert("作者".to_string(), author_name.clone());
                }
                if let Some(title) = &result.data.title {
                    metadata.insert("标题".to_string(), title.clone());
                }

                let url = result
                    .data
                    .ext_urls
                    .as_ref()
                    .unwrap()
                    .get(0)
                    .unwrap()
                    .clone();

                return Ok(Some(SourceImage {
                    url,
                    searcher: self.get_name(),
                    metadata,
                }));
            }
            _ => Ok(None),
        }
    }
}
