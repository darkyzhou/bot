use crate::utils;
use lazy_static::lazy_static;
use std::time::Duration;

lazy_static! {
    pub static ref CLIENT: reqwest::Client = {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::USER_AGENT,
            reqwest::header::HeaderValue::from_static(utils::DEFAULT_HEADER),
        );
        reqwest::Client::builder()
            .default_headers(headers)
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap()
    };
}
