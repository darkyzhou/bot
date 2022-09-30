use anyhow::{anyhow, bail, Context, Result};
use futures_util::StreamExt;
use lazy_static::lazy_static;
use log::{info, warn};
use nanoid::nanoid;
use regex::Regex;
use std::path::Path;
use std::time::Duration;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use url::Url;
use visdom::Vis;

use crate::cfg;
use crate::client::*;
use crate::message::*;

pub async fn on_private_message(message: OneBotPrivateMessage) -> Option<BotResponseAction> {
    let OneBotPrivateMessage {
        user_id, message, ..
    } = message;
    if user_id != cfg::BOT_CONFIG.admin_user_id {
        return None;
    }

    let message = message.trim();
    match handle_download_command(&message).await {
        Some(Ok((size, _))) => Some(BotResponseAction::PrivateMessage {
            user_id,
            message: format!(
                "视频保存成功，大小: {}",
                human_bytes::human_bytes(size as f64)
            ),
        }),
        Some(Err(err)) => Some(BotResponseAction::PrivateMessage {
            user_id,
            message: format!("保存视频时出错: {:#?}", err),
        }),
        None => None,
    }
}

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

    let message = message.trim();
    match handle_download_command(message).await {
        Some(Ok((_, path))) => Some(BotResponseAction::GroupFile {
            group_id,
            file: path.clone(),
            name: Path::new(&path).file_name()?.to_str()?.to_string(),
        }),
        Some(Err(err)) => Some(BotResponseAction::GroupMessage {
            group_id,
            message: format!("保存视频时出错: {:#?}", err),
        }),
        None => None,
    }
}

async fn handle_download_command(message: &str) -> Option<Result<(u64, String)>> {
    if message.contains("twitter.com") {
        Some(download_twitter_video(message).await)
    } else if let Some(url) = message.strip_prefix("v ") {
        Some(download_video(url).await)
    } else {
        None
    }
}

async fn download_twitter_video(url: &str) -> Result<(u64, String)> {
    async fn do_request(url: &str) -> Result<String> {
        let result = CLIENT
            .post("https://twdown.net/download.php")
            .form(&[("URL", url.to_string())])
            .header(reqwest::header::REFERER, "https://twdown.net/")
            .header(reqwest::header::ORIGIN, "https://twdown.net/")
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;
        Ok(result)
    }

    let response = do_request(url).await.context("failed to request the api")?;
    let url = find_url_from_response(&response)?
        .ok_or_else(|| anyhow!(format!("failed to find url from response: {}", response)))?;

    let size = download_video(&url)
        .await
        .context("failed to download the video")?;
    Ok(size)
}

fn find_url_from_response(html: &str) -> Result<Option<String>> {
    lazy_static! {
        static ref RESOLUTION_REGEX: Regex = Regex::new(r"/(\d+)x(\d+)/").unwrap();
    }

    let root = Vis::load(html).map_err(|e| anyhow!(e))?;
    Ok(root
        .find("a[href]")
        .into_iter()
        .filter_map(|element| element.get_attribute("href"))
        .filter_map(|resolution| {
            let resolution = resolution.to_string();
            let caps = RESOLUTION_REGEX.captures(&resolution)?;
            match (caps.get(1), caps.get(2)) {
                (Some(w), Some(h)) => {
                    match (w.as_str().parse::<u64>(), h.as_str().parse::<u64>()) {
                        (Ok(w), Ok(h)) => Some((resolution, w * h)),
                        _ => None,
                    }
                }
                _ => None,
            }
        })
        .max_by(|(_, r_a), (_, r_b)| r_a.cmp(r_b))
        .map(|(url, _)| url))
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

async fn download_video(url: &str) -> Result<(u64, String)> {
    for times in 1..4 {
        match do_download_video(url).await {
            Ok(size) => {
                return Ok(size);
            }
            Err(err) => {
                if times >= 3 {
                    return Err(err.context("download exceeds maximum retry times"));
                }
            }
        }
        tokio::time::sleep(Duration::from_secs(3)).await;
    }
    return Err(anyhow!("download exceeds maximum retry times"));
}

async fn do_download_video(url: &str) -> Result<(u64, String)> {
    let response = CLIENT.get(url).send().await?.error_for_status()?;
    let file_name = get_file_name(response.url()).unwrap_or(format!("{}.mp4", nanoid!()));
    let path = format!("{}/{}", cfg::BOT_CONFIG.twitter_videos_path, file_name);
    let size = response.content_length().unwrap_or(0);
    info!("downloading video from {} to {}, size: {}", url, path, size);

    if let Ok(metadata) = tokio::fs::metadata(&path).await {
        let len = metadata.len();
        if len == size {
            return Ok((size, path));
        }
        warn!(
            "video file {} exists, but size unmatched, expected {} actual {}",
            path, size, len
        )
    }

    let mut file = File::create(&path).await?;
    let mut stream = response.bytes_stream();
    while let Some(item) = stream.next().await {
        file.write_all(&item?).await?;
    }
    info!("download finished for url {}", url);
    Ok((size, path))
}

#[cfg(test)]
mod tests {
    use crate::download::find_url_from_response;

    #[test]
    fn extract_twdown_response() {
        assert_eq!(
            find_url_from_response(
                r###"<!DOCTYPE html>
<html lang="en">
<head>
	<title>Download Twitter Videos in MP4 & MP3</title>
	<meta name="description" content="Here you can download your favorite twitter videos in MP4 or convert them to MP3 all in 1 click and without using any software, java or extensions.">
	<meta name="viewport" content="width=device-width, initial-scale=1.0">
	<meta charset="utf-8">
	
	<link href="css/bootstrap.min.css" rel="stylesheet">
	<link href="css/style.css" rel="stylesheet">
	
	<!-- HTML5 shim, for IE6-8 support of HTML5 elements -->
	<!--[if lt IE 9]>
	<script src="js/html5shiv.js"></script>
	<![endif]-->

	<link rel="apple-touch-icon-precomposed" sizes="144x144" href="img/apple-touch-icon-144-precomposed.png">
	<link rel="apple-touch-icon-precomposed" sizes="114x114" href="img/apple-touch-icon-114-precomposed.png">
	<link rel="apple-touch-icon-precomposed" sizes="72x72" href="img/apple-touch-icon-72-precomposed.png">
	<link rel="apple-touch-icon-precomposed" href="img/apple-touch-icon-57-precomposed.png">
	<link rel="shortcut icon" href="favicon.png">

	<script type="text/javascript" src="js/jquery.min.js"></script>
	<script type="text/javascript" src="js/bootstrap.min.js"></script>
	<script type="text/javascript" src="js/scripts.js"></script>
		<style>
	    .card-horizontal 
	    {
            display: flex;
            flex: 1 1 auto;
        }
	</style>
</head>
<body>
	<script>
  (function(i,s,o,g,r,a,m){i['GoogleAnalyticsObject']=r;i[r]=i[r]||function(){
  (i[r].q=i[r].q||[]).push(arguments)},i[r].l=1*new Date();a=s.createElement(o),
  m=s.getElementsByTagName(o)[0];a.async=1;a.src=g;m.parentNode.insertBefore(a,m)
  })(window,document,'script','https://www.google-analytics.com/analytics.js','ga');

  ga('create', 'UA-89665192-1', 'auto');
  ga('send', 'pageview');

</script>	<div class="navbar navbar-default navbar-static-top">
  <div class="container">
    <div class="navbar-header">
      <button type="button" class="navbar-toggle" data-toggle="collapse" data-target=".navbar-collapse">
        <span class="icon-bar"></span>
        <span class="icon-bar"></span>
        <span class="icon-bar"></span>
      </button>
      <a class="navbar-brand"href="index.php" title="Twitter Downloader Online"><img class="img-responsive" alt="Twitter Video Downloader Online" style="margin-top: -2px;" src="img/twdown.net.png" width="160px"></a>
    </div>
    <div class="collapse navbar-collapse">
      <ul class="nav navbar-nav">
        <li><a href="how-to-download-twitter-videos.php"><strong>How to Download?</strong></a></li>
        <li><a href="https://fdown.net" target="_blank">Facebook Downloader</a></li>
      </ul>
      <ul class="nav navbar-nav navbar-right">
        <li><a href="contact.php">Contact us</a></li>
        <li><a href="about.php">About</a></li>
      </ul>
    </div><!--/.nav-collapse -->
  </div>
</div>			<div class="jumbotron">
			<div class="container">
			<center>
				<a href="index.php" title="Twitter Video Downloader"><img class="img-responsive" alt="Twitter Video Downloader online" title="Twitter Video Downloader" src="img/twitter-video-downloader-online.png" width="280px"></a>
				<h1 style="font-size:19px;margin-top:-0%;font-weight:bold">Twitter Video Downloader</h1>
				<h2 style="font-size:16px;margin-top:-0.5%">Download your Twitter video in MP4 or MP3</h2>
				<br />
				<div style="float:none;" class="col-lg-8">
									</div>
				<br />
					<div class="row" style="height: auto !important; min-height: 0px !important;">
<div class="col-md-8 col-md-offset-2">
	<div class="col-md-8 col-md-offset-2">
		<div class="col-md-6">
			<img class="img-thumbnail" src="https://pbs.twimg.com/ext_tw_video_thumb/1575500658668892166/pu/img/TfYltANvsBdfkAez.jpg" width="240px">			
		</div>
		<div style="overflow-wrap: break-word;" class="col-md-6"><h4><strong>AWS Architecture</strong></h4><p>Tune in RIGHT NOW to watch #BuildOnLive: Observability Day! Hear from observability experts &amp;amp; learn strategies, tips &amp;amp; tricks, &amp;amp; what&rsquo;s new in the open source landscape. One, turn on &lsquo;puter. Two, go to Twitch. Three, let&rsquo;s go 💻 https://t.co/eqXz9b33xs

#awsobservability https://t.co/DCv0hKoIHE</p></div>
		
	</div>
	<br />
	<div class="col-md-10 col-md-offset-1">
	<script async src="//pagead2.googlesyndication.com/pagead/js/adsbygoogle.js"></script>
<!-- TwDown-1 d-->
<ins class="adsbygoogle"
     style="display:block;"
     data-ad-client="ca-pub-9232253514792453"
     data-ad-slot="3333461467"
     data-ad-format="horizontal"></ins>
<script>
(adsbygoogle = window.adsbygoogle || []).push({});
</script>	</div>
	<br />
	<div class="col-md-8 col-md-offset-2">
    	<table class="table table-bordered table-hover table-striped" style="width: 100%;-webkit-box-shadow: 0px 0px 32px -13px rgba(0,0,0,0.75);-moz-box-shadow: 0px 0px 32px -13px rgba(0,0,0,0.75);box-shadow: 0px 0px 32px -13px rgba(0,0,0,0.75);">
    	   <thead>
    	      <tr class="active">
    	      	<th><span class="glyphicon glyphicon-eye-open"></span></th>
    		<th>Quality <span class="glyphicon glyphicon-hd-video"></span></th>
    		<th>File Type <span class="glyphicon glyphicon-play-circle"></span></th>
    		<th>Downloads <span class="glyphicon glyphicon-download"></span></th>
    	      </tr>
    	   </thead>
    	   <tbody>
    	   
    		    			    <tr>	
    				    <td><a data-toggle="modal" data-target="#myModal" href="#" onclick="preview_video('https://video.twimg.com/ext_tw_video/1575500658668892166/pu/vid/320x320/61V_Gb5BeVrOj0T0.mp4?tag=12')"><span class="glyphicon glyphicon-play"></span></a>
    			        <td>320x320</td>
    			        <td>MP4</td>
    			        <td><a download href="https://video.twimg.com/ext_tw_video/1575500658668892166/pu/vid/320x320/61V_Gb5BeVrOj0T0.mp4?tag=12" target="_blank"><strong>Download</strong></a></td>
    		        </tr>
    		    			    <tr>	
    				    <td><a data-toggle="modal" data-target="#myModal" href="#" onclick="preview_video('https://video.twimg.com/ext_tw_video/1575500658668892166/pu/vid/540x540/zumkZ68pLyJf7O0h.mp4?tag=12')"><span class="glyphicon glyphicon-play"></span></a>
    			        <td>540x540</td>
    			        <td>MP4</td>
    			        <td><a download href="https://video.twimg.com/ext_tw_video/1575500658668892166/pu/vid/540x540/zumkZ68pLyJf7O0h.mp4?tag=12" target="_blank"><strong>Download</strong></a></td>
    		        </tr>
    		    			    <tr>	
    				    <td><a data-toggle="modal" data-target="#myModal" href="#" onclick="preview_video('https://video.twimg.com/ext_tw_video/1575500658668892166/pu/vid/720x720/UV8ZvfS9WH0lZrV2.mp4?tag=12')"><span class="glyphicon glyphicon-play"></span></a>
    			        <td>720x720</td>
    			        <td>MP4</td>
    			        <td><a download href="https://video.twimg.com/ext_tw_video/1575500658668892166/pu/vid/720x720/UV8ZvfS9WH0lZrV2.mp4?tag=12" target="_blank"><strong>Download</strong></a></td>
    		        </tr>
    		    		<tr>
    					<td></td>
    					<th>Convert to MP3</th>
    					<td>MP3</td>
    					<td><a href='mp3.php?v=MjE9Z2F0PzRwbS4yVnJabDBIVzlTZnZaOFZVLzAyN3gwMjcvZGl2L3VwLzY2MTI5ODg2Njg1NjAwNTU3NTEvb2VkaXZfd3RfdHhlL21vYy5nbWl3dC5vZWRpdi8vOnNwdHRo&t=token%260c05c632a2822a0a877c7e991602543'><strong>Download</strong></a></td>
    		</tr>				
    	   </tbody>
    	</table>
	</div>
	<br />
	
</div>
</div>
<div class="col-md-2"></div>
<!-- Modal -->
<div class="modal fade" id="myModal" tabindex="-1" role="dialog" data-backdrop="static" aria-labelledby="myModalLabel">
	<div class="modal-dialog" role="document">
		<div class="modal-content">
			<div class="modal-header">
				<button type="button" onclick="destroy()" class="close" data-dismiss="modal" aria-label="Close"><span aria-hidden="true">&times;</span></button>
				<h4 class="modal-title" id="myModalLabel">Preview Video</h4>
			</div>
			<div class="modal-body">
				<div id="previewer" class="embed-responsive embed-responsive-16by9">
								
				</div>
			</div>
			<div class="modal-footer">
				<button type="button" class="btn btn-default" data-dismiss="modal" onclick="destroy()">Close</button>
			</div>
		</div>
	</div>
</div>
<script>
	function preview_video(x)
	{
		var video = $
		(
			'<video />', 
			{
				id: 'preview',
				src: x,
				type: 'video/mp4',
				controls: true
			}
		);
			video.appendTo($('#previewer'));	
	}
	
	function destroy()
	{
		$("#preview").remove();
	}
</script>
				<br />
				<div style="float:none;" class="col-lg-8">
								</div>
				<br />
			</center>
			</div>
		
		</div>
		<div class="container">
		<div class="row" style="display: -webkit-box; display: -webkit-flex; display: -ms-flexbox; display: flex;">
			<div class="well col-md-3">
				<h2 class="window-head">Twitter Video Downloader Online</h2>
				<img src="img/download-twitter-videos.png" style="float: left;max-width: 48px;margin: 10px;">TWDown is the best and most secure free Twitter video downloader online tool, it helps you generate direct links for your favorite twitter videos and save them for offline viewing and sharing.
			</div>
			<div class="col-md-1">
			</div>
			<div class="well col-md-4">
				<h2 class="window-head">Download Twitter Videos Fast</h2>
				<img src="img/twitter-video-converter.png" style="float: left;max-width: 48px;margin: 10px;">With TWDown.net you'll be able to get any twitter video in almost no time, as it is powered by the most powerful server we have, and it should be the fastest twitter videos downloader online.
			</div>
			<div class="col-md-1">
			</div>
			<div class="well col-md-3">
				<h2 class="window-head">Twitter to MP3 Converter</h2>
				<img src="img/twitter-to-mp3-converter-online.png" style="float: left;max-width: 48px;margin: 10px;">TWDown also helps you convert your favorite twitter videos to MP3 and listen to them offline while on the go, you can convert any type of twitter videos to MP3 online including Music.
			</div>
		</div>
		0			<br /><div class="well col-md-12">
<center>
<strong>Twitter Online Video Downloader </strong> | <a href="privacy.php">Privacy Policy</a><br />
Our other service : <a href="http://fbdown.net/"><strong>Facebook Video Downloader</strong></a>
</center>
</div>		</div>
</body>
</html>"###
            ).unwrap(),
            Some("https://video.twimg.com/ext_tw_video/1575500658668892166/pu/vid/720x720/UV8ZvfS9WH0lZrV2.mp4?tag=12".to_string())
        );
    }
}
