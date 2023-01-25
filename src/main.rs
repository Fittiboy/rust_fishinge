#![allow(clippy::expect_used, clippy::unwrap_used)]

use anyhow::Result;
use eventsub_websocket::types::TwitchMessage;
use eventsub_websocket::{event_handler, get_session};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

use fishinge::{create_subscription, get_ids, write_output, Config};

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

    let _ = thread::Builder::new()
        .name("handler".into())
        .spawn(move || -> Result<()> {
            let mut session = get_session(None)?;
            event_handler(&mut session, tx)?;
            Ok(())
        });

    let output = Arc::new(Mutex::new(String::new()));
    let output_write1 = Arc::clone(&output);
    let output_write2 = Arc::clone(&output);
    let output_read = Arc::clone(&output);

    let config = Config::load()
        .expect("config has to exist at this point, unless some system operation failed");
    let config2 = config.clone();

    thread::spawn(move || -> Result<(), anyhow::Error> {
        if let Err(err) = config.test() {
            write_output(&output_write2, &err.to_string())
                .expect("should be able to write to window at this point");
            drop(fish_rx);
            return Err(err);
        }
        if let Err(err) = fish_rx.recv() {
            write_output(&output_write2, &err.to_string())
                .expect("should be able to write to window at this point");
            return Err(err.into());
        }
        let err: anyhow::Error = loop {
            if let Err(err) = fish_rx.recv() {
                break err.into();
            }
            if let Err(err) = handle_notification(&output_write2, &config) {
                break err;
            }
        };
        write_output(&output_write2, &err.to_string())
            .expect("should be able to write to window at this point");
        Err(err)
    });

    thread::spawn(move || -> Result<(), anyhow::Error> {
        let mut welcome_count = 0;
        if let Err(err) = fish_tx.send("Healthy!") {
            write_output(&output_write1, &err.to_string())
                .expect("should be able to write to window at this point");
            return Err(err.into());
        }
        let err: anyhow::Error = loop {
            let msg: TwitchMessage = match rx.recv() {
                Ok(msg) => msg,
                Err(err) => break err.into(),
            };
            match msg {
                TwitchMessage::Notification(_) => {
                    if let Err(err) = fish_tx.send("Ping!") {
                        break err.into();
                    }
                }
                TwitchMessage::Welcome(msg) => {
                    welcome_count += 1;
                    if welcome_count == 1 {
                        let session_id = msg.session_id().to_owned();
                        if let Err(err) = subscribe(&output_write1, session_id, &config2) {
                            break err;
                        }
                        write_output(&output_write1, "Subscribed!")
                            .expect("should be able to write to window at this point");
                    } else {
                        write_output(&output_write1, "Reconnected to Twitch!")
                            .expect("should be able to write to window at this point");
                    }
                }
                _ => {}
            }
        };
        write_output(&output_write1, &err.to_string())
            .expect("should be able to write to window at this point");
        Err(err)
    });

    eframe::run_native(
        "Pond opener 3000™",
        options,
        Box::new(move |_cc| {
            Box::new(FishingeOutput {
                output: output_read,
            })
        }),
    );

    Ok(())
}

fn handle_notification(output: &Arc<Mutex<String>>, config: &Config) -> Result<()> {
    fishinge::update_command(output, config)
}

fn subscribe(output: &Arc<Mutex<String>>, session_id: String, config: &Config) -> Result<()> {
    let (broadcaster_id, reward_id) = get_ids(config)?;
    write_output(
        output,
        &format!(
            "Got ids:\n\tBroadcaster: {},\n\tReward: {}",
            broadcaster_id, reward_id
        ),
    )?;
    create_subscription(config, session_id, broadcaster_id, reward_id)
}
