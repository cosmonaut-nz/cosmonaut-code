//! 
//! 
//!
//! 
//!
// src/ui/mod.rs
use eframe::{egui, Frame, NativeOptions, run_native, App};

const APP_TITLE: &str = ">_ cosmonaut code reviewer";

// Refactored
struct MyAppUI {
    repo_path: String,
    analysis_type: i32, // 1 for general code review, 2 for security review
    analysis_result: String,
}

impl MyAppUI {
    pub fn new() -> Self {
        Self { 
            repo_path: String::new(),
            analysis_type: 1, // Default to general code review
            analysis_result: String::new(),
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        // Input text box for repository path
        ui.horizontal(|ui| {
            ui.label("Repository Path:");
            ui.text_edit_singleline(&mut self.repo_path);
        });

        // Select analysis type
        ui.horizontal(|ui| {
            ui.radio_value(&mut self.analysis_type, 1, "General Code Review");
            ui.radio_value(&mut self.analysis_type, 2, "Security Review");
        });

        // Button to trigger analysis
        if ui.button("Analyze").clicked() {
            // Call your analysis function here, e.g.:
            // self.analysis_result = analyze_code(&self.repo_path, self.analysis_type);
        }

        // Display the analysis result in a scrollable text area
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.label(&self.analysis_result);
        });
    }
}


// Orig
// impl MyAppUI {
//     pub fn new() -> Self {
//         Self { counter: 0 }
//     }

//     pub fn ui(&mut self, ui: &mut egui::Ui) {
//         ui.horizontal(|ui| {
//             if ui.button("−").clicked() {
//                 self.counter -= 1;
//             }
//             ui.label(self.counter.to_string());
//             if ui.button("+").clicked() {
//                 self.counter += 1;
//             }
//         });
//     }
// }

pub fn run() {
    let options: NativeOptions = NativeOptions::default();
    let _ = run_native(
        APP_TITLE,
        options,
        Box::new(|_cc: &eframe::CreationContext<'_>| Box::new(MyApp { ui: MyAppUI::new() })),
    );
}

struct MyApp {
    ui: MyAppUI,
}

impl App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            self.ui.ui(ui);
        });
    }
}

