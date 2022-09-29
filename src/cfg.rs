use lazy_static::lazy_static;
use serde::Deserialize;

lazy_static! {
    pub static ref BOT_CONFIG: BotConfig = config::Config::builder()
        .add_source(config::File::with_name("config"))
        .add_source(config::Environment::with_prefix("APP"))
        .build()
        .expect("failed to load config")
        .try_deserialize()
        .expect("failed to parse config");
}

#[derive(Debug, Deserialize)]
pub struct BotConfig {
    pub ws_url: String,
    pub twitter_videos_path: String,
    pub proxy_url: Option<String>,
    pub saucenao_api_key: String,
}
