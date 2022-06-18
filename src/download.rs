use std::time::Duration;
use anyhow::{bail, Context, Result};
use lazy_static::lazy_static;
use regex::Regex;
use serde_json::Value;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use futures_util::StreamExt;
use log::{error, info};
use nanoid::nanoid;
use url::Url;

use crate::cfg;
use crate::message::*;

lazy_static! {
    static ref VIDEO_ID_REGEX: Regex = Regex::new(r"(\d+)(?:$|\?)").unwrap();
    static ref CLIENT: reqwest::Client = {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(reqwest::header::USER_AGENT, reqwest::header::HeaderValue::from_static("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/102.0.0.0 Safari/537.36"));
        reqwest::Client::builder()
            .default_headers(headers)
            .timeout(Duration::from_secs(20))
            .build()
            .unwrap()
    };
}

pub async fn on_private_message(message: OneBotPrivateMessage) -> Option<SendMessage> {
    let OneBotPrivateMessage { user_id, message, .. } = message;
    let message = message.trim();

    if user_id != 2569766005 {
        return None;
    }

    let message_to_send: Option<String> = {
        if message.contains("twitter.com") {
            let video_id = match VIDEO_ID_REGEX.captures(message) {
                None => None,
                Some(capture) => {
                    if capture.len() < 2 {
                        return None;
                    }
                    capture[1].parse::<i64>().ok()
                }
            };

            match video_id {
                None => Some("输入的 url 有误".to_string()),
                Some(video_id) => match download_twitter_video(video_id).await {
                    Err(err) => {
                        error!("failed to download twitter video: {:#?}", err);
                        Some("视频保存失败".to_string())
                    }
                    Ok(size) => Some(format!("视频保存成功，大小: {}", human_bytes::human_bytes(size as f64)))
                }
            }
        } else if message.starts_with("v ") {
            match download_video(&message[2..]).await {
                Err(err) => {
                    error!("failed to download twitter video: {:#?}", err);
                    Some("视频保存失败".to_string())
                }
                Ok(size) => Some(format!("视频保存成功，大小: {}", human_bytes::human_bytes(size as f64)))
            }
        } else {
            None
        }
    };

    message_to_send.map(|message| SendMessage::Private {
        user_id,
        message,
    })
}

async fn download_twitter_video(video_id: i64) -> Result<u64> {
    async fn do_request(video_id: i64) -> Result<Value> {
        let result = CLIENT.get(format!("https://6xmdq42sp7.execute-api.us-east-1.amazonaws.com/prod/videos/{}", video_id))
            .header(reqwest::header::REFERER, "https://twittervideo.org/")
            .header(reqwest::header::ORIGIN, "https://twittervideo.org/")
            .send()
            .await?
            .error_for_status()?
            .json::<Value>()
            .await?;
        Ok(result)
    }

    let response: Value = do_request(video_id).await.context("failed to request twittervideo.org api")?;
    let url = {
        let url = response.get("url").and_then(|value| value.as_str());
        match url {
            None => bail!("fail to extract url from api response of twittervideo.org"),
            Some(url) => url
        }
    };

    let size = download_video(url).await.context("failed to download video")?;
    Ok(size)
}

fn get_file_name(url: &Url) -> Result<String> {
    let result = url.path()
        .split('/')
        .into_iter()
        .rev()
        .next()
        .map(|name| {
            if name.contains('.') {
                name.to_string()
            } else {
                format!("{}.mp4", name)
            }
        });
    match result {
        Some(name) => Ok(name),
        None => bail!("failed to infer file name from {}", url.as_str())
    }
}

async fn download_video(url: &str) -> Result<u64> {
    for times in 1..3 {
        match download(url).await {
            Ok(size) => { return Ok(size); }
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
    let response = CLIENT.get(url)
        .header(reqwest::header::USER_AGENT, "curl/7.79.1")
        .send()
        .await?
        .error_for_status()?;
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
