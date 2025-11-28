#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release


use std::collections::HashMap;
use std::future::IntoFuture;
use std::path::{Path, PathBuf};

use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use csgo_gsi::update::player::WeaponState;
use eframe::egui;
use cs2_buttplug::config::Config;
use cs2_buttplug::{async_main, spawn_buttplug_client, ClientEvent, CloseEvent, GuiEvent};
use futures::future::RemoteHandle;
use futures::FutureExt;
use log::error;
use tokio::runtime::Runtime;
use std::{fs, io};
use pretty_env_logger::env_logger::Target;

const CS2_BP_DIR_PATH: &str = "CS2_BP_DIR_PATH";
const CS2_BP_PORT: &str = "CS2_BP_PORT";
const CS2_BP_INTIFACE_ADDR: &str = "CS2_BP_INTIFACE_ADDR";

// This struct is used as an adaptor, it implements io::Write and forwards the buffer to a mpsc::Sender
struct GuiLog {
    //log_buffer: &'a mut Vec<String>,
    logs: Arc<Mutex<Vec<String>>>,
}

impl io::Write for GuiLog {
    // On write we forward each u8 of the buffer to the sender and return the length of the buffer
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        
        self.logs.lock().unwrap().push(std::str::from_utf8(buf).unwrap().to_string());
        std::io::stdout().write(buf).expect("failed to write to stdout");
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn main() -> Result<(), eframe::Error> {
    let logs: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    
    pretty_env_logger::formatted_builder()
        .filter_level(log::LevelFilter::Warn)
        .filter(Some("cs2_buttplug"), log::LevelFilter::max())
        .filter(Some("cs2_buttplug_ui"), log::LevelFilter::max())
        .filter(Some("csgo_gsi"), log::LevelFilter::max())
        .target(Target::Pipe(Box::new(GuiLog{logs: logs.clone()})))
        .init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([640.0, 640.0]),
        ..Default::default()
    };

    eframe::run_native(
        "cs2-buttplug-ui",
        options,
        Box::new(|cc| {
            let mut config = Config::default();

            if let Some(storage) = cc.storage {
                if let Some(csgo_dir_path) = storage.get_string(CS2_BP_DIR_PATH) {
                    if let Ok(path) = PathBuf::from_str(csgo_dir_path.as_ref()) {
                        config.cs_script_dir = Some(path);
                    }
                }
                if let Some(port) = storage.get_string(CS2_BP_PORT) {
                    if let Ok(port) = port.parse::<u16>() {
                        config.cs_integration_port = port;
                    }
                }
                if let Some(addr) = storage.get_string(CS2_BP_INTIFACE_ADDR) {
                    config.buttplug_server_url = addr;
                }
            }

            Ok(Box::<CsButtplugUi>::new(CsButtplugUi::new(config, logs)))
        }),
    )
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum Status {
    Absent,
    Disabled,
    Enabled
}

struct CsButtplugUi {
    tokio_runtime: Runtime,
    main_handle: Option<RemoteHandle<()>>,
    buttplug_thread: Option<RemoteHandle<()>>,
    config: Config,
    close_send: tokio::sync::broadcast::Sender<CloseEvent>,
    _close_receive: tokio::sync::broadcast::Receiver<CloseEvent>,

    client_events: Option<tokio::sync::broadcast::Receiver<ClientEvent>>,
    gui_send: Option<tokio::sync::broadcast::Sender<GuiEvent>>,

    editor_port: String,

    logs: Arc<Mutex<Vec<String>>>,

    devices: HashMap<u32, (String, Status)>,

    autostarted: bool,

    last_update: Arc<Mutex<Option<csgo_gsi::Update>>>,

    settings_window: bool,
}

impl CsButtplugUi {
    fn new(config: Config, logs: Arc<Mutex<Vec<String>>>) -> Self {
        let (close_send, _close_receive) = tokio::sync::broadcast::channel(64);

        Self {
            tokio_runtime: Runtime::new().unwrap(),
            main_handle: None,
            buttplug_thread: None,
            config: config,
            close_send, _close_receive,
            client_events: None,
            gui_send: None,
            editor_port: "".to_string(),
            logs,
            devices: HashMap::new(),
            autostarted: false,
            last_update: Arc::new(Mutex::new(None)),
            settings_window: false,
        }
    }
}

impl CsButtplugUi {
    pub fn launch_main(&mut self) {
        // TODO: end previous
        let handle = self.tokio_runtime.handle().clone();
        let mut config = self.config.clone();
        let sender = self.close_send.clone();

        config.rhai_script_path = match config.rhai_script_path {
            Some(_) => config.rhai_script_path,
            None => Some(Path::new("./scripts/default_script.rhai").to_path_buf())
        };

        let (client_send, client_receive) = tokio::sync::broadcast::channel(64);
        self.client_events = Some(client_receive);
        let (gui_send, gui_receive) = tokio::sync::broadcast::channel(64);
        self.gui_send = Some(gui_send);

        //let mut gsi_update = None;

        let bp_future = async {
            spawn_buttplug_client(&config.buttplug_server_url, sender.subscribe(), Some(client_send), Some(gui_receive)).await.unwrap()
        };

        let (buttplug_send, buttplug_thread) = self.tokio_runtime.block_on(bp_future);

        let mutex = self.last_update.clone();

        let func = move |gsi_update: &csgo_gsi::Update| { 
            let mut locked_update = mutex.lock().unwrap(); 
            *locked_update = Some(gsi_update.clone());
        };

        let (main_future, main_handle) = async move {
            let _ = async_main(config, handle, sender, buttplug_send, Some(func) /* None::<fn(&csgo_gsi::Update)> */ /* here */).await;
        }.remote_handle();

        let tokio_handle = self.tokio_runtime.handle().clone();
        self.tokio_runtime.spawn_blocking(move || {
            tokio_handle.block_on(main_future);
        });

        self.buttplug_thread = Some(buttplug_thread);
        self.main_handle = Some(main_handle);
    }

    pub fn close(&mut self) {
        self.close_send.send(CloseEvent{}).expect("Failed to send close event.");
        if let Some(handle) = &mut self.buttplug_thread {
            self.tokio_runtime.block_on(handle.into_future());
        }
        self.buttplug_thread = None;
        if let Some(handle) = &mut self.main_handle {
            self.tokio_runtime.block_on(handle.into_future());
        }
        self.main_handle = None;
    }
}

impl eframe::App for CsButtplugUi {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {

        if let Some(handle) = &mut self.main_handle {
            if let Some(_) = handle.now_or_never() {
                self.main_handle = None;
            }
        }

        ctx.request_repaint_after(Duration::from_millis(10));

        if let Some(ref mut client_events) = self.client_events {
            while let Ok(ev) = client_events.try_recv() {
                match ev {
                    ClientEvent::DeviceFound(name, index) => {
                        self.devices.insert(index, (name, Status::Enabled));
                    },
                    ClientEvent::DeviceLost(name, index) => {
                        self.devices.insert(index, (name, Status::Absent));
                    },
                }
            }
        }
        
        egui::CentralPanel::default().show(ctx, |mut ui| {

            ui.heading("CS2 Buttplug.io integration");
            ui.add_space(5.0);

            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                ui.label(format!("This is cs2-buttplug (gui), v{}, github: ", env!("CARGO_PKG_VERSION"))); 
                ui.hyperlink("https://github.com/gloss-click/cs2-buttplug");
            });
            ui.label("Credit to original author hornycactus.");
    
            ui.separator();
            ui.add_space(5.0);

            ui.heading(format!("Status: {}", match self.main_handle {
                Some(_) => "launched",
                None => "stopped",
            }));

            ui.horizontal(|ui| {
                if self.main_handle.is_none() {
                    if ui.button("Launch").clicked() {
                        self.launch_main();
                    }
                    if !self.autostarted {
                        self.launch_main();
                        self.autostarted = true
                    }
                } else {
                    if ui.button("Relaunch").clicked() {
                        self.close();
                        self.launch_main();
                    }
                }

                ui.set_enabled(match self.main_handle {
                    Some(_) => true,
                    None => false,
                });

                if ui.button("Stop").clicked() {
                    self.close();
                }
            });

            if ctx.input(|i| i.viewport().close_requested()) {
                self.close();
            }

            egui::CollapsingHeader::new("Settings").show(&mut ui, |mut ui| {

                ui.set_enabled(match self.main_handle {
                    Some(_) => false,
                    None => true,
                });
                ui.label("Path to CS2 integration script dir:");
                ui.horizontal(|ui| {
                    ui.label(match &self.config.cs_script_dir {
                        Some(path) => path.display().to_string(),
                        None => "[None]".to_string(),
                    });
                    
                    if ui.button("Browse").clicked() {
                        if let Some(path) = rfd::FileDialog::new().pick_folder() {
                            if let Some(storage) = frame.storage_mut() {
                                storage.set_string(CS2_BP_DIR_PATH, path.display().to_string())
                            }

                            self.config.cs_script_dir = Some(path);
                        }
                    }
                });

                ui.add_space(5.0);
                
                ui.label(format!("Intiface URL (default: ws://127.0.0.1:12345):"));
                if ui.text_edit_singleline(&mut self.config.buttplug_server_url).changed() {
                    if self.config.buttplug_server_url.len() > 0 {
                        if let Some(storage) = frame.storage_mut() {
                            storage.set_string(CS2_BP_INTIFACE_ADDR, self.config.buttplug_server_url.clone());
                        }
                    }
                }

                ui.add_space(5.0);

                ui.label(format!("Advanced: GSI integration port (default 42069): {}", self.config.cs_integration_port));

                if ui.text_edit_singleline(&mut self.editor_port).changed() {
                    match self.editor_port.parse::<u16>() {
                        Ok(i) => { 
                            if let Some(storage) = frame.storage_mut() {
                                storage.set_string(CS2_BP_PORT, i.to_string());
                            }

                            self.config.cs_integration_port = i; 
                        },
                        Err(_) => {},
                    }
                }
            });

            ui.separator();
            ui.add_space(5.0);

            const DEFAULT_SCRIPT: &'static str = "default_script.rhai";

            ui.heading(format!("Script: {}", match &self.config.rhai_script_path {
                Some(path) => path.file_name().unwrap().to_str().unwrap(),
                None => DEFAULT_SCRIPT,
            }));

            let script_dir_path = Path::new("./scripts");

            if !script_dir_path.exists() {
                match fs::create_dir(script_dir_path) {
                    Ok(_) => {
                        let script_path = script_dir_path.join(DEFAULT_SCRIPT);
                        match fs::write(&script_path, include_str!("../default_script.rhai")) {
                            Ok(_) => {},
                            Err(e) => {
                                ui.label(e.to_string());
                            }
                        }
                    },
                    Err(e) => {
                        ui.label(e.to_string());
                    }
                }
            }

            ui.horizontal(|ui| {
                let paths = fs::read_dir(script_dir_path);
                match paths {
                    Ok(paths) => {
                        for path in paths {
                            if let Ok(path) = path {
                                if (ui.selectable_value(&mut self.config.rhai_script_path, Some(path.path()), path.file_name().to_str().unwrap())).clicked() {
                                    self.close();
                                    self.launch_main();
                                }
                            }
                        }
                    },
                    Err(e) => {
                        ui.label(e.to_string());
                    }
                }
            });

            ui.separator();
            ui.add_space(5.0);

            ui.heading("Devices");

            fn status_to_label(status: &Status) -> &str {                
                match status {
                    Status::Enabled => "Enabled",
                    Status::Disabled => "Disabled",
                    Status::Absent => "Disconnected"
                }
            }

            for (index, (name, status)) in self.devices.iter_mut() {
                ui.set_enabled(match status {
                    Status::Absent => false,
                    _ => true,
                });

                let mut checked = *status == Status::Enabled;
                if (ui.checkbox(&mut checked, format!("{}: {}", name, status_to_label(status)))).changed() {
                    *status = match checked {
                        true => Status::Enabled,
                        false => Status::Disabled,
                    };
                    if let Some(ref mut gui_send) = self.gui_send {
                        match gui_send.send(GuiEvent::DeviceToggle(*index, checked)) {
                            Ok(_) => {},
                            Err(e) => error!("Error sending GUI command: {}", e),
                        }
                    }
                }
            }
            ui.separator();
            ui.add_space(5.0);

            ui.heading("Game State");

            egui::CollapsingHeader::new("Values").show(&mut ui, |mut ui| {
                let locked_update = self.last_update.lock().unwrap();

                if let Some(ref update) = *locked_update {
                    if let Some(ref player) = update.player {
                        ui.label(format!("name: {} team: {} spectating: {}", 
                            player.name, 
                            match player.team { None => "", Some(csgo_gsi::update::Team::T) => "Terrorists", Some(csgo_gsi::update::Team::CT) => "CTs"},
                            match &update.provider {
                                Some(provider) => match &provider.steam_id {
                                    i if *i == player.steam_id => "no",
                                    _ => "yes",
                                },
                                _ => "unknown",
                            }
                        ));
                        if let Some(ref match_stats) = player.match_stats {
                            ui.label(format!("kills: {} deaths: {} assists: {} mvps: {}", match_stats.kills, match_stats.deaths, match_stats.assists, match_stats.mvps));
                        }
                        if let Some(ref state) = player.state {
                            ui.label(format!("health: {} armour: {} helmet: {} cash: {}", state.health, state.armor, match state.helmet { true => "yes", false => "no" }, state.money));
                            ui.label(format!("flashed: {} smoked: {} burning: {} defuse kit: {}", 
                                match state.flashed { i if i > 0 => "yes", _ => "no" }, 
                                match state.smoked { i if i > 0 => "yes", _ => "no" },
                                match state.burning { i if i > 0 => "yes", _ => "no" }, match state.defuse_kit { Some(true) => "yes", _ => "no" }
                            ));
                        }
                        
                        for (_name, weapon) in player.weapons.iter() {
                            match weapon.state {
                                WeaponState::Active => {
                                    if let (Some(clip), Some(mmax)) = (weapon.ammo_clip, weapon.ammo_clip_max) {
                                        ui.label(format!("active weapon: {} ammo: {}/{}", weapon.name, clip, mmax));
                                    }
                                },
                                _ => {},
                            }
                        }
                    }
                }
            });

            ui.separator();

            egui::CollapsingHeader::new("Log").show(&mut ui, |mut ui| {
                egui::ScrollArea::new([false, true]).max_height(400.0).stick_to_bottom(true).auto_shrink([false, true]).show(&mut ui, |ui| {
                    let logs = self.logs.lock().unwrap();

                    let log = logs.concat();

                    ui.add_sized(egui::vec2(650.0, 100.0), egui::TextEdit::multiline(&mut log.as_ref()));
                });
            });
            
        });
    }
}