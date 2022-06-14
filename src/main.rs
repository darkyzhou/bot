mod saucenao;
mod searcher;
mod ascii2d;
mod iqdb;

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use regex::Regex;
use std::sync::Arc;
use tokio::sync::Mutex;
use config::Config;
use lazy_static::lazy_static;
use log::{debug, error, info, warn};
use tokio::signal::unix::{signal, SignalKind};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;

use crate::ascii2d::*;
use crate::searcher::*;
use crate::saucenao::*;
use crate::iqdb::*;

lazy_static! {
    static ref BOT_CONFIG: BotConfig = Config::builder()
        .add_source(config::File::with_name("config"))
        .add_source(config::Environment::with_prefix("APP"))
        .build()
        .expect("failed to load config")
        .try_deserialize()
        .expect("failed to parse config");

    static ref IMAGE_URL_REGEX: Regex = Regex::new(r"url=([^]]+)]").unwrap();
    static ref REPLY_ID_REGEX: Regex = Regex::new(r"id=([^]]+)]").unwrap();

    static ref SEARCHERS: Box<[Box<dyn ImageSearcher + Send + Sync>]> = Box::new([
        Box::new(Ascii2dImageSearcher {}),
        Box::new(SauceNaoImageSearcher {
            api_key: "978457bfdcbbfd9a205a9ee13d6621c6970023f7".to_string()
        }),
        Box::new(IqdbImageSearcher {})
    ]);
}

async fn search_image(url: &str) -> Option<SourceImage> {
    for searcher in SEARCHERS.iter() {
        info!("trying searcher {} for image {}", searcher.get_name(), url);
        match searcher.search(url).await {
            Ok(result) => {
                match result {
                    None => {
                        info!("no results found");
                        continue;
                    }
                    Some(mut image) => {
                        info!("found result {}", &image.url);
                        image.metadata.insert("服务".to_string(), searcher.get_name().to_string());
                        return Some(image);
                    }
                }
            }
            Err(err) => {
                error!("failed to search the image: {:#?}", err);
                continue;
            }
        };
    }
    None
}

#[derive(Debug, Deserialize)]
struct BotConfig {
    ws_url: String,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "post_type")]
enum OneBotMessage {
    #[serde(rename = "message")]
    UserMessage {
        message_type: String,
        message: String,
        message_id: i32,
        group_id: i32,
    },
    #[serde(rename = "meta_event")]
    MetaEvent {
        meta_event_type: String
    },
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum OneBotMessageWrapper {
    OneBotMessage(OneBotMessage),
    Other(serde_json::Value),
}

#[derive(Debug, Serialize)]
struct SendGroupMessage {
    group_id: i32,
    message: String,
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let (streams, _) = tokio_tungstenite::connect_async(&BOT_CONFIG.ws_url).await.expect("failed to connect to websocket server");
    let (mut write, read) = streams.split();
    let (tx, mut rx) = mpsc::channel::<SendGroupMessage>(128);

    tokio::spawn(async move {
        while let Some(ref message) = rx.recv().await {
            let params = match serde_json::to_string(message) {
                Ok(params) => params,
                Err(err) => {
                    error!("failed to parse SendGroupMessage: {}", err);
                    continue;
                }
            };
            let data = format!(r#"{{
                "action": "send_group_msg",
                "params": {},
            }}"#, params);
            if let Err(err) = write.send(tungstenite::Message::Text(data)).await {
                error!("failed to send message to websocket server: {:#?}", err);
            }
        }
    });

    tokio::spawn(async move {
        let image_id_cache: Arc<Mutex<HashMap<i32, String>>> = Arc::new(Mutex::new(HashMap::new()));
        let tx = Arc::new(Mutex::new(tx));

        read.for_each(|message| async {
            let image_id_cache = image_id_cache.clone();
            let tx = tx.clone();

            let data = message.expect("failed to read message").into_data();
            let data = String::from_utf8(data).expect("malformed message, not a utf-8 string");
            let data = data.replacen("\"message_type\":\"group\",", "", 1);

            debug!("{}", data.as_str());
            let message: OneBotMessageWrapper = serde_json::from_str(data.as_str()).expect("malformed json");

            if let OneBotMessageWrapper::OneBotMessage(OneBotMessage::UserMessage { ref message, message_id, group_id, .. }) = message {
                if message.starts_with("[CQ:image") {
                    if let Some(caps) = IMAGE_URL_REGEX.captures(message) {
                        if caps.len() < 2 {
                            return;
                        }
                        image_id_cache.lock().await.insert(message_id, caps[1].to_string());
                    }
                } else if message.starts_with("[CQ:reply") {
                    if !message.contains("查出处") {
                        return;
                    }

                    if let Some(caps) = REPLY_ID_REGEX.captures(message) {
                        if caps.len() < 2 {
                            return;
                        }

                        if let Ok(ref reply_id) = caps[1].parse::<i32>() {
                            if let Some(image_url) = image_id_cache.lock().await.get(reply_id).cloned() {
                                let message_to_send = match search_image(image_url.as_str()).await {
                                    Some(ref image) => {
                                        SendGroupMessage {
                                            group_id,
                                            message: format!("[CQ:reply,id={}]{}\n{}", message_id, image.url, image.metadata.iter().fold("".to_string(), |acc, (key, val)| format!("{}\n{}：{}", acc, key, val))),
                                        }
                                    }
                                    None => {
                                        SendGroupMessage {
                                            group_id,
                                            message: format!("[CQ:reply,id={}]并没有找到出处", message_id),
                                        }
                                    }
                                };
                                if let Err(err) = tx.lock().await.send(message_to_send).await {
                                    error!("failed to send group message over channel: {}", err);
                                }
                            }
                        }
                    }
                }
            }
        }).await;
    });

    let (mut int_signal, mut term_signal) = (signal(SignalKind::interrupt()).unwrap(), signal(SignalKind::terminate()).unwrap());
    tokio::select! {
        _ = int_signal.recv() => {},
        _ = term_signal.recv() => {},
    }
    warn!("signal received, shutting down");
}