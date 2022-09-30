mod ascii2d;
mod cfg;
mod client;
mod database;
mod download;
mod iqdb;
mod message;
mod saucenao;
mod searcher;
mod utils;

use crate::cfg::*;
use crate::database::*;
use crate::message::*;
use futures_util::{SinkExt, StreamExt};
use log::{debug, error, info, warn};
use std::sync::Arc;
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::mpsc;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let (streams, _) = tokio_tungstenite::connect_async(&BOT_CONFIG.ws_url)
        .await
        .expect("failed to connect to websocket server");
    info!("connected to the server");

    let (mut write, read) = streams.split();
    let (tx, mut rx) = mpsc::channel::<BotResponseAction>(128);

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

            debug!("{}", data.as_str());
            let message: OneBotMessageWrapper =
                serde_json::from_str(data.as_str()).expect("malformed json");

            let messages_to_send: Vec<BotResponseAction> = match message {
                OneBotMessageWrapper::Message(OneBotMessage::Message(message)) => match message {
                    OneBotUserMessage::Group(message) => [
                        searcher::on_group_message(message.clone()).await,
                        download::on_group_message(message.clone()).await,
                    ]
                    .into_iter()
                    .flatten()
                    .collect(),
                    OneBotUserMessage::Private(message) => {
                        [download::on_private_message(message).await]
                            .into_iter()
                            .flatten()
                            .collect()
                    }
                },
                _ => vec![],
            };

            for message in messages_to_send {
                if let Err(err) = tx.send(message).await {
                    error!("failed to send message: {:#?}", err);
                }
            }
        })
        .await;
    });

    let (mut int_signal, mut term_signal) = (
        signal(SignalKind::interrupt()).unwrap(),
        signal(SignalKind::terminate()).unwrap(),
    );
    tokio::select! {
        _ = int_signal.recv() => {},
        _ = term_signal.recv() => {},
    }
    warn!("signal received, shutting down");
    DATABASE.flush().expect("failed to flush database");
}
