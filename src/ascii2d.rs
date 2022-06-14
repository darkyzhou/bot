use std::collections::HashMap;
use std::time::Duration;
use async_trait::async_trait;
use visdom::Vis;
use snafu::prelude::*;

use crate::searcher::*;

#[derive(Debug, Snafu)]
pub enum Ascii2dError {
    #[snafu(display("Failed to parse response html from ascii2d"))]
    ParseHtml {
        url: String,
        source: visdom::types::BoxDynError,
    },
    #[snafu(display("Failed to find source url from parsed html"))]
    FindSourceUrl {
        url: String,
        html: String,
    },
}

pub struct Ascii2dImageSearcher {}

#[async_trait]
impl ImageSearcher for Ascii2dImageSearcher {
    fn get_name(&self) -> &'static str {
        "ascii2d"
    }

    async fn search(&self, url: &str) -> ImageSearchResult {
        let client = reqwest::Client::new();
        let response = client.get(format!("https://ascii2d.net/search/url/{}", url))
            .header(reqwest::header::USER_AGENT, "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/102.0.0.0 Safari/537.36")
            .timeout(Duration::from_secs(15))
            .send()
            .await?
            .error_for_status()?;
        let html = response.text().await?;
        parse_result(url, html.as_str())
    }
}

fn parse_result(url: &str, html: &str) -> ImageSearchResult {
    let root = Vis::load(html).map_err(|e| Ascii2dError::ParseHtml {
        url: url.to_string(),
        source: e,
    })?;
    let item_box = root.find(".item-box ~ .item-box");
    let source_url = item_box.find(".detail-box a:nth-of-type(1)").attr("href").ok_or(Ascii2dError::FindSourceUrl {
        url: url.to_string(),
        html: html.to_string(),
    })?.to_string();
    let metadata = {
        let name = item_box.find(".detail-box a:nth-of-type(2)").html();
        if name.is_empty() {
            HashMap::default()
        } else {
            HashMap::from([("作者".to_string(), name)])
        }
    };

    Ok(Some(SourceImage {
        url: source_url,
        metadata,
    }))
}

#[cfg(test)]
mod tests {
    use crate::ascii2d::parse_result;
    use crate::SourceImage;
    use std::collections::HashMap;

    #[test]
    fn parse_result_test() {
        let raw_html = r#"<!DOCTYPE html>
<html lang='ja'>
<head>
<meta charset='utf-8'>
<meta content='width=device-width,initial-scale=1.0,minimum-scale=1.0' name='viewport'>
<title>二次元画像詳細検索</title>
<link rel="shortcut icon" type="image/x-icon" href="/assets/favicon-461e7af86f6c1a73f716cf8c729e65d6164851b66470932d01ef928ebbaed6ba.ico" />
<link rel="stylesheet" media="screen" href="/assets/application-2e127fee08fa600eb645946ab08a0881b955052a83c54c9427c4cf91a3a5aa72.css" data-turbolinks-track="true" />
<script src="/assets/application-1f7760e9e18c27155fe55e67c00edf533a88a016796bf6ab72641adf2a0d1ddd.js" data-turbolinks-track="true"></script>

<meta name="csrf-param" content="authenticity_token" />
<meta name="csrf-token" content="t7DHTESJ2eKBQ5U7pMLkoKPWg5kFyUTzS4ppsRtIZJPw6VA8PxEFhKdtifJ4i1iq+MDevVPmmHrfaa/jOO24wQ==" />
<script async src='/cdn-cgi/challenge-platform/h/g/scripts/invisible.js?ts=1655193600'></script></head>
<body>
<div class='container'>
<header class='navbar navbar-static-top' id='header' role='banner'>
<div class='clearfix'>
<div class='row'>
<a class='hidden-md-up nav-item nav-link navbar-brand' href='/'>二次元画像詳細検索</a>
<button class='hidden-md-up navbar-toggler pull-xs-right' data-target='#menu-bar' data-toggle='collapse' type='button'>
<span class='small navbar-menu'>目次</span>
</button>
</div>
<div class='row'>
<div class='search-nav-bar pull-md-left'>
<ul class='nav hidden-sm-down'>
<li class='nav-item'>
<form class="form-inline" id="nav-search" enctype="multipart/form-data" action="/search/multi" accept-charset="UTF-8" method="post"><input name="utf8" type="hidden" value="&#x2713;" /><input type="hidden" name="authenticity_token" value="mmWbAXZAfWfwpn00NeD6kZydU6VkQxQGAC9ZZDyTSTfdPAxxDdihAdaIYf3pqUabx4sOgTJsyI+UzJ82HzaVZQ==" /><div class='form-group'>
<input class='form-control form-control-sm' id='nav-file-form' name='file' placeholder='ファイル' type='file'>
</div>
<div class='form-group'>
<input class='form-control form-control-sm' name='uri' placeholder='画像のURL' type='url' value=''>
</div>
<button class='btn btn-secondary btn-sm text-muted' name='search' type='submit'>検索</button>
</form></li>
</ul>
</div>
<div class='collapse navbar-toggleable-sm' id='menu-bar'>
<ul class='nav navbar-nav pull-md-right'>
<li class='nav-item'>
<a class='nav-link' href='/'>top</a>
</li>
<li class='nav-item'>
<a class='nav-link' href='/readme'>説明</a>
</li>
<li class='nav-item'>
<a class='nav-link' href='/recently'>最近の検索</a>
</li>
<li class='nav-item'>
<a class='nav-link' href='/ranking/daily'>ランキング</a>
</li>
<li class='nav-item dropdown'>
<a aria-expanded='false' class='dropdown-toggle nav-link' data-toggle='dropdown' href='#' role='button'>ツール</a>
<div class='dropdown-menu'>
<a class='dropdown-item' href='https://chrome.google.com/webstore/detail/dlnbkfiafmkajgbhpdfmkeljamdlfelo' rel='noopener' target='_blank'>Chrome拡張</a>
<div class='dropdown-divider'></div>
<a class='dropdown-item' href='https://addons.mozilla.org/ja/firefox/addon/256705/' rel='noopener' target='_blank'>Firefox拡張</a>
<div class='dropdown-divider'></div>
<a class='dropdown-item' href='https://microsoftedge.microsoft.com/addons/detail/ohjihjimkibfeigmbkijiklcamdenido' target='_blank'>Edge拡張</a>
</div>
</li>
<li class='nav-item dropdown'>
<a aria-expanded='false' class='dropdown-toggle nav-link' data-toggle='dropdown' href='#' role='button'>連絡先</a>
<div class='dropdown-menu'>
<a class='dropdown-item' href='https://jbbs.shitaraba.net/computer/42759/' rel='noopener' target='_blank'>したらば掲示板</a>
<div class='dropdown-divider'></div>
<a class='dropdown-item' href='https://twitter.com/ascii2d' rel='noopener' target='_blank'>twitter</a>
<div class='dropdown-divider'></div>
<a class='dropdown-item' href="/cdn-cgi/l/email-protection#017664636c607275647341607262686833652f6f6475"><span class="__cf_email__" data-cfemail="93e4f6f1fef2e0e7f6e1d3f2e0f0fafaa1f7bdfdf6e7">[email&#160;protected]</span></a>
</div>
</li>
</ul>
</div>
</div>
</div>
</header>


<div class='row'>
<div class='col-xs-12 col-lg-8 col-xl-8'>
<h5 class='p-t-1 text-xs-center'>色合検索</h5>
<hr>
<div class='row item-box'>
<div class='col-xs-12 col-sm-12 col-md-4 col-xl-4 text-xs-center image-box'>
<img loading="eager" src="/thumbnail/8/3/7/5/83755f962f130abf929860113fd9b989.jpg" alt="83755f962f130abf929860113fd9b989" width="142" height="200" />
</div>
<div class='col-xs-12 col-sm-12 col-md-8 col-xl-8 info-box'>
<div class='hash'>83755f962f130abf929860113fd9b989</div>
<small class='text-muted'>849x1200 JPEG 829.9KB</small>
<div class='pull-xs-right'></div>
<div class='detail-box gray-link'>
</div>
</div>
<div class='detail-link pull-xs-right hidden-sm-down gray-link'>
<span><a href="/search/color/83755f962f130abf929860113fd9b989">色合検索</a></span>
<span><a href="/search/bovw/83755f962f130abf929860113fd9b989">特徴検索</a></span>
<span><a href="/details/83755f962f130abf929860113fd9b989/new">詳細登録</a></span>
</div>
<div class='btn-block text-xs-center hidden-md-up p-d-1'>
<a class="btn btn-secondary" href="/search/color/83755f962f130abf929860113fd9b989">色合検索</a>
<a class="btn btn-secondary" href="/search/bovw/83755f962f130abf929860113fd9b989">特徴検索</a>
<a class="btn btn-secondary" href="/details/83755f962f130abf929860113fd9b989/new">詳細登録</a>
</div>
</div>
<div class='clearfix'></div>

<hr>
<div class='row item-box'>
<div class='col-xs-12 col-sm-12 col-md-4 col-xl-4 text-xs-center image-box'>
<img loading="lazy" src="/thumbnail/7/8/0/8/78082c083ce210fc0e803d4dfa5c7ab2.jpg" alt="78082c083ce210fc0e803d4dfa5c7ab2" width="142" height="200" />
</div>
<div class='col-xs-12 col-sm-12 col-md-8 col-xl-8 info-box'>
<div class='hash'>78082c083ce210fc0e803d4dfa5c7ab2</div>
<small class='text-muted'>2480x3508 JPEG 3906.0KB</small>
<div class='pull-xs-right'></div>
<div class='detail-box gray-link'>
<h6>
<img src="/assets/twitter-15e2a6aec006e029bcccaf870ab8606a4c03a7ff3df90239ff5cd889ca585a39.ico" alt="Twitter" width="14" height="14" />
<a target="_blank" rel="noopener" href="https://twitter.com/tororo51791023/status/1521412000689324032">2022.05.03</a>
<a target="_blank" rel="noopener" href="https://twitter.com/intent/user?user_id=900369275050962944">tororo51791023</a>
<small class='text-muted'>twitter</small>
</h6>

</div>
</div>
<div class='detail-link pull-xs-right hidden-sm-down gray-link'>
<span><a href="/search/color/78082c083ce210fc0e803d4dfa5c7ab2">色合検索</a></span>
<span><a href="/search/bovw/78082c083ce210fc0e803d4dfa5c7ab2">特徴検索</a></span>
<span><a href="/details/78082c083ce210fc0e803d4dfa5c7ab2/new">詳細登録</a></span>
</div>
<div class='btn-block text-xs-center hidden-md-up p-d-1'>
<a class="btn btn-secondary" href="/search/color/78082c083ce210fc0e803d4dfa5c7ab2">色合検索</a>
<a class="btn btn-secondary" href="/search/bovw/78082c083ce210fc0e803d4dfa5c7ab2">特徴検索</a>
<a class="btn btn-secondary" href="/details/78082c083ce210fc0e803d4dfa5c7ab2/new">詳細登録</a>
</div>
</div>
<div class='clearfix'></div>

<hr>
<div class='row item-box'>
<div class='col-xs-12 col-sm-12 col-md-4 col-xl-4 text-xs-center image-box'>
<img loading="lazy" src="/thumbnail/4/f/0/f/4f0fbadcb40f57dd61813db66677e818.jpg" alt="4f0fbadcb40f57dd61813db66677e818" width="142" height="200" />
</div>
<div class='col-xs-12 col-sm-12 col-md-8 col-xl-8 info-box'>
<div class='hash'>4f0fbadcb40f57dd61813db66677e818</div>
<small class='text-muted'>2480x3508 JPEG 4250.1KB</small>
<div class='pull-xs-right'></div>
<div class='detail-box gray-link'>
<h6>
<img class="to-link-icon" src="/assets/pixiv-628a47348a82153ebc34acba4e5b287777a11631bb382dbb00fd4b88083bed95.ico" alt="Pixiv" width="14" height="14" />
<a target="_blank" rel="noopener" href="https://www.pixiv.net/artworks/98072196">芙洛伦唤醒皮肤</a>
<a target="_blank" rel="noopener" href="https://www.pixiv.net/users/8782224">药锅锅</a>
<small>
pixiv
</small>
</h6>

</div>
</div>
<div class='detail-link pull-xs-right hidden-sm-down gray-link'>
<span><a href="/search/color/4f0fbadcb40f57dd61813db66677e818">色合検索</a></span>
<span><a href="/search/bovw/4f0fbadcb40f57dd61813db66677e818">特徴検索</a></span>
<span><a href="/details/4f0fbadcb40f57dd61813db66677e818/new">詳細登録</a></span>
</div>
<div class='btn-block text-xs-center hidden-md-up p-d-1'>
<a class="btn btn-secondary" href="/search/color/4f0fbadcb40f57dd61813db66677e818">色合検索</a>
<a class="btn btn-secondary" href="/search/bovw/4f0fbadcb40f57dd61813db66677e818">特徴検索</a>
<a class="btn btn-secondary" href="/details/4f0fbadcb40f57dd61813db66677e818/new">詳細登録</a>
</div>
</div>
<div class='clearfix'></div>

<hr>
<div class='row item-box'>
<div class='col-xs-12 col-sm-12 col-md-4 col-xl-4 text-xs-center image-box'>
<img loading="lazy" src="/thumbnail/b/5/e/6/b5e652ffcd05dfeec231142fc1fa9996.jpg" alt="B5e652ffcd05dfeec231142fc1fa9996" width="150" height="200" />
</div>
<div class='col-xs-12 col-sm-12 col-md-8 col-xl-8 info-box'>
<div class='hash'>b5e652ffcd05dfeec231142fc1fa9996</div>
<small class='text-muted'>1536x2048 JPEG 1112.9KB</small>
<div class='pull-xs-right'></div>
<div class='detail-box gray-link'>
<h6>
<img class="to-link-icon" src="/assets/pixiv-628a47348a82153ebc34acba4e5b287777a11631bb382dbb00fd4b88083bed95.ico" alt="Pixiv" width="14" height="14" />
<a target="_blank" rel="noopener" href="https://www.pixiv.net/artworks/77970054">Twitter落書き詰め詰め(3)</a>
<a target="_blank" rel="noopener" href="https://www.pixiv.net/users/37712046">あめ?</a>
<small>
pixiv
</small>
</h6>

</div>
</div>
<div class='detail-link pull-xs-right hidden-sm-down gray-link'>
<span><a href="/search/color/b5e652ffcd05dfeec231142fc1fa9996">色合検索</a></span>
<span><a href="/search/bovw/b5e652ffcd05dfeec231142fc1fa9996">特徴検索</a></span>
<span><a href="/details/b5e652ffcd05dfeec231142fc1fa9996/new">詳細登録</a></span>
</div>
<div class='btn-block text-xs-center hidden-md-up p-d-1'>
<a class="btn btn-secondary" href="/search/color/b5e652ffcd05dfeec231142fc1fa9996">色合検索</a>
<a class="btn btn-secondary" href="/search/bovw/b5e652ffcd05dfeec231142fc1fa9996">特徴検索</a>
<a class="btn btn-secondary" href="/details/b5e652ffcd05dfeec231142fc1fa9996/new">詳細登録</a>
</div>
</div>
<div class='clearfix'></div>

<hr>
<div class='row item-box'>
<div class='col-xs-12 col-sm-12 col-md-4 col-xl-4 text-xs-center image-box'>
<img loading="lazy" src="/thumbnail/8/8/f/6/88f6573b2d38300bf80c51f3b4d21d9b.jpg" alt="88f6573b2d38300bf80c51f3b4d21d9b" width="150" height="200" />
</div>
<div class='col-xs-12 col-sm-12 col-md-8 col-xl-8 info-box'>
<div class='hash'>88f6573b2d38300bf80c51f3b4d21d9b</div>
<small class='text-muted'>834x1112 JPEG 543.9KB</small>
<div class='pull-xs-right'></div>
<div class='detail-box gray-link'>
<h6>
<img class="to-link-icon" src="/assets/pixiv-628a47348a82153ebc34acba4e5b287777a11631bb382dbb00fd4b88083bed95.ico" alt="Pixiv" width="14" height="14" />
<a target="_blank" rel="noopener" href="https://www.pixiv.net/artworks/72393700">ログ</a>
<a target="_blank" rel="noopener" href="https://www.pixiv.net/users/2120235">らりねこ</a>
<small>
pixiv
</small>
</h6>

</div>
</div>
<div class='detail-link pull-xs-right hidden-sm-down gray-link'>
<span><a href="/search/color/88f6573b2d38300bf80c51f3b4d21d9b">色合検索</a></span>
<span><a href="/search/bovw/88f6573b2d38300bf80c51f3b4d21d9b">特徴検索</a></span>
<span><a href="/details/88f6573b2d38300bf80c51f3b4d21d9b/new">詳細登録</a></span>
</div>
<div class='btn-block text-xs-center hidden-md-up p-d-1'>
<a class="btn btn-secondary" href="/search/color/88f6573b2d38300bf80c51f3b4d21d9b">色合検索</a>
<a class="btn btn-secondary" href="/search/bovw/88f6573b2d38300bf80c51f3b4d21d9b">特徴検索</a>
<a class="btn btn-secondary" href="/details/88f6573b2d38300bf80c51f3b4d21d9b/new">詳細登録</a>
</div>
</div>
<div class='clearfix'></div>

<hr>
<div class='row item-box'>
<div class='col-xs-12 col-sm-12 col-md-4 col-xl-4 text-xs-center image-box'>
<img loading="lazy" src="/thumbnail/2/2/1/f/221f5ae2016481e741eb29d6eb72c401.jpg" alt="221f5ae2016481e741eb29d6eb72c401" width="156" height="200" />
</div>
<div class='col-xs-12 col-sm-12 col-md-8 col-xl-8 info-box'>
<div class='hash'>221f5ae2016481e741eb29d6eb72c401</div>
<small class='text-muted'>600x771 PNG 66.7KB</small>
<div class='pull-xs-right'></div>
<div class='detail-box gray-link'>
<h6>
<img class="to-link-icon" src="/assets/pixiv-628a47348a82153ebc34acba4e5b287777a11631bb382dbb00fd4b88083bed95.ico" alt="Pixiv" width="14" height="14" />
<a target="_blank" rel="noopener" href="https://www.pixiv.net/artworks/43566211">桐横ついぴくろぐ2</a>
<a target="_blank" rel="noopener" href="https://www.pixiv.net/users/9692880">トキザワ@桐横用</a>
<small>
pixiv
</small>
</h6>

</div>
</div>
<div class='detail-link pull-xs-right hidden-sm-down gray-link'>
<span><a href="/search/color/221f5ae2016481e741eb29d6eb72c401">色合検索</a></span>
<span><a href="/search/bovw/221f5ae2016481e741eb29d6eb72c401">特徴検索</a></span>
<span><a href="/details/221f5ae2016481e741eb29d6eb72c401/new">詳細登録</a></span>
</div>
<div class='btn-block text-xs-center hidden-md-up p-d-1'>
<a class="btn btn-secondary" href="/search/color/221f5ae2016481e741eb29d6eb72c401">色合検索</a>
<a class="btn btn-secondary" href="/search/bovw/221f5ae2016481e741eb29d6eb72c401">特徴検索</a>
<a class="btn btn-secondary" href="/details/221f5ae2016481e741eb29d6eb72c401/new">詳細登録</a>
</div>
</div>
<div class='clearfix'></div>

<hr>
<div class='row item-box'>
<div class='col-xs-12 col-sm-12 col-md-4 col-xl-4 text-xs-center image-box'>
<img loading="lazy" src="/thumbnail/f/2/0/b/f20b5fdff8f219122efbcacab666dffe.jpg" alt="F20b5fdff8f219122efbcacab666dffe" width="126" height="200" />
</div>
<div class='col-xs-12 col-sm-12 col-md-8 col-xl-8 info-box'>
<div class='hash'>f20b5fdff8f219122efbcacab666dffe</div>
<small class='text-muted'>828x1316 JPEG 334.0KB</small>
<div class='pull-xs-right'></div>
<div class='detail-box gray-link'>
<h6>
<img class="to-link-icon" src="/assets/pixiv-628a47348a82153ebc34acba4e5b287777a11631bb382dbb00fd4b88083bed95.ico" alt="Pixiv" width="14" height="14" />
<a target="_blank" rel="noopener" href="https://www.pixiv.net/artworks/98216147">咲希ちゃん♡ハピバ♡</a>
<a target="_blank" rel="noopener" href="https://www.pixiv.net/users/79223724">みやこ</a>
<small>
pixiv
</small>
</h6>

</div>
</div>
<div class='detail-link pull-xs-right hidden-sm-down gray-link'>
<span><a href="/search/color/f20b5fdff8f219122efbcacab666dffe">色合検索</a></span>
<span><a href="/search/bovw/f20b5fdff8f219122efbcacab666dffe">特徴検索</a></span>
<span><a href="/details/f20b5fdff8f219122efbcacab666dffe/new">詳細登録</a></span>
</div>
<div class='btn-block text-xs-center hidden-md-up p-d-1'>
<a class="btn btn-secondary" href="/search/color/f20b5fdff8f219122efbcacab666dffe">色合検索</a>
<a class="btn btn-secondary" href="/search/bovw/f20b5fdff8f219122efbcacab666dffe">特徴検索</a>
<a class="btn btn-secondary" href="/details/f20b5fdff8f219122efbcacab666dffe/new">詳細登録</a>
</div>
</div>
<div class='clearfix'></div>

<hr>
<div class='row item-box'>
<div class='col-xs-12 col-sm-12 col-md-4 col-xl-4 text-xs-center image-box'>
<img loading="lazy" src="/thumbnail/9/b/b/5/9bb5c79d9af0d9e19a12e3511030a488.jpg" alt="9bb5c79d9af0d9e19a12e3511030a488" width="117" height="200" />
</div>
<div class='col-xs-12 col-sm-12 col-md-8 col-xl-8 info-box'>
<div class='hash'>9bb5c79d9af0d9e19a12e3511030a488</div>
<small class='text-muted'>828x1425 JPEG 394.4KB</small>
<div class='pull-xs-right'></div>
<div class='detail-box gray-link'>
<h6>
<img class="to-link-icon" src="/assets/pixiv-628a47348a82153ebc34acba4e5b287777a11631bb382dbb00fd4b88083bed95.ico" alt="Pixiv" width="14" height="14" />
<a target="_blank" rel="noopener" href="https://www.pixiv.net/artworks/98819102">ワンダショLINEまとめ</a>
<a target="_blank" rel="noopener" href="https://www.pixiv.net/users/79223724">みやこ</a>
<small>
pixiv
</small>
</h6>

</div>
</div>
<div class='detail-link pull-xs-right hidden-sm-down gray-link'>
<span><a href="/search/color/9bb5c79d9af0d9e19a12e3511030a488">色合検索</a></span>
<span><a href="/search/bovw/9bb5c79d9af0d9e19a12e3511030a488">特徴検索</a></span>
<span><a href="/details/9bb5c79d9af0d9e19a12e3511030a488/new">詳細登録</a></span>
</div>
<div class='btn-block text-xs-center hidden-md-up p-d-1'>
<a class="btn btn-secondary" href="/search/color/9bb5c79d9af0d9e19a12e3511030a488">色合検索</a>
<a class="btn btn-secondary" href="/search/bovw/9bb5c79d9af0d9e19a12e3511030a488">特徴検索</a>
<a class="btn btn-secondary" href="/details/9bb5c79d9af0d9e19a12e3511030a488/new">詳細登録</a>
</div>
</div>
<div class='clearfix'></div>

<hr>
<div class='row item-box'>
<div class='col-xs-12 col-sm-12 col-md-4 col-xl-4 text-xs-center image-box'>
<img loading="lazy" src="/thumbnail/6/b/5/0/6b50325d4cf88226d1398f55d923ba6f.jpg" alt="6b50325d4cf88226d1398f55d923ba6f" width="150" height="200" />
</div>
<div class='col-xs-12 col-sm-12 col-md-8 col-xl-8 info-box'>
<div class='hash'>6b50325d4cf88226d1398f55d923ba6f</div>
<small class='text-muted'>1536x2048 JPEG 398.8KB</small>
<div class='pull-xs-right'></div>
<div class='detail-box gray-link'>
<h6>
<img src="/assets/twitter-15e2a6aec006e029bcccaf870ab8606a4c03a7ff3df90239ff5cd889ca585a39.ico" alt="Twitter" width="14" height="14" />
<a target="_blank" rel="noopener" href="https://twitter.com/ryoutaou20011/status/885153478149980160">2017.07.13</a>
<a target="_blank" rel="noopener" href="https://twitter.com/intent/user?user_id=3048818431">ryoutaou20011</a>
<small class='text-muted'>twitter</small>
</h6>

</div>
</div>
<div class='detail-link pull-xs-right hidden-sm-down gray-link'>
<span><a href="/search/color/6b50325d4cf88226d1398f55d923ba6f">色合検索</a></span>
<span><a href="/search/bovw/6b50325d4cf88226d1398f55d923ba6f">特徴検索</a></span>
<span><a href="/details/6b50325d4cf88226d1398f55d923ba6f/new">詳細登録</a></span>
</div>
<div class='btn-block text-xs-center hidden-md-up p-d-1'>
<a class="btn btn-secondary" href="/search/color/6b50325d4cf88226d1398f55d923ba6f">色合検索</a>
<a class="btn btn-secondary" href="/search/bovw/6b50325d4cf88226d1398f55d923ba6f">特徴検索</a>
<a class="btn btn-secondary" href="/details/6b50325d4cf88226d1398f55d923ba6f/new">詳細登録</a>
</div>
</div>
<div class='clearfix'></div>

<hr>
<div class='row item-box'>
<div class='col-xs-12 col-sm-12 col-md-4 col-xl-4 text-xs-center image-box'>
<img loading="lazy" src="/thumbnail/a/3/f/1/a3f11346972a7e652243399a7993cc16.jpg" alt="A3f11346972a7e652243399a7993cc16" width="160" height="200" />
</div>
<div class='col-xs-12 col-sm-12 col-md-8 col-xl-8 info-box'>
<div class='hash'>a3f11346972a7e652243399a7993cc16</div>
<small class='text-muted'>480x600 PNG 755.9KB</small>
<div class='pull-xs-right'></div>
<div class='detail-box gray-link'>
<h6>
<img class="to-link-icon" src="/assets/pixiv-628a47348a82153ebc34acba4e5b287777a11631bb382dbb00fd4b88083bed95.ico" alt="Pixiv" width="14" height="14" />
<a target="_blank" rel="noopener" href="https://www.pixiv.net/artworks/49180388">小説表紙　フリー素材</a>
<a target="_blank" rel="noopener" href="https://www.pixiv.net/users/6781491">kenpoo</a>
<small>
pixiv
</small>
</h6>

</div>
</div>
<div class='detail-link pull-xs-right hidden-sm-down gray-link'>
<span><a href="/search/color/a3f11346972a7e652243399a7993cc16">色合検索</a></span>
<span><a href="/search/bovw/a3f11346972a7e652243399a7993cc16">特徴検索</a></span>
<span><a href="/details/a3f11346972a7e652243399a7993cc16/new">詳細登録</a></span>
</div>
<div class='btn-block text-xs-center hidden-md-up p-d-1'>
<a class="btn btn-secondary" href="/search/color/a3f11346972a7e652243399a7993cc16">色合検索</a>
<a class="btn btn-secondary" href="/search/bovw/a3f11346972a7e652243399a7993cc16">特徴検索</a>
<a class="btn btn-secondary" href="/details/a3f11346972a7e652243399a7993cc16/new">詳細登録</a>
</div>
</div>
<div class='clearfix'></div>

<hr>
<div class='row item-box'>
<div class='col-xs-12 col-sm-12 col-md-4 col-xl-4 text-xs-center image-box'>
<img loading="lazy" src="/thumbnail/0/0/f/8/00f860f3e77cbbc2e4099be58ffa547c.jpg" alt="00f860f3e77cbbc2e4099be58ffa547c" width="149" height="200" />
</div>
<div class='col-xs-12 col-sm-12 col-md-8 col-xl-8 info-box'>
<div class='hash'>00f860f3e77cbbc2e4099be58ffa547c</div>
<small class='text-muted'>1113x1500 JPEG 777.9KB</small>
<div class='pull-xs-right'></div>
<div class='detail-box gray-link'>
<h6>
<img class="to-link-icon" src="/assets/pixiv-628a47348a82153ebc34acba4e5b287777a11631bb382dbb00fd4b88083bed95.ico" alt="Pixiv" width="14" height="14" />
<a target="_blank" rel="noopener" href="https://www.pixiv.net/artworks/59938316">ステクラろぐ11</a>
<a target="_blank" rel="noopener" href="https://www.pixiv.net/users/1406403">麦畑ケンコ■</a>
<small>
pixiv
</small>
</h6>

</div>
</div>
<div class='detail-link pull-xs-right hidden-sm-down gray-link'>
<span><a href="/search/color/00f860f3e77cbbc2e4099be58ffa547c">色合検索</a></span>
<span><a href="/search/bovw/00f860f3e77cbbc2e4099be58ffa547c">特徴検索</a></span>
<span><a href="/details/00f860f3e77cbbc2e4099be58ffa547c/new">詳細登録</a></span>
</div>
<div class='btn-block text-xs-center hidden-md-up p-d-1'>
<a class="btn btn-secondary" href="/search/color/00f860f3e77cbbc2e4099be58ffa547c">色合検索</a>
<a class="btn btn-secondary" href="/search/bovw/00f860f3e77cbbc2e4099be58ffa547c">特徴検索</a>
<a class="btn btn-secondary" href="/details/00f860f3e77cbbc2e4099be58ffa547c/new">詳細登録</a>
</div>
</div>
<div class='clearfix'></div>

<hr>
<div class='row item-box'>
<div class='col-xs-12 col-sm-12 col-md-4 col-xl-4 text-xs-center image-box'>
<img loading="lazy" src="/thumbnail/7/f/9/6/7f965eeb5994281b4ceb37f77bf3c2f7.jpg" alt="7f965eeb5994281b4ceb37f77bf3c2f7" width="128" height="200" />
</div>
<div class='col-xs-12 col-sm-12 col-md-8 col-xl-8 info-box'>
<div class='hash'>7f965eeb5994281b4ceb37f77bf3c2f7</div>
<small class='text-muted'>653x1024 JPEG 264.6KB</small>
<div class='pull-xs-right'></div>
<div class='detail-box gray-link'>
<h6>
<img class="to-link-icon" src="/assets/pixiv-628a47348a82153ebc34acba4e5b287777a11631bb382dbb00fd4b88083bed95.ico" alt="Pixiv" width="14" height="14" />
<a target="_blank" rel="noopener" href="https://www.pixiv.net/artworks/84809796">監♂受けまとめ</a>
<a target="_blank" rel="noopener" href="https://www.pixiv.net/users/52119695">🍓</a>
<small>
pixiv
</small>
</h6>

</div>
</div>
<div class='detail-link pull-xs-right hidden-sm-down gray-link'>
<span><a href="/search/color/7f965eeb5994281b4ceb37f77bf3c2f7">色合検索</a></span>
<span><a href="/search/bovw/7f965eeb5994281b4ceb37f77bf3c2f7">特徴検索</a></span>
<span><a href="/details/7f965eeb5994281b4ceb37f77bf3c2f7/new">詳細登録</a></span>
</div>
<div class='btn-block text-xs-center hidden-md-up p-d-1'>
<a class="btn btn-secondary" href="/search/color/7f965eeb5994281b4ceb37f77bf3c2f7">色合検索</a>
<a class="btn btn-secondary" href="/search/bovw/7f965eeb5994281b4ceb37f77bf3c2f7">特徴検索</a>
<a class="btn btn-secondary" href="/details/7f965eeb5994281b4ceb37f77bf3c2f7/new">詳細登録</a>
</div>
</div>
<div class='clearfix'></div>

<hr>
<div class='row item-box'>
<div class='col-xs-12 col-sm-12 col-md-4 col-xl-4 text-xs-center image-box'>
<img loading="lazy" src="/thumbnail/9/5/6/6/9566e0a8b1300ac25094f28237efb8e6.jpg" alt="9566e0a8b1300ac25094f28237efb8e6" width="113" height="200" />
</div>
<div class='col-xs-12 col-sm-12 col-md-8 col-xl-8 info-box'>
<div class='hash'>9566e0a8b1300ac25094f28237efb8e6</div>
<small class='text-muted'>600x1066 JPEG 75.0KB</small>
<div class='pull-xs-right'></div>
<div class='detail-box gray-link'>
<h6>
<img class="to-link-icon" src="/assets/pixiv-628a47348a82153ebc34acba4e5b287777a11631bb382dbb00fd4b88083bed95.ico" alt="Pixiv" width="14" height="14" />
<a target="_blank" rel="noopener" href="https://www.pixiv.net/artworks/53770484">チョロ受詰2｛無駄に多い、ほぼアナログ｝</a>
<a target="_blank" rel="noopener" href="https://www.pixiv.net/users/11098890">御手洗はついったに生息中</a>
<small>
pixiv
</small>
</h6>

</div>
</div>
<div class='detail-link pull-xs-right hidden-sm-down gray-link'>
<span><a href="/search/color/9566e0a8b1300ac25094f28237efb8e6">色合検索</a></span>
<span><a href="/search/bovw/9566e0a8b1300ac25094f28237efb8e6">特徴検索</a></span>
<span><a href="/details/9566e0a8b1300ac25094f28237efb8e6/new">詳細登録</a></span>
</div>
<div class='btn-block text-xs-center hidden-md-up p-d-1'>
<a class="btn btn-secondary" href="/search/color/9566e0a8b1300ac25094f28237efb8e6">色合検索</a>
<a class="btn btn-secondary" href="/search/bovw/9566e0a8b1300ac25094f28237efb8e6">特徴検索</a>
<a class="btn btn-secondary" href="/details/9566e0a8b1300ac25094f28237efb8e6/new">詳細登録</a>
</div>
</div>
<div class='clearfix'></div>

<hr>
<div class='row item-box'>
<div class='col-xs-12 col-sm-12 col-md-4 col-xl-4 text-xs-center image-box'>
<img loading="lazy" src="/thumbnail/f/0/1/b/f01b06c033510a0fb4dabbe891fe5d65.jpg" alt="F01b06c033510a0fb4dabbe891fe5d65" width="128" height="200" />
</div>
<div class='col-xs-12 col-sm-12 col-md-8 col-xl-8 info-box'>
<div class='hash'>f01b06c033510a0fb4dabbe891fe5d65</div>
<small class='text-muted'>1134x1783 JPEG 514.5KB</small>
<div class='pull-xs-right'></div>
<div class='detail-box gray-link'>
<h6>
<img class="to-link-icon" src="/assets/pixiv-628a47348a82153ebc34acba4e5b287777a11631bb382dbb00fd4b88083bed95.ico" alt="Pixiv" width="14" height="14" />
<a target="_blank" rel="noopener" href="https://www.pixiv.net/artworks/81374109">ついったーのまとめ。</a>
<a target="_blank" rel="noopener" href="https://www.pixiv.net/users/46418315">毬藻</a>
<small>
pixiv
</small>
</h6>

</div>
</div>
<div class='detail-link pull-xs-right hidden-sm-down gray-link'>
<span><a href="/search/color/f01b06c033510a0fb4dabbe891fe5d65">色合検索</a></span>
<span><a href="/search/bovw/f01b06c033510a0fb4dabbe891fe5d65">特徴検索</a></span>
<span><a href="/details/f01b06c033510a0fb4dabbe891fe5d65/new">詳細登録</a></span>
</div>
<div class='btn-block text-xs-center hidden-md-up p-d-1'>
<a class="btn btn-secondary" href="/search/color/f01b06c033510a0fb4dabbe891fe5d65">色合検索</a>
<a class="btn btn-secondary" href="/search/bovw/f01b06c033510a0fb4dabbe891fe5d65">特徴検索</a>
<a class="btn btn-secondary" href="/details/f01b06c033510a0fb4dabbe891fe5d65/new">詳細登録</a>
</div>
</div>
<div class='clearfix'></div>

<hr>
<div class='row item-box'>
<div class='col-xs-12 col-sm-12 col-md-4 col-xl-4 text-xs-center image-box'>
<img loading="lazy" src="/thumbnail/d/a/0/e/da0e789514d7f3532d2560c7a72a356b.jpg" alt="Da0e789514d7f3532d2560c7a72a356b" width="129" height="200" />
</div>
<div class='col-xs-12 col-sm-12 col-md-8 col-xl-8 info-box'>
<div class='hash'>da0e789514d7f3532d2560c7a72a356b</div>
<small class='text-muted'>771x1200 JPEG 457.2KB</small>
<div class='pull-xs-right'></div>
<div class='detail-box gray-link'>
<h6>
<img class="to-link-icon" src="/assets/pixiv-628a47348a82153ebc34acba4e5b287777a11631bb382dbb00fd4b88083bed95.ico" alt="Pixiv" width="14" height="14" />
<a target="_blank" rel="noopener" href="https://www.pixiv.net/artworks/46924470">みにきゃら。</a>
<a target="_blank" rel="noopener" href="https://www.pixiv.net/users/8163927">まい</a>
<small>
pixiv
</small>
</h6>

</div>
</div>
<div class='detail-link pull-xs-right hidden-sm-down gray-link'>
<span><a href="/search/color/da0e789514d7f3532d2560c7a72a356b">色合検索</a></span>
<span><a href="/search/bovw/da0e789514d7f3532d2560c7a72a356b">特徴検索</a></span>
<span><a href="/details/da0e789514d7f3532d2560c7a72a356b/new">詳細登録</a></span>
</div>
<div class='btn-block text-xs-center hidden-md-up p-d-1'>
<a class="btn btn-secondary" href="/search/color/da0e789514d7f3532d2560c7a72a356b">色合検索</a>
<a class="btn btn-secondary" href="/search/bovw/da0e789514d7f3532d2560c7a72a356b">特徴検索</a>
<a class="btn btn-secondary" href="/details/da0e789514d7f3532d2560c7a72a356b/new">詳細登録</a>
</div>
</div>
<div class='clearfix'></div>

<hr>
<div class='row item-box'>
<div class='col-xs-12 col-sm-12 col-md-4 col-xl-4 text-xs-center image-box'>
<img loading="lazy" src="/thumbnail/1/0/c/6/10c6a1cb8be75ea7f89f2764be8a0ef5.jpg" alt="10c6a1cb8be75ea7f89f2764be8a0ef5" width="150" height="200" />
</div>
<div class='col-xs-12 col-sm-12 col-md-8 col-xl-8 info-box'>
<div class='hash'>10c6a1cb8be75ea7f89f2764be8a0ef5</div>
<small class='text-muted'>1536x2048 JPEG 915.7KB</small>
<div class='pull-xs-right'></div>
<div class='detail-box gray-link'>
<h6>
<img class="to-link-icon" src="/assets/pixiv-628a47348a82153ebc34acba4e5b287777a11631bb382dbb00fd4b88083bed95.ico" alt="Pixiv" width="14" height="14" />
<a target="_blank" rel="noopener" href="https://www.pixiv.net/artworks/77970054">Twitter落書き詰め詰め(3)</a>
<a target="_blank" rel="noopener" href="https://www.pixiv.net/users/37712046">あめ?</a>
<small>
pixiv
</small>
</h6>

</div>
</div>
<div class='detail-link pull-xs-right hidden-sm-down gray-link'>
<span><a href="/search/color/10c6a1cb8be75ea7f89f2764be8a0ef5">色合検索</a></span>
<span><a href="/search/bovw/10c6a1cb8be75ea7f89f2764be8a0ef5">特徴検索</a></span>
<span><a href="/details/10c6a1cb8be75ea7f89f2764be8a0ef5/new">詳細登録</a></span>
</div>
<div class='btn-block text-xs-center hidden-md-up p-d-1'>
<a class="btn btn-secondary" href="/search/color/10c6a1cb8be75ea7f89f2764be8a0ef5">色合検索</a>
<a class="btn btn-secondary" href="/search/bovw/10c6a1cb8be75ea7f89f2764be8a0ef5">特徴検索</a>
<a class="btn btn-secondary" href="/details/10c6a1cb8be75ea7f89f2764be8a0ef5/new">詳細登録</a>
</div>
</div>
<div class='clearfix'></div>

<hr>
<div class='row item-box'>
<div class='col-xs-12 col-sm-12 col-md-4 col-xl-4 text-xs-center image-box'>
<img loading="lazy" src="/thumbnail/f/3/4/4/f344bd779a71d70937660393ceb7ae9d.jpg" alt="F344bd779a71d70937660393ceb7ae9d" width="158" height="200" />
</div>
<div class='col-xs-12 col-sm-12 col-md-8 col-xl-8 info-box'>
<div class='hash'>f344bd779a71d70937660393ceb7ae9d</div>
<small class='text-muted'>1053x1333 JPEG 1024.2KB</small>
<div class='pull-xs-right'></div>
<div class='detail-box gray-link'>
<h6>
<img class="to-link-icon" src="/assets/pixiv-628a47348a82153ebc34acba4e5b287777a11631bb382dbb00fd4b88083bed95.ico" alt="Pixiv" width="14" height="14" />
<a target="_blank" rel="noopener" href="https://www.pixiv.net/artworks/54185268">※キャプション必読※クソ漫画もどきとおそ一さん２</a>
<a target="_blank" rel="noopener" href="https://www.pixiv.net/users/13786822">もももも</a>
<small>
pixiv
</small>
</h6>

</div>
</div>
<div class='detail-link pull-xs-right hidden-sm-down gray-link'>
<span><a href="/search/color/f344bd779a71d70937660393ceb7ae9d">色合検索</a></span>
<span><a href="/search/bovw/f344bd779a71d70937660393ceb7ae9d">特徴検索</a></span>
<span><a href="/details/f344bd779a71d70937660393ceb7ae9d/new">詳細登録</a></span>
</div>
<div class='btn-block text-xs-center hidden-md-up p-d-1'>
<a class="btn btn-secondary" href="/search/color/f344bd779a71d70937660393ceb7ae9d">色合検索</a>
<a class="btn btn-secondary" href="/search/bovw/f344bd779a71d70937660393ceb7ae9d">特徴検索</a>
<a class="btn btn-secondary" href="/details/f344bd779a71d70937660393ceb7ae9d/new">詳細登録</a>
</div>
</div>
<div class='clearfix'></div>

<hr>
<div class='row item-box'>
<div class='col-xs-12 col-sm-12 col-md-4 col-xl-4 text-xs-center image-box'>
<img loading="lazy" src="/thumbnail/2/a/d/8/2ad8e200c37bcead93a5040c90cb7a4b.jpg" alt="2ad8e200c37bcead93a5040c90cb7a4b" width="150" height="200" />
</div>
<div class='col-xs-12 col-sm-12 col-md-8 col-xl-8 info-box'>
<div class='hash'>2ad8e200c37bcead93a5040c90cb7a4b</div>
<small class='text-muted'>750x1000 JPEG 150.0KB</small>
<div class='pull-xs-right'></div>
<div class='detail-box gray-link'>
<h6>
<img class="to-link-icon" src="/assets/pixiv-628a47348a82153ebc34acba4e5b287777a11631bb382dbb00fd4b88083bed95.ico" alt="Pixiv" width="14" height="14" />
<a target="_blank" rel="noopener" href="https://www.pixiv.net/artworks/64918812">テニス3</a>
<a target="_blank" rel="noopener" href="https://www.pixiv.net/users/3601569">nabo</a>
<small>
pixiv
</small>
</h6>

</div>
</div>
<div class='detail-link pull-xs-right hidden-sm-down gray-link'>
<span><a href="/search/color/2ad8e200c37bcead93a5040c90cb7a4b">色合検索</a></span>
<span><a href="/search/bovw/2ad8e200c37bcead93a5040c90cb7a4b">特徴検索</a></span>
<span><a href="/details/2ad8e200c37bcead93a5040c90cb7a4b/new">詳細登録</a></span>
</div>
<div class='btn-block text-xs-center hidden-md-up p-d-1'>
<a class="btn btn-secondary" href="/search/color/2ad8e200c37bcead93a5040c90cb7a4b">色合検索</a>
<a class="btn btn-secondary" href="/search/bovw/2ad8e200c37bcead93a5040c90cb7a4b">特徴検索</a>
<a class="btn btn-secondary" href="/details/2ad8e200c37bcead93a5040c90cb7a4b/new">詳細登録</a>
</div>
</div>
<div class='clearfix'></div>

<hr>
<div class='row item-box'>
<div class='col-xs-12 col-sm-12 col-md-4 col-xl-4 text-xs-center image-box'>
<img loading="lazy" src="/thumbnail/0/1/2/a/012ac47e0c697d6550cbdf8544a6e8a7.jpg" alt="012ac47e0c697d6550cbdf8544a6e8a7" width="154" height="200" />
</div>
<div class='col-xs-12 col-sm-12 col-md-8 col-xl-8 info-box'>
<div class='hash'>012ac47e0c697d6550cbdf8544a6e8a7</div>
<small class='text-muted'>832x1086 JPEG 306.3KB</small>
<div class='pull-xs-right'></div>
<div class='detail-box gray-link'>
<h6>
<img class="to-link-icon" src="/assets/pixiv-628a47348a82153ebc34acba4e5b287777a11631bb382dbb00fd4b88083bed95.ico" alt="Pixiv" width="14" height="14" />
<a target="_blank" rel="noopener" href="https://www.pixiv.net/artworks/76239320">ごちゃ混ぜログ(sktさんキャラ崩壊)</a>
<a target="_blank" rel="noopener" href="https://www.pixiv.net/users/23586968">Azma</a>
<small>
pixiv
</small>
</h6>

</div>
</div>
<div class='detail-link pull-xs-right hidden-sm-down gray-link'>
<span><a href="/search/color/012ac47e0c697d6550cbdf8544a6e8a7">色合検索</a></span>
<span><a href="/search/bovw/012ac47e0c697d6550cbdf8544a6e8a7">特徴検索</a></span>
<span><a href="/details/012ac47e0c697d6550cbdf8544a6e8a7/new">詳細登録</a></span>
</div>
<div class='btn-block text-xs-center hidden-md-up p-d-1'>
<a class="btn btn-secondary" href="/search/color/012ac47e0c697d6550cbdf8544a6e8a7">色合検索</a>
<a class="btn btn-secondary" href="/search/bovw/012ac47e0c697d6550cbdf8544a6e8a7">特徴検索</a>
<a class="btn btn-secondary" href="/details/012ac47e0c697d6550cbdf8544a6e8a7/new">詳細登録</a>
</div>
</div>
<div class='clearfix'></div>

<hr>
<div class='row item-box'>
<div class='col-xs-12 col-sm-12 col-md-4 col-xl-4 text-xs-center image-box'>
<img loading="lazy" src="/thumbnail/8/9/c/5/89c5cade051de2f79e5bd74774afd10f.jpg" alt="89c5cade051de2f79e5bd74774afd10f" width="158" height="200" />
</div>
<div class='col-xs-12 col-sm-12 col-md-8 col-xl-8 info-box'>
<div class='hash'>89c5cade051de2f79e5bd74774afd10f</div>
<small class='text-muted'>1612x2046 JPEG 950.5KB</small>
<div class='pull-xs-right'></div>
<div class='detail-box gray-link'>
<h6>
<img class="to-link-icon" src="/assets/pixiv-628a47348a82153ebc34acba4e5b287777a11631bb382dbb00fd4b88083bed95.ico" alt="Pixiv" width="14" height="14" />
<a target="_blank" rel="noopener" href="https://www.pixiv.net/artworks/44032791">ツイッターやつ</a>
<a target="_blank" rel="noopener" href="https://www.pixiv.net/users/5066651">ブロッコリー</a>
<small>
pixiv
</small>
</h6>

</div>
</div>
<div class='detail-link pull-xs-right hidden-sm-down gray-link'>
<span><a href="/search/color/89c5cade051de2f79e5bd74774afd10f">色合検索</a></span>
<span><a href="/search/bovw/89c5cade051de2f79e5bd74774afd10f">特徴検索</a></span>
<span><a href="/details/89c5cade051de2f79e5bd74774afd10f/new">詳細登録</a></span>
</div>
<div class='btn-block text-xs-center hidden-md-up p-d-1'>
<a class="btn btn-secondary" href="/search/color/89c5cade051de2f79e5bd74774afd10f">色合検索</a>
<a class="btn btn-secondary" href="/search/bovw/89c5cade051de2f79e5bd74774afd10f">特徴検索</a>
<a class="btn btn-secondary" href="/details/89c5cade051de2f79e5bd74774afd10f/new">詳細登録</a>
</div>
</div>
<div class='clearfix'></div>

<hr>
<div class='row item-box'>
<div class='col-xs-12 col-sm-12 col-md-4 col-xl-4 text-xs-center image-box'>
<img loading="lazy" src="/thumbnail/5/3/b/a/53ba1179d866bc21ee8079fcf7979386.jpg" alt="53ba1179d866bc21ee8079fcf7979386" width="150" height="200" />
</div>
<div class='col-xs-12 col-sm-12 col-md-8 col-xl-8 info-box'>
<div class='hash'>53ba1179d866bc21ee8079fcf7979386</div>
<small class='text-muted'>720x960 JPEG 498.0KB</small>
<div class='pull-xs-right'></div>
<div class='detail-box gray-link'>
<h6>
<img class="to-link-icon" src="/assets/pixiv-628a47348a82153ebc34acba4e5b287777a11631bb382dbb00fd4b88083bed95.ico" alt="Pixiv" width="14" height="14" />
<a target="_blank" rel="noopener" href="https://www.pixiv.net/artworks/57187431">今日の落書き</a>
<a target="_blank" rel="noopener" href="https://www.pixiv.net/users/3756254">TKG</a>
<small>
pixiv
</small>
</h6>

</div>
</div>
<div class='detail-link pull-xs-right hidden-sm-down gray-link'>
<span><a href="/search/color/53ba1179d866bc21ee8079fcf7979386">色合検索</a></span>
<span><a href="/search/bovw/53ba1179d866bc21ee8079fcf7979386">特徴検索</a></span>
<span><a href="/details/53ba1179d866bc21ee8079fcf7979386/new">詳細登録</a></span>
</div>
<div class='btn-block text-xs-center hidden-md-up p-d-1'>
<a class="btn btn-secondary" href="/search/color/53ba1179d866bc21ee8079fcf7979386">色合検索</a>
<a class="btn btn-secondary" href="/search/bovw/53ba1179d866bc21ee8079fcf7979386">特徴検索</a>
<a class="btn btn-secondary" href="/details/53ba1179d866bc21ee8079fcf7979386/new">詳細登録</a>
</div>
</div>
<div class='clearfix'></div>


</div>
<div class='hidden-md-down col-lg-4 col-xl-4'>
<div class='message'>
<h5 class='p-t-1 text-xs-center'>お知らせ</h5>
<hr>
<div class='p-l-1 gray-link'>
<h6>エラー</h6>
<p>が出てる人は <a href="https://jbbs.shitaraba.net/computer/42759/">掲示板</a> などで教えてもらえると助かります</p>
<p class='small timestamp text-muted text-xs-right'>2022/01/06 12:43</p>
</div>
<div class='p-l-1 gray-link'>
<h6>WEBP</h6>
<p>一応対応済み</p>
<p class='small timestamp text-muted text-xs-right'>2020/07/17 23:05</p>
</div>
<div class='p-l-1 gray-link'>
<h6>実験的に</h6>
<p>https://ascii2d.net/search/url/画像のエンコード済みURL?type=color<br />で検索できるようにしました</p>
<p class='small timestamp text-muted text-xs-right'>2018/07/19 14:17</p>
</div>
</div>
<div class='com p-t-1'>
<h5 class='text-xs-center'>広告</h5>
<hr>
<div class='text-xs-center'>
<div class='p-l-l gray-link'>
<h6>指揮官を癒やし隊！・アドミラル・ヒッパーとの温泉リョジョウ</h6>
<a href='https://www.dlsite.com/home/dlaf/=/t/s/link/work/aid/conoco01/id/RJ391672.html' target='_blank'>
<img src="/images/RJ391672.webp" alt="Rj391672" width="250" height="191" />
</a>
</div>
<hr>
</div>
</div>
<div class='com p-t-1'>
<h5 class='text-xs-center'>寄付</h5>
<hr>
<div class='banner text-xs-center'>
<p><a target="_blank" rel="noopener" href="https://www.amazon.co.jp/dp/B004N3APGO/"><img src="/assets/ag2-4659ef7742d7cde56a7abc27e1f214a53e8b0dd029e5d191b3ade91aa57646b0.jpg" alt="Ag2" width="234" height="60" /></a></p>
</div>
<div class='message'>
<p class='text-muted text-xs-center'><a href="/cdn-cgi/l/email-protection" class="__cf_email__" data-cfemail="a9decccbc4c8daddccdbe9c8dacac0c09bcd87c7ccdd">[email&#160;protected]</a> まで</p>
</div>
</div>
<div class='com p-t-1'>
<h5 class='text-xs-center'>広告</h5>
<hr>
<div class='amazon text-xs-center p-t-1'>
<a href='http://www.amazon.co.jp/exec/obidos/redirect-home?tag=conoco-22' rel='noopener' target='_blank'>
<img src="/assets/amazon_logo-b4c601bb2dc525d6863da15102758d83e7e8fa5532919ed0e1b8f6befc9317b7.gif" alt="Amazon logo" width="116" height="32" />
</a>
</div>
<div class='banner text-xs-center p-t-1'>
<p><a target="_blank" rel="noopener" href="https://www.dlsite.com/maniax/dlaf/=/link/profile/aid/conoco01/maker.html"><img src="/assets/dlbn2n-552118e686a28a7f0af67bd1bb1e66f680d14109e323143a47fcfb575828b5a3.gif" alt="Dlbn2n" width="200" height="40" /></a></p>
<p><a target="_blank" rel="noopener" href="https://al.dmm.co.jp/?lurl=https%3A%2F%2Fwww.dmm.co.jp%2Fdc%2Fdoujin%2F&amp;af_id=conoco-002&amp;ch=toolbar&amp;ch_id=text"><img src="/assets/dmm-e17421cc68ea0c6cf85d47e84d1f90a2257e7191d62e00c9378067c3dff06f5d.jpg" alt="Dmm" width="200" height="51" /></a></p>
<p><a target="_blank" rel="noopener" href="https://image.getchu.com/api/geturl.phtml/af/132/aftype/2/sid/204/bid/85/url/top.html-/"><img src="/assets/getchu-6aba1a80510ac0d53e6c77f73656d1d0f5a63d6e4d77da3f4e6ddcde58f563eb.jpg" alt="Getchu" width="199" height="51" /></a></p>
</div>
</div>
<div class='link p-t-1'>
<h5 class='text-xs-center'>LINK</h5>
<hr>
</div>

</div>
<div class='clearfix'></div>
<hr>
<footer class='col-xs-12 col-lg-9 col-xl-9 p-t-1 p-b-1 m-b-1'>
<h6 class='small pull-xs-left gray-link'>
<a href='/'>二次元画像詳細検索</a>
</h6>
</footer>
</div>
</div>
</html>"#;
        let result = parse_result("__TEST_URL__", raw_html);
        assert_eq!(result.is_ok(), true);
        assert_eq!(result.unwrap(), Some(SourceImage {
            url: "https://twitter.com/tororo51791023/status/1521412000689324032".to_string(),
            metadata: HashMap::from([("作者".to_string(), "tororo51791023".to_string())]),
        }));
    }
}
