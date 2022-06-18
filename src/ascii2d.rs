use std::collections::HashMap;
use std::time::Duration;
use anyhow::anyhow;
use async_trait::async_trait;
use visdom::Vis;

use crate::searcher::*;

pub struct Ascii2dImageSearcher {}

#[async_trait]
impl ImageSearcher for Ascii2dImageSearcher {
    fn get_name(&self) -> &'static str {
        "ascii2d"
    }

    async fn search(&self, url: &str) -> ImageSearchResult {
        let client = reqwest::Client::new();
        let response = client.get(format!("https://ascii2d.net/search/url/{}", url))
            .header(reqwest::header::USER_AGENT, "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/102.0.0.0 Safari/537.36")
            .timeout(Duration::from_secs(15))
            .send()
            .await?
            .error_for_status()?;
        let html = response.text().await?;
        self.parse_result(html.as_str())
    }
}

impl Ascii2dImageSearcher {
    fn parse_result(&self, html: &str) -> ImageSearchResult {
        let root = Vis::load(html).map_err(|e| anyhow!(e))?;
        let item_box = root.find(".item-box ~ .item-box");
        let source_url = item_box.find(".detail-box a:nth-of-type(1)")
            .attr("href")
            .ok_or_else(|| anyhow!("failed to find href for ascii2d result"))?
            .to_string();
        let metadata = {
            let name = item_box.find(".detail-box a:nth-of-type(2)").html();
            if name.is_empty() {
                HashMap::default()
            } else {
                HashMap::from([("作者".to_string(), name)])
            }
        };

        Ok(Some(SourceImage {
            url: source_url,
            searcher: self.get_name(),
            metadata,
        }))
    }
}
