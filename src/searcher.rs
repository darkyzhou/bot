use async_trait::async_trait;
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::error::Error;
use log::{error, info};
use regex::Regex;

use crate::database::*;
use crate::message::*;
use crate::utils;
use crate::ascii2d;
use crate::saucenao;
use crate::iqdb;

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

lazy_static! {
    static ref IMAGE_URL_REGEX: Regex = Regex::new(r"url=([^]]+)]").unwrap();
    static ref REPLY_ID_REGEX: Regex = Regex::new(r"id=([^]]+)]").unwrap();
}

pub async fn on_group_message(message: OneBotGroupMessage) -> Option<SendMessage> {
    let OneBotGroupMessage { ref message, message_id, group_id, .. } = message;
    if message.starts_with("[CQ:image") {
        if let Some(caps) = IMAGE_URL_REGEX.captures(message) {
            if caps.len() < 2 {
                return None;
            }
            if let Err(err) = DATABASE.insert(format!("image_url:{}", message_id).as_str(), &caps[1]) {
                error!("failed to insert record into database: {}", err);
                return None;
            }
        }
    } else if message.starts_with("[CQ:reply") {
        if !message.contains("查出处") && !message.contains("ccc") {
            return None;
        }

        if let Some(caps) = REPLY_ID_REGEX.captures(message) {
            if caps.len() < 2 {
                return None;
            }

            if let Ok(ref reply_id) = caps[1].parse::<i32>() {
                return match DATABASE.get(format!("image_url:{}", reply_id).as_str()) {
                    Ok(Some(image_url)) => {
                        let image_url = String::from_utf8(image_url.to_vec()).unwrap();
                        Some(SendMessage::Group {
                            group_id,
                            message: match search_image(image_url.as_str()).await {
                                Some(ref image) => format!("[CQ:reply,id={}]{}\n{}", message_id, image.url, utils::serialize_hashmap(&image.metadata)),
                                None => format!("[CQ:reply,id={}]并没有找到出处", message_id)
                            },
                        })
                    }
                    Ok(None) => None,
                    Err(err) => {
                        error!("failed to get record from database: {}", err);
                        None
                    }
                };
            }
        }
    }

    None
}

lazy_static! {
    static ref SEARCHERS: Box<[Box<dyn ImageSearcher + Send + Sync>]> = Box::new([
        Box::new(ascii2d::Ascii2dImageSearcher {}),
        Box::new(saucenao::SauceNaoImageSearcher {
            api_key: "978457bfdcbbfd9a205a9ee13d6621c6970023f7".to_string()
        }),
        Box::new(iqdb::IqdbImageSearcher {})
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


