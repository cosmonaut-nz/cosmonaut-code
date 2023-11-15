//!
//!
//!
//!
//!
use eframe::{egui, run_native, NativeOptions};
use egui::Color32;
use log::{debug, info};
use std::path::PathBuf;

const APP_TITLE: &str = ">_ cosmonaut code reviewer";
const LOGO_TXT: &str = ">_ cosmonaut";

/// Runs the app
pub fn run() {
    let options: NativeOptions = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(1080.0, 640.0)),
        ..Default::default()
    };
    let _ = run_native(
        APP_TITLE,
        options,
        Box::new(|_cc| {
            // Image support
            egui_extras::install_image_loaders(&_cc.egui_ctx);

            Box::<CodeReviewApp>::default()
        }),
    );
}

/// Struct to contain the UI features
struct CodeReviewApp {
    repo_path: PathBuf, // Enable input for the user to select the directory of the repository
    analysis_type: i32, // 1 for general code review, 2 for security review
    analysis_result: String, // output panel
                        // error_message: Option<String>,
}

impl Default for CodeReviewApp {
    fn default() -> Self {
        Self {
            repo_path: PathBuf::new(),
            analysis_type: 1,
            analysis_result: String::new(),
            // error_message: None,
        }
    }
}

impl eframe::App for CodeReviewApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Configure the window panel
        let configured_frame = egui::containers::Frame {
            inner_margin: egui::style::Margin {
                left: 1.,
                right: 1.,
                top: 1.,
                bottom: 1.,
            },
            outer_margin: egui::style::Margin {
                left: 1.,
                right: 1.,
                top: 1.,
                bottom: 1.,
            },
            rounding: egui::Rounding {
                nw: 0.1,
                ne: 0.1,
                sw: 0.1,
                se: 0.1,
            },
            shadow: eframe::epaint::Shadow {
                extrusion: 1.0,
                color: Color32::DARK_GRAY,
            },
            fill: Color32::LIGHT_GRAY,
            stroke: egui::Stroke::new(1.0, Color32::DARK_GRAY),
        };
        egui::CentralPanel::default()
            .frame(configured_frame)
            .show(ctx, |ui| {
                ui.set_width(ui.available_width());
                ui.set_height(ui.available_height());

                let available_size = ui.available_size(); // Get the available size
                let image_aspect_ratio = 1.0; // Replace with your image's aspect ratio

                ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                    let image_size = egui::vec2(
                        available_size.x.min(available_size.y * image_aspect_ratio),
                        available_size.y.min(available_size.x / image_aspect_ratio),
                    );

                    // TODO the UI elements are constraining size and or resizing...
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::Image::new(egui::include_image!(
                                "../../assets/img/cosmonaut_logo_trans.png"
                            ))
                            .max_size(image_size),
                        );
                        ui.heading(LOGO_TXT);
                    });
                    // Input text box for repository path
                    ui.horizontal(|ui| {
                        ui.label("Repository Path:");
                        if ui.button("Open File Dialog").clicked() {
                            if let Some(path) = rfd::FileDialog::new().pick_folder() {
                                // Use the selected path
                                debug!("Selected folder: {:?}", path);
                                self.repo_path = path;
                            }
                        }
                    });
                    // Select analysis type
                    ui.horizontal(|ui| {
                        ui.radio_value(&mut self.analysis_type, 1, "General Code Review");
                        ui.radio_value(&mut self.analysis_type, 2, "Security Review");
                    });

                    // Button to trigger analysis
                    if ui.button("Analyze").clicked() {
                        // Load config
                        // let config = config::Config::load();
                        // provider::assess_code_base(config).await?;
                        info!("Analysing repository... {}", self.repo_path.display());
                    }

                    // Display the analysis result in a scrollable text area
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.label(&self.analysis_result);
                    });
                });
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_parsing() {
        let image_source: egui::ImageSource =
            egui::include_image!("../../assets/img/cosmonaut_logo_trans.png");
        assert_eq!(
            image_source.uri(),
            Some("bytes://../../assets/img/cosmonaut_logo_trans.png")
        );
    }
}
