use async_trait::async_trait;
use std::collections::HashMap;
use std::error::Error;

#[derive(Debug, PartialEq)]
pub struct SourceImage {
    pub url: String,
    pub metadata: HashMap<String, String>,
}

pub type ImageSearchResult = Result<Option<SourceImage>, Box<dyn Error>>;

#[async_trait]
pub trait ImageSearcher {
    fn get_name(&self) -> &'static str;
    async fn search(&self, url: &str) -> ImageSearchResult;
}
