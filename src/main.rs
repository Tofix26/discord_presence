#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

mod image;
mod menu_bar;
mod presence_button;
mod preset;
mod storage;
mod timestamp;

use storage::Storage;
use timestamp::{Timestamp, TimestampEnum};

use std::time::Duration;
use std::vec;

use serde_json::{from_str, to_string};

use discord_rich_presence::activity::{Activity, Assets, Button, Party, Timestamps};
use discord_rich_presence::{DiscordIpc, DiscordIpcClient};

use chrono::{DateTime, Local, Utc};

use eframe::egui::{self, Layout, Vec2};
use eframe::emath::Align;
use eframe::{run_native, NativeOptions};

fn main() {
    let options = NativeOptions {
        decorated: true,
        drag_and_drop_support: true,
        icon_data: None,
        initial_window_size: Some(Vec2::new(600.0, 650.0)),
        resizable: false,
        vsync: true,
        ..Default::default()
    };
    run_native(
        "Discord Presence",
        options,
        Box::new(|cc| Box::new(App::new(cc))),
    );
}
struct App {
    menu_bar: menu_bar::MenuBar,
    first_btn: presence_button::PresenceButton,
    second_btn: presence_button::PresenceButton,
    first_img: image::Image,
    second_img: image::Image,
    id: String,
    details: String,
    state: String,
    party: u8,
    party_of: u8,
    timestamp: timestamp::Timestamp,
    client: DiscordIpcClient,
    connected: bool,
    started: DateTime<Utc>,
    last_update: DateTime<Utc>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            menu_bar: menu_bar::MenuBar::default(),
            first_btn: presence_button::PresenceButton::default(),
            second_btn: presence_button::PresenceButton::default(),
            first_img: image::Image::default(),
            second_img: image::Image::default(),
            id: String::new(),
            details: String::new(),
            party: 0,
            party_of: 0,
            state: String::new(),
            timestamp: Timestamp::default(),
            client: DiscordIpcClient::new("0").unwrap(),
            connected: false,
            started: Utc::now(),
            last_update: Utc::now(),
        }
    }
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let storage = match cc.storage.unwrap().get_string("settings") {
            None => "".to_string(),
            Some(value) => value,
        };
        let storage: Storage = match from_str(&storage) {
            Ok(storage) => storage,
            Err(_) => Storage::default(),
        };
        match storage.darkmode {
            true => cc.egui_ctx.set_visuals(egui::Visuals::dark()),
            false => cc.egui_ctx.set_visuals(egui::Visuals::light()),
        }
        let mut client = DiscordIpcClient::new(&storage.id)
            .expect("Failed to create client while loading storage");
        if storage.autoconnect {
            client.connect().expect("Failed to autoconnect on startup");
        }
        let mut app = App {
            id: storage.id,
            details: storage.details,
            state: storage.state,
            party: storage.party,
            party_of: storage.party_of,
            timestamp: timestamp::Timestamp {
                timestamp: storage.timestamp,
                date: Utc::now().date(),
            },
            first_btn: presence_button::PresenceButton {
                label: storage.first_btn_label,
                url: storage.first_btn_url,
            },
            second_btn: presence_button::PresenceButton {
                label: storage.second_btn_label,
                url: storage.second_btn_url,
            },
            first_img: image::Image {
                key: storage.large_image_key,
                text: storage.large_image_label,
            },
            second_img: image::Image {
                key: storage.small_image_key,
                text: storage.small_image_label,
            },
            menu_bar: menu_bar::MenuBar {
                autoconnect: storage.autoconnect,
                darkmode: storage.darkmode,
                ..Default::default()
            },
            client,
            ..Default::default()
        };
        if storage.autoconnect {
            app.set_presence();
            app.connected = true;
        }
        app
    }
}

impl eframe::App for App {
    fn persist_native_window(&self) -> bool {
        false
    }
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        let save = Storage::new(
            &self.id,
            &self.details,
            &self.state,
            self.party,
            self.party_of,
            self.timestamp.timestamp,
            &self.first_img.key,
            &self.second_img.key,
            &self.first_img.text,
            &self.second_img.text,
            &self.first_btn.label,
            &self.second_btn.label,
            &self.first_btn.url,
            &self.second_btn.url,
            self.menu_bar.autoconnect,
            self.menu_bar.darkmode,
        );
        storage.set_string(
            "settings",
            to_string(&save).expect("Failed to parse save struct"),
        );
    }
    fn auto_save_interval(&self) -> std::time::Duration {
        Duration::from_secs(5)
    }
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.menu_bar.run(ctx);
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.with_layout(Layout::top_down_justified(Align::Center), |ui| {
                ui.heading("Discord Presence");
            });
            ui.horizontal(|ui| {
                ui.add_space(60.);
                ui.label("ID");
                ui.text_edit_singleline(&mut self.id);
                ui.add_space(10.);
                if ui
                    .add_enabled(!self.connected, egui::Button::new("Connect"))
                    .clicked()
                {
                    if self.id != "".to_string() {
                        self.client = DiscordIpcClient::new(&self.id).expect("sus");
                        self.client.connect().expect("Failed to connect to discord");
                        self.last_update = Utc::now();
                        self.set_presence();
                        self.connected = true;
                    }
                }
                ui.add_space(10.);
                if ui
                    .add_enabled(self.connected, egui::Button::new("Disconnect"))
                    .clicked()
                {
                    self.client.close().expect("Failed to disconnect");
                    self.connected = false;
                }
            });
            ui.add_space(5.);
            ui.horizontal(|ui| {
                ui.add_space(34.);
                ui.label("Details");
                ui.text_edit_singleline(&mut self.details);
            });
            ui.add_space(5.);
            ui.horizontal(|ui| {
                ui.add_space(42.);
                ui.label("State");
                ui.text_edit_singleline(&mut self.state);
                ui.label("Party");
                ui.add(egui::DragValue::new(&mut self.party).clamp_range(0..=32));
                ui.label("of");
                ui.add(egui::DragValue::new(&mut self.party_of).clamp_range(1..=32));
            });
            ui.add_space(15.);
            self.timestamp.run(ui);
            ui.add_space(15.);
            ui.horizontal(|ui| {
                ui.add_space(75.);
                self.first_img.run(ui, "Large Image");
                self.second_img.run(ui, "Small Image");
            });
            ui.horizontal(|ui| {
                ui.add_space(75.);
                self.first_btn.run(ui, "Button 1");
                self.second_btn.run(ui, "Button 2");
            });
            ui.add_space(50.);
            ui.with_layout(Layout::top_down(Align::Center), |ui| {
                if ui
                    .add_enabled(
                        self.connected,
                        egui::widgets::Button::new("Update Presence"),
                    )
                    .clicked()
                {
                    self.last_update = Utc::now();
                    self.set_presence()
                }
            });
        });
        egui::containers::Window::new("About")
            .open(&mut self.menu_bar.about_me)
            .resizable(false)
            .fixed_size(Vec2::new(200., 100.))
            .show(ctx, |ui| {
                ui.with_layout(Layout::top_down(Align::Center), |ui| {
                    ui.heading("Discord Presence");
                    ui.label("Version v0.2-beta");
                });
            });
        self.load_preset()
    }
}
impl App {
    fn set_presence(&mut self) {
        if self.id != self.client.client_id {
            self.client
                .close()
                .expect("Failed to disconnect while updating application");
            self.client = DiscordIpcClient::new(&self.id)
                .expect("Failed to create client while updating application id");
            self.client.connect().expect("Failed to connect to discord");
        }
        let first_btn = Button::new(&self.first_btn.label, &self.first_btn.url);
        let second_btn = Button::new(&self.second_btn.label, &self.second_btn.url);
        let mut buttons = vec![];
        let timestamp = match self.timestamp.timestamp {
            TimestampEnum::LocalTime => {
                let hour = Local::now()
                    .format("%H")
                    .to_string()
                    .parse::<i64>()
                    .unwrap()
                    * 3_600;
                let minute = Local::now()
                    .format("%M")
                    .to_string()
                    .parse::<i64>()
                    .unwrap()
                    * 60;
                let second = Local::now()
                    .format("%S")
                    .to_string()
                    .parse::<i64>()
                    .unwrap();
                let local_time = Utc::now().timestamp() - (hour + minute + second);
                Timestamps::new().start(local_time)
            }
            TimestampEnum::CustomTimeStamp => Timestamps::new()
                .start(self.timestamp.date.naive_utc().and_hms(0, 0, 0).timestamp()),
            TimestampEnum::SinceStart => Timestamps::new().start(self.started.timestamp()),
            TimestampEnum::SinceLastUpdate => Timestamps::new().start(self.last_update.timestamp()),
            _ => Timestamps::new(),
        };
        let assets = Assets::new();
        let assets = match self.first_img.key.as_str() {
            "" => assets,
            _ => assets.large_image(&self.first_img.key),
        };
        let assets = match self.first_img.text.as_str() {
            "" => assets,
            _ => assets.large_text(&self.first_img.text),
        };
        let assets = match self.second_img.key.as_str() {
            "" => assets,
            _ => assets.small_image(&self.second_img.key),
        };
        let assets = match self.second_img.text.as_str() {
            "" => assets,
            _ => assets.small_text(&self.second_img.text),
        };
        let activity = Activity::new().timestamps(timestamp).assets(assets);

        let activity = match self.details.as_str() {
            "" => activity,
            _ => activity.details(&self.details),
        };

        let activity = match self.state.as_str() {
            "" => activity,
            _ => activity.state(&self.state),
        };
        let first_btn_label_exists = self.first_btn.label != "".to_string();
        let first_btn_url_exists = self.first_btn.url != "".to_string();
        if first_btn_label_exists && first_btn_url_exists {
            buttons.push(first_btn);
        }

        let second_btn_label_exists = self.second_btn.label != "".to_string();
        let second_btn_url_exists = self.second_btn.url != "".to_string();
        if second_btn_label_exists && second_btn_url_exists {
            buttons.push(second_btn);
        }

        let activity = match buttons.len() > 0 {
            true => activity.buttons(buttons),
            false => activity,
        };

        let part_exists = self.party != 0;
        let activity = match part_exists && self.state != "" {
            true => activity.party(Party::new().size([self.party_of as i32, self.party as i32])),
            false => activity,
        };
        self.client
            .set_activity(activity)
            .expect("Failed to set activity");
    }
    fn load_preset(&mut self) {
        if self.menu_bar.loaded_preset != None {
            let preset = self.menu_bar.loaded_preset.as_ref().unwrap();
            if preset.ID != None {
                self.id = preset.ID.as_ref().unwrap().to_string();
            }
            if preset.Details != None {
                self.details = preset.Details.as_ref().unwrap().to_string();
            }
            if preset.State != None {
                self.state = preset.State.as_ref().unwrap().to_string();
            }
            if preset.PartySize != None {
                self.party = preset.PartySize.unwrap();
            }
            if preset.PartyMax != None {
                self.party_of = preset.PartyMax.unwrap();
            }
            self.timestamp.timestamp = preset.timestamp();
            if preset.LargeKey != None {
                self.first_img.key = preset.LargeKey.as_ref().unwrap().to_string()
            }
            if preset.LargeText != None {
                self.first_img.text = preset.LargeText.as_ref().unwrap().to_string()
            }
            if preset.SmallKey != None {
                self.second_img.key = preset.SmallKey.as_ref().unwrap().to_string()
            }
            if preset.SmallText != None {
                self.second_img.text = preset.SmallText.as_ref().unwrap().to_string()
            }
            if preset.Button1Text != None {
                self.first_btn.label = preset.Button1Text.as_ref().unwrap().to_string()
            }
            if preset.Button1URL != None {
                self.first_btn.url = preset.Button1URL.as_ref().unwrap().to_string()
            }
            if preset.Button2Text != None {
                self.second_btn.label = preset.Button2Text.as_ref().unwrap().to_string()
            }
            if preset.Button2URL != None {
                self.second_btn.url = preset.Button2URL.as_ref().unwrap().to_string()
            }
            self.menu_bar.loaded_preset = None
        }
    }
}
