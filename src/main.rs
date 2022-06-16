mod saucenao;
mod searcher;
mod ascii2d;
mod iqdb;
mod download;
mod utils;
mod database;
mod message;
mod cfg;

use std::sync::Arc;
use tokio::sync::Mutex;
use log::{debug, error, warn};
use tokio::signal::unix::{signal, SignalKind};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;

use crate::database::*;
use crate::message::*;
use crate::cfg::*;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let (streams, _) = tokio_tungstenite::connect_async(&BOT_CONFIG.ws_url).await.expect("failed to connect to websocket server");
    let (mut write, read) = streams.split();
    let (tx, mut rx) = mpsc::channel::<SendMessage>(128);

    tokio::spawn(async move {
        while let Some(ref message) = rx.recv().await {
            let params = match serde_json::to_string(message) {
                Ok(params) => params,
                Err(err) => {
                    error!("failed to parse SendMessage: {}", err);
                    continue;
                }
            };
            if let Err(err) = write.send(tungstenite::Message::Text(params)).await {
                error!("failed to send message to websocket server: {:#?}", err);
            }
        }
    });

    tokio::spawn(async move {
        let tx = Arc::new(Mutex::new(tx));

        read.for_each(|message| async {
            let tx = tx.clone().lock().await.clone();

            let data = message.expect("failed to read message").into_data();
            let data = String::from_utf8(data).expect("malformed message, not an utf-8 string");
            let data = data.replacen("\"message_type\":\"group\",", "", 1);

            debug!("{}", data.as_str());
            let message: OneBotMessageWrapper = serde_json::from_str(data.as_str()).expect("malformed json");

            let message_to_send: Option<SendMessage> = match message {
                OneBotMessageWrapper::Message(OneBotMessage::Message(message)) => match message {
                    OneBotUserMessage::Group(message) => searcher::on_group_message(message).await,
                    OneBotUserMessage::Private(message) => download::on_private_message(message).await,
                },
                _ => None,
            };

            if let Some(message) = message_to_send {
                if let Err(err) = tx.send(message).await {
                    error!("failed to send SendGroupMessage: {}", err);
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
    DATABASE.flush().expect("failed to flush database");
}