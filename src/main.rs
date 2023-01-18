use eventsub_websocket::types::TwitchMessage;
use eventsub_websocket::{event_handler, get_session};
use std::sync::mpsc;
use std::thread;

use fishinge::{create_subscription, get_ids};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (tx, rx) = mpsc::channel();
    let (fish_tx, fish_rx) = async_channel::unbounded();
    let mut session = get_session(None)?;

    let _ = thread::Builder::new()
        .name("handler".into())
        .spawn(move || -> Result<(), String> {
            event_handler(&mut session, tx)?;
            Ok(())
        });

    let _ = tokio::spawn(async move {
        loop {
            fish_rx.recv().await.unwrap();
            handle_notification().await.unwrap();
        }
    });

    loop {
        let mut welcome_count = 0;
        let msg: TwitchMessage = rx.recv().map_err(|err| format!("{}", err))?;
        match msg {
            TwitchMessage::Notification(_) => fish_tx.send("Ping!").await?,
            TwitchMessage::Welcome(msg) => {
                welcome_count += 1;
                if welcome_count == 1 {
                    let session_id = msg.session_id().to_owned();
                    subscribe(session_id).await?;
                    println!("Subscribed!");
                }
            }
            _ => {}
        }
    }
}

async fn handle_notification() -> Result<(), Box<dyn std::error::Error>> {
    println!("Time for a little nap!");
    fishinge::update_command().await?;
    Ok(())
}

async fn subscribe(session_id: String) -> Result<(), Box<dyn std::error::Error>> {
    let (broadcaster_id, reward_id) = get_ids().await?;
    println!(
        "Got ids:\n\tBroadcaster: {},\n\tReward: {}",
        broadcaster_id, reward_id
    );
    create_subscription(session_id, broadcaster_id, reward_id).await?;
    Ok(())
}
