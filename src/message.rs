use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(tag = "post_type")]
pub enum OneBotMessage {
    #[serde(rename = "message")]
    Message(OneBotUserMessage),

    #[serde(rename = "meta_event")]
    Meta {
        meta_event_type: String
    },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "message_type")]
pub enum OneBotUserMessage {
    #[serde(rename = "private")]
    Private(OneBotPrivateMessage),

    #[serde(rename = "group")]
    Group(OneBotGroupMessage),
}

#[derive(Debug, Deserialize)]
pub struct OneBotPrivateMessage {
    pub message_id: i32,
    pub user_id: i64,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct OneBotGroupMessage {
    pub message_id: i32,
    pub group_id: i64,
    pub message: String,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum OneBotMessageWrapper {
    Message(OneBotMessage),
    Other(serde_json::Value),
}

#[derive(Debug, Serialize)]
#[serde(tag = "action", content = "params")]
pub enum SendMessage {
    #[serde(rename = "send_group_msg")]
    Group {
        group_id: i64,
        message: String,
    },
    #[serde(rename = "send_private_msg")]
    Private {
        user_id: i64,
        message: String,
    },
}
