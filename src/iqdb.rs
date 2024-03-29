use anyhow::anyhow;
use async_trait::async_trait;
use std::collections::HashMap;
use visdom::Vis;

use crate::client::*;
use crate::searcher::*;

pub struct IqdbImageSearcher {}

#[async_trait]
impl ImageSearcher for IqdbImageSearcher {
    fn get_name(&self) -> &'static str {
        "iqdb"
    }

    async fn search(&self, url: &str) -> ImageSearchResult {
        let response = CLIENT
            .get(format!("https://iqdb.org/?url={}", url))
            .send()
            .await?
            .error_for_status()?;
        let html = response.text().await?;
        self.parse_result(html.as_str())
    }
}

impl IqdbImageSearcher {
    fn parse_result(&self, html: &str) -> ImageSearchResult {
        let root = Vis::load(html).map_err(|e| anyhow!(e))?;
        let target = root.find("#pages > div + div");
        let mut source_url = target
            .find("tr:nth-of-type(2) a")
            .attr("href")
            .ok_or_else(|| anyhow!("failed to find href of the result"))?
            .to_string();
        if source_url.starts_with("//") {
            source_url = format!("https:{}", source_url);
        }
        Ok(Some(SourceImage {
            url: source_url,
            searcher: self.get_name(),
            metadata: HashMap::default(),
        }))
    }
}
