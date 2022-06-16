use std::time::Duration;
use lazy_static::lazy_static;
use regex::Regex;
use serde_json::Value;
use snafu::Snafu;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use futures_util::StreamExt;
use log::{error, info};

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
    if user_id != 2569766005 || !message.contains("twitter.com") {
        return None;
    }

    let video_id = match VIDEO_ID_REGEX.captures(message.as_str()) {
        None => None,
        Some(capture) => {
            if capture.len() < 2 {
                return None;
            }
            capture[1].parse::<i64>().ok()
        }
    };

    let message_to_send = match video_id {
        None => "输入的 url 有误",
        Some(video_id) => match save_video(video_id).await {
            Err(err) => {
                error!("failed to download twitter video: {:#?}", err);
                "视频保存失败"
            }
            Ok(_) => "视频保存成功"
        }
    };

    Some(SendMessage::Private {
        user_id,
        message: message_to_send.to_string(),
    })
}

#[derive(Debug, Snafu)]
enum SaveVideoError {
    #[snafu(display("failed to request the api"))]
    Request { video_id: i64, source: Box<dyn std::error::Error> },

    #[snafu(display("failed to get url from api response"))]
    Response { video_id: i64, response: String },

    #[snafu(display("failed to download url"))]
    Download { video_id: i64, source: Box<dyn std::error::Error> },
}

async fn save_video(video_id: i64) -> Result<(), SaveVideoError> {
    async fn do_request(video_id: i64) -> Result<Value, Box<dyn std::error::Error>> {
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

    async fn download(video_id: i64, url: &str) -> Result<(), Box<dyn std::error::Error>> {
        let path = format!("{}/{}.mp4", cfg::BOT_CONFIG.twitter_videos_path, video_id);
        info!("downloading video from {} to {}", url, path);

        let mut file = File::create(path.as_str()).await?;
        let mut stream = CLIENT.get(url).send().await?.error_for_status()?.bytes_stream();
        while let Some(item) = stream.next().await {
            file.write_all(&item?).await?;
        }
        Ok(())
    }

    let response: Value = do_request(video_id).await.map_err(|e| SaveVideoError::Request { video_id, source: e })?;
    let url = {
        let url = response.get("url").and_then(|value| value.as_str());
        match url {
            None => { return Err(SaveVideoError::Response { video_id, response: response.to_string() }); }
            Some(url) => url
        }
    };

    download(video_id, url).await.map_err(|e| SaveVideoError::Download { video_id, source: e })?;

    Ok(())
}
