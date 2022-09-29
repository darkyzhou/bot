use crate::database::*;
use crate::iqdb;
use crate::message::*;
use crate::saucenao;
use crate::{ascii2d, utils};
use anyhow::Result;
use async_trait::async_trait;
use lazy_static::lazy_static;
use log::error;
use regex::Regex;
use std::collections::HashMap;

#[derive(Debug, PartialEq)]
pub struct SourceImage {
    pub url: String,
    pub searcher: &'static str,
    pub metadata: HashMap<String, String>,
}

pub type ImageSearchResult = Result<Option<SourceImage>>;

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
    let OneBotGroupMessage {
        ref message,
        message_id,
        group_id,
        ..
    } = message;

    if message.contains("[CQ:image") {
        if let Some(caps) = IMAGE_URL_REGEX.captures(message) {
            if caps.len() < 2 {
                return None;
            }
            if let Err(err) =
                DATABASE.insert(format!("image_url:{}", message_id).as_str(), &caps[1])
            {
                error!("failed to insert record into database: {}", err);
                return None;
            }
        }
    }

    if message.contains("[CQ:reply") && (message.contains("查出处") || message.contains("ccc")) {
        if let Some(caps) = REPLY_ID_REGEX.captures(message) {
            if caps.len() < 2 {
                return None;
            }

            if let Ok(ref reply_id) = caps[1].parse::<i32>() {
                return match DATABASE.get(format!("image_url:{}", reply_id).as_str()) {
                    Ok(None) => None,
                    Err(err) => {
                        error!("failed to get record from database: {}", err);
                        None
                    }
                    Ok(Some(image_url)) => {
                        let image_url = String::from_utf8(image_url.to_vec()).unwrap();
                        let message = match search_image(image_url.as_str()).await.as_slice() {
                            images @ [_, ..] => {
                                format!("[CQ:reply,id={}]{}", message_id, parse_result(images))
                            }
                            _ => format!("[CQ:reply,id={}]并没有找到出处", message_id),
                        };
                        Some(SendMessage::Group { group_id, message })
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

async fn search_image(url: &str) -> Vec<SourceImage> {
    let mut tasks = vec![];
    for searcher in SEARCHERS.iter() {
        tasks.push(searcher.search(url));
    }

    let results = futures::future::join_all(tasks.into_iter()).await;
    results
        .into_iter()
        .enumerate()
        .filter_map(|(i, result)| match result {
            Ok(Some(image)) => Some(image),
            Ok(None) => {
                error!(
                    "source image not found for {} using {}",
                    url,
                    SEARCHERS[i].get_name()
                );
                None
            }
            Err(err) => {
                error!(
                    "failed to search image {} using {}: {:#?}",
                    url,
                    SEARCHERS[i].get_name(),
                    err
                );
                None
            }
        })
        .collect()
}

fn parse_result(images: &[SourceImage]) -> String {
    images
        .iter()
        .fold(
            "🥵🥵🥵 色图出处 👇👇👇\n\n".to_string(),
            |mut result, image| {
                let url = {
                    if let Some(pixiv_id) = utils::extract_pixiv_artwork_id(image.url.as_str()) {
                        format!(
                            "{}\n国内加速: https://pixiv.re/{}.png",
                            image.url.as_str(),
                            pixiv_id
                        )
                    } else {
                        image.url.clone()
                    }
                };
                result.push_str(
                    format!(
                        "⚠️ {}\n{}\n{}\n\n",
                        image.searcher,
                        utils::serialize_hashmap(&image.metadata),
                        url
                    )
                    .as_str(),
                );
                result
            },
        )
        .trim_end()
        .to_string()
}
