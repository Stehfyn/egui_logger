use eframe::NativeOptions;

fn main() {
    // Initilize the logger
    egui_logger::init_with_max_level(log::LevelFilter::Debug).expect("Error initializing logger");
    let log_color_map = egui_logger::LogColorMap::new(
        egui::Color32::LIGHT_GREEN,
        egui::Color32::from_rgb(0, 0, 255),   // Blue
        egui::Color32::WHITE,                 // Green
        egui::Color32::from_rgb(255, 165, 0), // Orange
        egui::Color32::from_rgb(255, 0, 0),   // Red
    );

    egui_logger::set_log_color_map(log_color_map);

    let options = NativeOptions::default();

    eframe::run_native(
        "egui_logger",
        options,
        Box::new(|_cc| Box::new(MyApp::default())),
    )
    .unwrap();
}

#[derive(Default)]
struct MyApp;

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if ui.button("This produces Debug Info").clicked() {
                log::debug!("Very verbose Debug Info")
            }
            if ui.button("This produces an Info").clicked() {
                log::info!("Some Info");
            }
            if ui.button("This produces an Error").clicked() {
                log::error!("Error doing Something");
            }
            if ui.button("This produces a Warning").clicked() {
                log::warn!("Warn about something")
            }
        });
        egui::Window::new("Log").title_bar(false).show(ctx, |ui| {
            // draws the actual logger ui
            egui_logger::minimal_logger_ui(ui, egui::Color32::BLACK);
        });
    }
}
