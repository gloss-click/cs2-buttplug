#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use std::future::IntoFuture;
use std::path::PathBuf;
use std::str::FromStr;
use eframe::egui;
use cs2_buttplug::config::Config;
use cs2_buttplug::{async_main, CloseEvent};
use futures::future::RemoteHandle;
use futures::FutureExt;
use tokio::runtime::Runtime;

const CS2_BP_DIR_PATH: &str = "CS2_BP_DIR_PATH";
const CS2_BP_PORT: &str = "CS2_BP_PORT";
const CS2_BP_INTIFACE_ADDR: &str = "CS2_BP_INTIFACE_ADDR";

fn main() -> Result<(), eframe::Error> {
    pretty_env_logger::formatted_builder()
        .filter_level(log::LevelFilter::Warn)
        .filter(Some("cs2_buttplug"), log::LevelFilter::max())
        .filter(Some("cs2_buttplug_ui"), log::LevelFilter::max())
        .filter(Some("csgo_gsi"), log::LevelFilter::max())
        .init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([640.0, 300.0]),
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

            Box::<CsButtplugUi>::new(CsButtplugUi::new(config))
        }),
    )
}

struct CsButtplugUi {
    tokio_runtime: Runtime,
    main_handle: Option<RemoteHandle<()>>,
    config: Config,
    close_send: tokio::sync::broadcast::Sender<CloseEvent>,
    _close_receive: tokio::sync::broadcast::Receiver<CloseEvent>,

    editor_port: String,
}

impl CsButtplugUi {
    fn new(config: Config) -> Self {
        let (close_send, _close_receive) = tokio::sync::broadcast::channel(64);

        Self {
            tokio_runtime: Runtime::new().unwrap(),
            main_handle: None,
            config: config,
            close_send, _close_receive,
            editor_port: "".to_string(),
        }
    }
}

impl CsButtplugUi {
    pub fn launch_main(&mut self) {
        // TODO: end previous
        let handle = self.tokio_runtime.handle().clone();
        let config = self.config.clone();
        let sender = self.close_send.clone();
        let (main_future, main_handle) = async {
            let _ = async_main(config, handle, sender).await;
        }.remote_handle();

        let tokio_handle = self.tokio_runtime.handle().clone();
        self.tokio_runtime.spawn_blocking(move || {
            tokio_handle.block_on(main_future);
        });

        self.main_handle = Some(main_handle);
    }

    pub fn close(&mut self) {
        self.close_send.send(CloseEvent{}).expect("Failed to send close event.");
        if let Some(handle) = &mut self.main_handle {
            self.tokio_runtime.block_on(handle.into_future());
        }
        self.main_handle = None;
    }
}

impl eframe::App for CsButtplugUi {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("CS2 Buttplug.io integration");
            ui.add_space(5.0);
            
            ui.label(format!("This is cs2-buttplug (gui), v{}, original author hornycactus (https://cactus.sexy)", env!("CARGO_PKG_VERSION")));
    
            ui.separator();
            ui.add_space(5.0);

            ui.vertical(|ui| {
                ui.heading("Settings");

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

            ui.add_space(5.0);
            ui.separator();

            ui.heading("Start");

            if self.main_handle.is_none() {
                if ui.button("Launch").clicked() {
                    self.launch_main();
                }
            } else {
                if ui.button("Relaunch").clicked() {
                    self.close();
                    self.launch_main();
                }
            }

            ui.vertical(|ui| {
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
        });
    }
}