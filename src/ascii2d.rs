use crate::client::CLIENT;
use crate::searcher::*;
use anyhow::anyhow;
use async_trait::async_trait;
use std::collections::HashMap;
use visdom::Vis;

pub struct Ascii2dImageSearcher {}

#[async_trait]
impl ImageSearcher for Ascii2dImageSearcher {
    fn get_name(&self) -> &'static str {
        "ascii2d"
    }

    async fn search(&self, url: &str) -> ImageSearchResult {
        let response = CLIENT
            .get(format!("https://ascii2d.net/search/url/{}", url))
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
        let source_url = item_box
            .find(".detail-box a:nth-of-type(1)")
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
