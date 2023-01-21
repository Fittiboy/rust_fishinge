use anyhow::Result;
use eventsub_websocket::types::TwitchMessage;
use eventsub_websocket::{event_handler, get_session};
use std::sync::mpsc;
use std::thread;

use fishinge::{create_subscription, get_ids, Config};

#[tokio::main]
async fn main() -> Result<()> {
    let config: Config = Config::load()?;
    let (tx, rx) = mpsc::channel();
    let (fish_tx, fish_rx) = async_channel::unbounded();
    let mut session = get_session(None)?;

    let _ = thread::Builder::new()
        .name("handler".into())
        .spawn(move || -> Result<()> {
            event_handler(&mut session, tx)?;
            Ok(())
        });

    let thread_config = config.clone();
    tokio::spawn(async move {
        loop {
            if (fish_rx.recv().await).is_ok() {
                let _ = handle_notification(&thread_config).await;
            }
        }
    });

    loop {
        let mut welcome_count = 0;
        let msg: TwitchMessage = rx.recv()?;
        match msg {
            TwitchMessage::Notification(_) => fish_tx.send("Ping!").await?,
            TwitchMessage::Welcome(msg) => {
                welcome_count += 1;
                if welcome_count == 1 {
                    let session_id = msg.session_id().to_owned();
                    subscribe(session_id, &config).await?;
                    println!("Subscribed!");
                }
            }
            _ => {}
        }
    }
}

async fn handle_notification(config: &Config) -> Result<()> {
    fishinge::update_command(config).await
}

async fn subscribe(session_id: String, config: &Config) -> Result<()> {
    let (broadcaster_id, reward_id) = get_ids(config).await?;
    println!(
        "Got ids:\n\tBroadcaster: {},\n\tReward: {}",
        broadcaster_id, reward_id
    );
    create_subscription(config, session_id, broadcaster_id, reward_id).await
}
