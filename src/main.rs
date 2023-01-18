use eventsub_websocket::types::TwitchMessage;
use eventsub_websocket::{event_handler, get_session};
use std::sync::mpsc;
use std::thread;

use fishinge::get_reward_id;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (tx, rx) = mpsc::channel();
    let mut session = get_session(None)?;
    let _ = thread::Builder::new()
        .name("handler".into())
        .spawn(move || -> Result<(), String> {
            event_handler(&mut session, tx)?;
            Ok(())
        });
    loop {
        let mut welcome_count = 0;
        let msg: TwitchMessage = rx.recv().map_err(|err| format!("{}", err))?;
        match msg {
            TwitchMessage::Notification(_) => handle_notification(),
            TwitchMessage::Welcome(_) => {
                welcome_count += 1;
                if welcome_count == 1 {
                    subscribe().await?;
                }
            }
            _ => {}
        }
    }
}

fn handle_notification() {
    println!("Doing the pond thing!");
}

async fn subscribe() -> Result<(), Box<dyn std::error::Error>> {
    let reward_id = get_reward_id().await?;
    println!("Got reward id: {}", reward_id);
    Ok(())
}
