
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;
use std::sync::Mutex;

use egui::Key;
use egui::UserAttentionType;

use crate::{KeysightDevice, DeviceState, SiglentDevice, SiglentDeviceState, WaveformPoint, scpi_commands, emg_scpi_commands};


#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
    siglent_device: SiglentDevice,
    keysight_device: KeysightDevice,
    #[serde(skip)]
    device_state: DeviceState,
    #[serde(skip)]
    siglent_device_state: SiglentDeviceState,
    stimulation_amplitude: f64,
    pulse_width: f64,
    #[serde(skip)]
    siglent_connection_status: String,
    #[serde(skip)]
    connection_status: String,
    #[serde(skip)]
    is_stimulating: Arc<Mutex<bool>>,
    #[serde(skip)]
    stimulation_status: String,
    #[serde(skip)]
    progress_receiver: Option<Receiver<String>>,
    #[serde(skip)]
    waveform_receiver: Option<Receiver<(Vec<WaveformPoint>, f64)>>,

}

impl Default for TemplateApp {
    fn default() -> Self {
        Self {
            siglent_device: SiglentDevice::default(),
            siglent_device_state: SiglentDeviceState::default(),
            keysight_device: KeysightDevice::default(),
            device_state: DeviceState::default(),
            stimulation_amplitude: 0.0,
            pulse_width: 0.0001,
            connection_status: "Disconnected".to_owned(),
            siglent_connection_status: "Disconnected".to_owned(),
            is_stimulating: Arc::new(Mutex::new(false)),
            stimulation_status: "Idle".to_owned(),
            progress_receiver: None,
            waveform_receiver: None,
        }
    }
}

impl TemplateApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`            tcp_connection: None,.

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
         if let Some(storage) = cc.storage {
             eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
         } else {
             Default::default()
         }
    }
}

impl eframe::App for TemplateApp {
    /// Called by the framework to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Put your widgets into a `SidePanel`, `TopBottomPanel`, `CentralPanel`, `Window` or `Area`.
        // For inspiration and more examples, go to https://emilk.github.io/egui

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:

            egui::MenuBar::new().ui(ui, |ui| {
                // NOTE: no File->Quit on web pages!
                let is_web = cfg!(target_arch = "wasm32");
                if !is_web {
                    ui.menu_button("File", |ui| {
                        if ui.button("Quit").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                    ui.add_space(16.0);
                }

                egui::widgets::global_theme_preference_buttons(ui);
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {

            ui.horizontal(|ui| {
            ui.vertical(|ui| {
            ui.set_width(300.0);

            // The central panel the region left after adding TopPanel's and SidePanel's
            ui.heading("Paired associative stimulation");

            ui.horizontal(|ui| {
                ui.label("IP-address: ");
                ui.text_edit_singleline(&mut self.keysight_device.ip_address);
            });

            ui.horizontal(|ui| {
                ui.label("Port: ");
                ui.text_edit_singleline(&mut self.keysight_device.port);
            });

            if ui.button("Connect to Keysight").clicked() {
                self.connection_status = "Connecting...".to_owned();
                let result = scpi_commands::connect_to_keysight(&mut self.keysight_device, &mut self.device_state);
                match result {
                    Ok(()) => {
                        self.connection_status = "Keysight connection successful".to_owned();
                    }
                    Err(e) => {
                        self.connection_status = format!("Keysight connection failed :( : {}", e);
                    }
                }
            }

            ui.separator();
            ui.label(&self.connection_status);
            ui.separator();

            if ui.button("Reset Keysight").clicked() {
            let keysight_device_clone = self.keysight_device.clone();
            let device_state_clone = self.device_state.clone();
            std::thread::spawn(move || {
                let mut device_state = device_state_clone;
                let mut keysight_device = keysight_device_clone;
                let result = scpi_commands::reset_device(&mut keysight_device, &mut device_state);
                let _status = match result {
                    Ok(()) => "Reset Successful!".to_owned(),
                    Err(e) => format!("Reset Failed: {}", e),
                };
            });
            }

            ui.add(egui::Slider::new(&mut self.stimulation_amplitude, 0.0..=20.0).text("Stimulation amplitude (mA)"));
            ui.add(egui::Slider::new(&mut self.pulse_width, 0.0001..=0.0008).text("Pulse width (s)"));

            if ui.button("Setup for stimulation").clicked() {
                let device_state_clone = self.device_state.clone();
                let stimulation_amplitude = self.stimulation_amplitude;
                let pulse_width = self.pulse_width;
                std::thread::spawn(move || {
                    let mut device_state = device_state_clone;
                    let result = setup(&mut device_state, stimulation_amplitude, pulse_width);
                    let _status = match result {
                        Ok(()) => "Setup Successful!".to_owned(),
                        Err(e) => format!("Setup Failed: {}", e),
                    };
                });

            }

            if ui.button("Start PAS stimulation sequence").clicked() {
                let _ = scpi_commands::turn_on_ch1_ch2(&mut self.device_state);
                if let Some(mut stream) = self.device_state.tcp_connection.take() {
                    let is_stimulating = self.is_stimulating.clone();
                    {
                        let mut is_stimulating_lock = is_stimulating.lock().unwrap();
                        *is_stimulating_lock = true;
                    }
                    self.stimulation_status = "Starting stimulation...".to_owned();
                    let (sender, receiver) = channel();
                    self.progress_receiver = Some(receiver);
                    std::thread::spawn(move || {
                        if let Err(e) = scpi_commands::stimulate_and_send_trigger(&mut stream, sender, is_stimulating) {
                            eprintln!("Stimulation thread error: {}", e);
                        }
                    });
                } else {
                    self.stimulation_status = "Cannot start: Not connected.".to_owned();
                }
            }


            if ui.button("Stop stimulation sequence").clicked() {
                let is_stimulating = self.is_stimulating.clone();
                *is_stimulating.lock().unwrap() = false;
            }

            if let Some(receiver) = &self.progress_receiver {
                while let Ok(message) = receiver.try_recv() {
                    if message.contains("Finished") {
                        let mut is_stimulating = self.is_stimulating.lock().unwrap();
                        *is_stimulating = false;
                        self.connection_status = "Disconnected (stimulation finished)".to_owned();
                    }
                    self.stimulation_status = message;
                }
            }


            ui.label(&self.stimulation_status);
            ui.separator();
               });

            ui.vertical(|ui| {
                ui.heading("EMG measurement");
                ui.horizontal(|ui| {
                    ui.label("Occiloscope IP-address: ");
                    ui.text_edit_singleline(&mut self.siglent_device.ip_address);
                });
                ui.horizontal(|ui| {
                    ui.label("Port: ");
                    ui.text_edit_singleline(&mut self.siglent_device.port);
                });

                if ui.button("Connect to Occiloscope").clicked() {
                    self.siglent_connection_status = "Connecting...".to_owned();

                    let result = emg_scpi_commands::connect_to_occiloscope(&mut self.siglent_device, &mut self.siglent_device_state);

                    match result {
                        Ok(()) => {
                            self.siglent_connection_status = "Occiloscope connection successful ".to_owned();
                        }
                        Err(e) => {

                            self.siglent_connection_status = format!("Occiloscope connection failed :( : {}", e);
                        }
                    }
                }


                ui.separator();
                ui.label(&self.siglent_connection_status);
                ui.separator();

                if ui.button("Setup Keysight for EMG").clicked() {
                    let device_state_clone = self.device_state.clone();
                    let stimulation_amplitude = self.stimulation_amplitude;
                    let pulse_width = self.pulse_width;
                    std::thread::spawn(move || {
                        let mut device_state = device_state_clone;
                        let result = setup_for_emg(&mut device_state, stimulation_amplitude, pulse_width);
                        let _status = match result {
                            Ok(()) => "Setup Successful!".to_owned(),
                            Err(e) => format!("Setup Failed: {}", e),
                        };
                    });
                }

                if ui.button("Reset occiloscope").clicked() {
                let siglent_device_clone = self.siglent_device.clone();
                let device_state_clone = self.siglent_device_state.clone();
                std::thread::spawn(move || {
                    let mut device_state = device_state_clone;
                    let mut siglent_device = siglent_device_clone;
                    let result = emg_scpi_commands::reset_siglent_device(&mut siglent_device, &mut device_state);
                    let _status = match result {
                        Ok(()) => "Reset Successful!".to_owned(),
                        Err(e) => format!("Reset Failed: {}", e),
                    };
                });
                }

                if ui.button("Setup occiloscope for EMG").clicked() {
                    let siglent_device_state_clone = self.siglent_device_state.clone();
                    std::thread::spawn(move || {
                        let mut device_state = siglent_device_state_clone;
                        let result = setup_occiloscopefor_emg(&mut device_state) ;
                        let _status = match result {
                            Ok(()) => "Setup Successful!".to_owned(),
                            Err(e) => format!("Setup Failed: {}", e),
                        };
                    });
                }

                if ctx.input(|i| i.key_pressed(Key::Space)) {
                    let device_state_clone = self.device_state.clone();
                    ctx.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(UserAttentionType::Critical));
                    println!("STIMULATING");
                    std::thread::spawn(move || {
                        let device_state = device_state_clone;
                        if let Some(stream) = &device_state.tcp_connection {
                           let _ = emg_scpi_commands::stimulate_and_send_trigger(stream);
                        } else {
                            eprintln!("No connection available");
                        };
                    });
                }

                ui.separator();
                ui.separator();
                ui.label("PRESS SPACE TO STIMULATE");
                ui.separator();


            });

            });

            ui.add(egui::github_link_file!(
                "https://github.com/emilk/eframe_template/blob/main/",
                "Source code."
            ));

            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                powered_by_egui_and_eframe(ui);
                egui::warn_if_debug_build(ui);
            });
        });
    }
}


pub fn setup(
    mut device_state: &mut DeviceState,
    stimulation_amplitude: f64,
    pulse_width: f64,
) -> Result<(), String> {

    let _ = scpi_commands::set_impedance(&mut device_state);
    let _ = scpi_commands::configure_channel_1(&mut device_state);
    let _ = scpi_commands::configure_channel_2(&mut device_state, stimulation_amplitude, pulse_width);
    let _ = scpi_commands::arm_for_trigger(&mut device_state);

    Ok(())
}



pub fn setup_for_emg(
    mut device_state: &mut DeviceState,
    stimulation_amplitude: f64,
    pulse_width: f64,
) -> Result<(), String> {

    let _ = emg_scpi_commands::set_impedance(&mut device_state);
    let _ = emg_scpi_commands::configure_channel_2(&mut device_state, stimulation_amplitude, pulse_width);
    let _ = emg_scpi_commands::arm_for_trigger(&mut device_state);
    let _ = emg_scpi_commands::turn_on_ch2(&mut device_state);

    Ok(())
}


pub fn setup_occiloscopefor_emg(
    siglent_device_state: &mut SiglentDeviceState,
) -> Result<(), String> {

   let _ = emg_scpi_commands::configure_siglent_device(siglent_device_state);
    Ok(())
}


fn powered_by_egui_and_eframe(ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.label("Powered by ");
        ui.hyperlink_to("egui", "https://github.com/emilk/egui");
        ui.label(" and ");
        ui.hyperlink_to(
            "eframe",
            "https://github.com/emilk/egui/tree/master/crates/eframe",
        );
        ui.label(".");
    });
}
