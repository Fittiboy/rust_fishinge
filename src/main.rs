use eventsub_websocket::types::TwitchMessage;
use eventsub_websocket::{event_handler, get_session, parse_message};
use std::sync::mpsc;
use std::thread;

fn main() -> Result<(), String> {
    let (tx, rx) = mpsc::channel();
    let mut session = get_session()?;
    let _ = thread::Builder::new()
        .name("handler".into())
        .spawn(move || -> Result<(), String> {
            event_handler(&mut session, tx)?;
            Ok(())
        });
    loop {
        let msg = rx.recv().map_err(|err| format!("{}", err))?;
        let msg: TwitchMessage = parse_message(&msg).map_err(|err| format!("{}", err))?;
        println!("Handling message locally: {:#?}", msg);
    }
}
