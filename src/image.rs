use crate::client::CLIENT;
use crate::message::*;
use crate::{cfg, utils};
use anyhow::{anyhow, bail, Context, Result};
use lazy_static::lazy_static;
use nanoid::nanoid;
use regex::Regex;
use std::str::FromStr;
use visdom::Vis;

pub async fn on_group_message(message: OneBotGroupMessage) -> Option<BotResponseAction> {
    let OneBotGroupMessage {
        message,
        user_id,
        group_id,
        ..
    } = message;
    if user_id != cfg::BOT_CONFIG.admin_user_id {
        return None;
    }

    if !message.contains("mobile.twitter.com") {
        return None;
    }

    let messaeg_to_send = {
        let url = message
            .strip_prefix("i")
            .or_else(|| message.strip_prefix("ig"))?;
        let process = message.starts_with("i");
        match handle_request(url, process).await {
            Err(err) => format!("{:#?}", err),
            Ok(path) => format!("[CQ:image,file=file://{}]", path),
        }
    };

    Some(BotResponseAction::GroupMessage {
        group_id,
        message: messaeg_to_send,
    })
}

async fn handle_request(url: &str, process: bool) -> Result<String> {
    let image_url = {
        if url.contains("twitter.com") {
            get_twitter_image_url(url)
                .await
                .context("Failed to get image url")?
                .ok_or_else(|| anyhow!("No image urls found"))?
        } else {
            url.to_string()
        }
    };
    let image_url = url::Url::from_str(&image_url).context("Failed to parse url")?;

    let path = download_image(&image_url)
        .await
        .context("Failed to download the image")?;
    if process {
        let path = process_image(path)
            .await
            .context("Failed to process the image")?;
        return Ok(path);
    }
    return Ok(path);
}

async fn get_twitter_image_url(url: &str) -> Result<Option<String>> {
    lazy_static! {
        static ref TWITTER_URL_REGEX: Regex = Regex::new(r"/([^/]+)/status/(\d+)").unwrap();
    }

    match TWITTER_URL_REGEX
        .captures(url)
        .map(|captures| (captures.get(1), captures.get(2)))
    {
        Some((Some(user), Some(id))) => {
            let html = CLIENT
                .get(format!(
                    "https://nitter.it/{}/status/{}",
                    user.as_str(),
                    id.as_str()
                ))
                .send()
                .await?
                .error_for_status()?
                .text()
                .await?;
            let root = Vis::load(html).map_err(|e| anyhow!(e))?;
            Ok(root
                .find("div.attachments img[src]")
                .into_iter()
                .filter_map(|element| element.get_attribute("src").map(|src| src.to_string()))
                .find(|src| !src.is_empty())
                .map(|url| {
                    if url.starts_with("/") {
                        return format!("https://nitter.it{}", url);
                    }
                    url
                }))
        }
        _ => Err(anyhow!("Cannot find twitter id")),
    }
}

async fn download_image(url: &url::Url) -> Result<String> {
    let response = CLIENT.get(url.as_str()).send().await?.error_for_status()?;
    let content_type = response
        .headers()
        .get("Content-Type")
        .ok_or_else(|| anyhow!("Cannot find Content-Type from response {}", url))?
        .to_str()?;
    let file_name = match utils::extract_filename_from_url(url) {
        None => bail!("Failed to extract filename from url {}", url.as_str()),
        Some((name, None)) => {
            let extension = content_type
                .strip_prefix("image/")
                .ok_or_else(|| anyhow!("Unexpected Content-Type value {}", content_type))?;
            format!("{}.{}", name, extension)
        }
        Some((name, Some(extension))) => format!("{}.{}", name, extension),
    };
    let path = format!("{}/{}", cfg::BOT_CONFIG.download_path, file_name);

    let _ = utils::download_file_if_not_exists(response, &path).await?;
    Ok(path)
}

async fn process_image(path: String) -> Result<String> {
    let output_path = tokio::task::spawn_blocking(move || -> Result<String> {
        do_process_image(&path, &cfg::BOT_CONFIG.download_path)
    })
    .await?
    .context("Error running image processing task")?;

    Ok(output_path)
}

fn do_process_image(image_path: &str, output_directory: &str) -> Result<String> {
    let img = image::open(image_path)
        .context("Failed to open image")?
        .grayscale()
        .into_luma8();
    let mut img = imageproc::filter::box_filter(&img, 4, 4);

    let avg = {
        let mut avg = 0f64;
        let total = (img.height() * img.width()) as f64;
        for y in 0..img.height() {
            for x in 0..img.width() {
                let l = img.get_pixel(x, y);
                avg += l.0[0] as f64 / total;
            }
        }
        avg as u8
    };
    imageproc::contrast::threshold_mut(&mut img, avg);

    static ALPHA: f32 = 0.1;
    let mut blend = image::ImageBuffer::new(img.width(), img.height());
    for y in 0..img.height() {
        for x in 0..img.width() {
            let image::Luma([luma]) = img.get_pixel(x, y);
            let luma = ((255f32 * (1.0 - ALPHA)) + (*luma as f32) * ALPHA) as u8;
            blend.put_pixel(x, y, image::Luma([luma]));
        }
    }

    let path = format!("{}/{}.png", output_directory, nanoid!());
    let mut writer = std::io::BufWriter::new(
        std::fs::File::create(&path).context("Failed to create image file")?,
    );
    blend
        .write_to(&mut writer, image::ImageOutputFormat::Png)
        .context("Failed to write image file")?;

    Ok(path)
}

#[cfg(test)]
mod tests {
    use crate::image::get_twitter_image_url;

    use super::do_process_image;

    #[tokio::test]
    async fn get_image_url_test() {
        let result = get_twitter_image_url(
            "https://mobile.twitter.com/mery__S2_/status/1576244843281797120",
        )
        .await;
        assert_eq!(
            result.unwrap(),
            Some(
                "https://nitter.it/pic/enc/bWVkaWEvRmRfeHhfTGFjQUFzVGlqLmpwZz9uYW1lPXNtYWxs"
                    .to_string()
            )
        );
    }

    #[test]
    fn do_process_image_test() {
        let result = do_process_image(
            "/Users/darkyzhou/Downloads/IMG_0780.JPG",
            "/Users/darkyzhou/Downloads",
        );
        assert!(result.is_ok());
    }
}
