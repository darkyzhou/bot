use std::collections::HashMap;
use std::time::Duration;
use async_trait::async_trait;
use visdom::Vis;
use snafu::prelude::*;

use crate::searcher::*;

#[derive(Debug, Snafu)]
pub enum IqdbError {
    #[snafu(display("Failed to parse response html from iqdb"))]
    ParseHtml {
        url: String,
        source: visdom::types::BoxDynError,
    },
    #[snafu(display("Failed to find source url from parsed html"))]
    FindSourceUrl {
        url: String,
        html: String,
    },
}

pub struct IqdbImageSearcher {}

#[async_trait]
impl ImageSearcher for IqdbImageSearcher {
    fn get_name(&self) -> &'static str {
        "iqdb"
    }

    async fn search(&self, url: &str) -> ImageSearchResult {
        let client = reqwest::Client::new();
        let response = client.get(format!("https://iqdb.org/?url={}", url))
            .header(reqwest::header::USER_AGENT, "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/102.0.0.0 Safari/537.36")
            .timeout(Duration::from_secs(20))
            .send()
            .await?
            .error_for_status()?;
        let html = response.text().await?;
        parse_result(url, html.as_str())
    }
}

fn parse_result(url: &str, html: &str) -> ImageSearchResult {
    let root = Vis::load(html).map_err(|e| IqdbError::ParseHtml {
        url: url.to_string(),
        source: e,
    })?;
    let target = root.find("#pages > div + div");
    let mut source_url = target.find("tr:nth-of-type(2) a").attr("href").ok_or(IqdbError::FindSourceUrl {
        url: url.to_string(),
        html: html.to_string(),
    })?.to_string();
    if source_url.starts_with("//") {
        source_url = format!("https:{}", source_url);
    }
    Ok(Some(SourceImage {
        url: source_url,
        metadata: HashMap::default(),
    }))
}

