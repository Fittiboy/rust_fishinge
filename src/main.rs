#![allow(clippy::expect_used, clippy::unwrap_used)]

use anyhow::Result;
use eventsub_websocket::types::TwitchMessage;
use eventsub_websocket::{event_handler, CloseCode, CloseFrame};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

use fishinge::{create_subscription, get_ids, is_subscribed, write_output, Config};
use fishinge::{if_err_writer, let_match_writer, write_expect};

struct FishingeSetup {
    config: Config,
}

impl eframe::App for FishingeSetup {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Client ID");
            ui.text_edit_singleline(&mut self.config.client_id);
            ui.heading("Streamer");
            ui.text_edit_singleline(&mut self.config.streamer);
            ui.heading("User Access Token");
            ui.text_edit_singleline(&mut self.config.user_access_token);
            ui.heading("Reward Title");
            ui.text_edit_singleline(&mut self.config.reward_title);
            ui.heading("JWT");
            ui.text_edit_singleline(&mut self.config.jwt);
            ui.heading("Command Name");
            ui.text_edit_singleline(&mut self.config.command_name);
            if ui.button("Launch").clicked() {
                let config = &self.config;
                config.write().unwrap();
                frame.close();
            }
        });
    }
}

struct FishingeOutput {
    output: Arc<Mutex<String>>,
}

impl eframe::App for FishingeOutput {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        ctx.request_repaint();
        egui::CentralPanel::default().show(ctx, |ui| -> Result<()> {
            egui::ScrollArea::vertical().show(ui, |ui| {
                let text = self.output.lock().unwrap().clone();
                ui.horizontal_wrapped(|ui| {
                    ui.monospace(text);
                });
            });
            ui.with_layout(egui::Layout::left_to_right(egui::Align::BOTTOM), |ui| {
                if ui.button("Quit").clicked() {
                    frame.close();
                    std::process::exit(0);
                }
            });
            Ok(())
        });
    }
}

fn main() -> Result<()> {
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(320., 320.)),
        resizable: true,
        fullscreen: false,
        maximized: false,
        ..Default::default()
    };

    let options_clone = options.clone();

    let config = match Config::load() {
        Ok(config) => config,
        Err(_) => Config::empty(),
    };

    eframe::run_native(
        "Fishinge Setup",
        options_clone,
        Box::new(|_cc| Box::new(FishingeSetup { config })),
    );

    let (tx, rx) = mpsc::channel();
    let (fish_tx, fish_rx) = mpsc::channel();

    let output = Arc::new(Mutex::new(String::new()));
    let output_write1 = Arc::clone(&output);
    let output_write2 = Arc::clone(&output);
    let output_write3 = Arc::clone(&output);
    let output_read = Arc::clone(&output);

    let_match_writer!(event_res, event_handler(None, tx.clone()), output_write1);

    let config = Config::load()
        .expect("config has to exist at this point, unless some system operation failed");
    let config2 = config.clone();

    let notification_handle = thread::Builder::new().name("notifications".into()).spawn(
        move || -> Result<(), anyhow::Error> {
            if_err_writer!(config.test(), output_write2, drop(fish_rx));
            if_err_writer!(fish_rx.recv(), output_write2,);
            loop {
                if_err_writer!(fish_rx.recv(), output_write2,);
                if_err_writer!(handle_notification(&output_write2, &config), output_write2,);
            }
        },
    );

    let listener_handle = thread::Builder::new().name("listener".into()).spawn(
        move || -> Result<(), anyhow::Error> {
            let mut welcome_count = 0;
            if_err_writer!(fish_tx.send("Healthy!"), output_write3,);
            loop {
                let_match_writer!(msg, rx.recv(), output_write3);
                match msg {
                    TwitchMessage::Notification(_) => {
                        if_err_writer!(fish_tx.send("Ping!"), output_write3,);
                    }
                    TwitchMessage::Welcome(msg) => {
                        welcome_count += 1;
                        if welcome_count == 1 {
                            write_expect!(&output_write3, "Connected to Twitch!");
                        } else {
                            write_expect!(&output_write3, "Reconnected to Twitch!");
                        }
                        let session_id = msg.payload.session.id.to_owned();
                        if !is_subscribed(&config2, session_id.clone())? {
                            if_err_writer!(
                                subscribe(&output_write3, session_id, &config2),
                                output_write3,
                            );
                            write_expect!(&output_write3, "Subscribed to redemption notifications!\nWaiting for redmeptions...");
                        }
                    }
                    _ => {}
                }
            }
        },
    );

    eframe::run_native(
        "Pond opener 3000™",
        options,
        Box::new(move |_cc| {
            Box::new(FishingeOutput {
                output: output_read,
            })
        }),
    );

    event_res
        .session
        .lock()
        .expect("session should not be poisoned")
        .socket
        .close(Some(CloseFrame {
            code: CloseCode::Normal,
            reason: "Client encountered error.".into(),
        }))?;
    notification_handle?;
    listener_handle?;
    Ok(())
}

fn handle_notification(output: &Arc<Mutex<String>>, config: &Config) -> Result<()> {
    fishinge::update_command(output, config)
}

fn subscribe(output: &Arc<Mutex<String>>, session_id: String, config: &Config) -> Result<()> {
    let_match_writer!((broadcaster_id, reward_id), get_ids(config), *output);
    write_expect!(
        output,
        &format!(
            "Found all required ids:\n Broadcaster:\n  {}\n Reward:\n  {}",
            broadcaster_id, reward_id
        )
    );
    create_subscription(config, session_id, broadcaster_id, reward_id)
}
