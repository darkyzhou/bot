use anyhow::{bail, Context, Result};
use futures_util::StreamExt;
use lazy_static::lazy_static;
use log::{error, info};
use nanoid::nanoid;
use regex::Regex;
use serde_json::Value;
use std::time::Duration;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use url::Url;

use crate::cfg;
use crate::message::*;
use crate::utils;

lazy_static! {
    static ref CLIENT: reqwest::Client = {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::USER_AGENT,
            reqwest::header::HeaderValue::from_static(utils::DEFAULT_HEADER),
        );
        reqwest::Client::builder()
            .default_headers(headers)
            .timeout(Duration::from_secs(20))
            .build()
            .unwrap()
    };
}

pub async fn on_private_message(message: OneBotPrivateMessage) -> Option<SendMessage> {
    let OneBotPrivateMessage {
        user_id, message, ..
    } = message;
    let message = message.trim();

    if user_id != 2569766005 {
        return None;
    }

    let message_to_send: Option<String> = {
        if message.contains("twitter.com") {
            match download_twitter_video(message).await {
                Err(err) => {
                    error!("failed to download the twitter video: {:#?}", err);
                    Some(format!("视频保存失败 {:#?}", err))
                }
                Ok(size) => Some(format!(
                    "视频保存成功，大小: {}",
                    human_bytes::human_bytes(size as f64)
                )),
            }
        } else if let Some(url) = message.strip_prefix("v ") {
            match download_video(url).await {
                Err(err) => {
                    error!("failed to download the video: {:#?}", err);
                    Some("视频保存失败".to_string())
                }
                Ok(size) => Some(format!(
                    "视频保存成功，大小: {}",
                    human_bytes::human_bytes(size as f64)
                )),
            }
        } else {
            None
        }
    };

    message_to_send.map(|message| SendMessage::Private { user_id, message })
}

async fn download_twitter_video(url: &str) -> Result<u64> {
    async fn do_request(url: &str) -> Result<Value> {
        let result = CLIENT
            .post("https://twdown.net/download.php")
            .form(&[("URL", url.to_string())])
            .header(reqwest::header::REFERER, "https://twdown.net/")
            .header(reqwest::header::ORIGIN, "https://twdown.net/")
            .send()
            .await?
            .error_for_status()?
            .json::<Value>()
            .await?;
        Ok(result)
    }

    let response: Value = do_request(url).await.context("failed to request the api")?;
    let url = {
        let url = response.get("url").and_then(|value| value.as_str());
        match url {
            None => bail!("fail to extract url from the api response"),
            Some(url) => url,
        }
    };

    let size = download_video(url)
        .await
        .context("failed to download the video")?;
    Ok(size)
}

fn get_file_name(url: &Url) -> Result<String> {
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

async fn download_video(url: &str) -> Result<u64> {
    for times in 1..3 {
        match download(url).await {
            Ok(size) => {
                return Ok(size);
            }
            Err(err) => {
                if times >= 3 {
                    return Err(err.context("download exceeds maximum retry times"));
                }
            }
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
    unreachable!()
}

async fn download(url: &str) -> Result<u64> {
    let response = CLIENT.get(url).send().await?.error_for_status()?;
    let file_name = get_file_name(response.url()).unwrap_or(format!("{}.mp4", nanoid!()));
    let path = format!("{}/{}", cfg::BOT_CONFIG.twitter_videos_path, file_name);
    let size = response.content_length().unwrap_or(0);
    info!("downloading video from {} to {}, size: {}", url, path, size);

    let mut file = File::create(path.as_str()).await?;
    let mut stream = response.bytes_stream();
    while let Some(item) = stream.next().await {
        file.write_all(&item?).await?;
    }
    info!("download finished for url {}", url);
    Ok(size)
}
