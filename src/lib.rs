use std::sync::Mutex;

use egui::{Color32, Vec2};
use log::SetLoggerError;

use regex::{Regex, RegexBuilder};

const LEVELS: [log::Level; log::Level::Trace as usize] = [
    log::Level::Error,
    log::Level::Warn,
    log::Level::Info,
    log::Level::Debug,
    log::Level::Trace,
];

struct EguiLogger;

impl log::Log for EguiLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::STATIC_MAX_LEVEL
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            try_mut_log(|logs| logs.push((record.level(), record.args().to_string())));
        }
    }

    fn flush(&self) {}
}

pub struct LogColorMap {
    trace: egui::Color32,
    debug: egui::Color32,
    info: egui::Color32,
    warn: egui::Color32,
    error: egui::Color32,
}

impl LogColorMap {
    pub fn new(
        trace: egui::Color32,
        debug: egui::Color32,
        info: egui::Color32,
        warn: egui::Color32,
        error: egui::Color32,
    ) -> Self {
        Self {
            trace,
            debug,
            info,
            warn,
            error,
        }
    }

    pub fn default() -> Self {
        Self {
            trace: egui::Color32::from_gray(180),       // Light Gray
            debug: egui::Color32::from_rgb(0, 0, 255),  // Blue
            info: egui::Color32::from_rgb(0, 255, 0),   // Green
            warn: egui::Color32::from_rgb(255, 165, 0), // Orange
            error: egui::Color32::from_rgb(255, 0, 0),  // Red
        }
    }

    fn get_color(&self, level: &log::Level) -> egui::Color32 {
        match level {
            log::Level::Trace => self.trace,
            log::Level::Debug => self.debug,
            log::Level::Info => self.info,
            log::Level::Warn => self.warn,
            log::Level::Error => self.error,
        }
    }
}

/// Initilizes the global logger.
/// Should be called very early in the program.
/// Defaults to max level Info.
pub fn init() -> Result<(), SetLoggerError> {
    log::set_logger(&EguiLogger).map(|()| log::set_max_level(log::LevelFilter::Info))
}

/// Same as [`init()`] accepts a [`log::LevelFilter`] to set the max level
/// use [`Trace`](log::LevelFilter::Trace) with caution
pub fn init_with_max_level(max_level: log::LevelFilter) -> Result<(), SetLoggerError> {
    log::set_logger(&EguiLogger).map(|()| log::set_max_level(max_level))
}

type GlobalLog = Vec<(log::Level, String)>;

static LOG: Mutex<GlobalLog> = Mutex::new(Vec::new());

static LOGGER_UI: once_cell::sync::Lazy<Mutex<LoggerUi>> =
    once_cell::sync::Lazy::new(Default::default);

fn try_mut_log<F, T>(f: F) -> Option<T>
where
    F: FnOnce(&mut GlobalLog) -> T,
{
    match LOG.lock() {
        Ok(ref mut global_log) => Some((f)(global_log)),
        Err(_) => None,
    }
}

fn try_get_log<F, T>(f: F) -> Option<T>
where
    F: FnOnce(&GlobalLog) -> T,
{
    match LOG.lock() {
        Ok(ref global_log) => Some((f)(global_log)),
        Err(_) => None,
    }
}

struct MinLoggerUiStats {
    content_size: Vec2,
    scroll_offset: Vec2,
    visible_size: Vec2,
}

impl Default for MinLoggerUiStats {
    fn default() -> Self {
        Self {
            content_size: egui::Vec2::ZERO,
            scroll_offset: egui::Vec2::ZERO,
            visible_size: egui::Vec2::ZERO,
        }
    }
}
struct LoggerUi {
    loglevels: [bool; log::Level::Trace as usize],
    search_term: String,
    regex: Option<Regex>,
    search_case_sensitive: bool,
    search_use_regex: bool,
    max_log_length: usize,
    min_logger_ui_stats: MinLoggerUiStats,
    log_color_map: LogColorMap,
}

impl Default for LoggerUi {
    fn default() -> Self {
        Self {
            loglevels: [true, true, true, false, false],
            search_term: String::new(),
            search_case_sensitive: false,
            regex: None,
            search_use_regex: false,
            max_log_length: 1000,
            min_logger_ui_stats: MinLoggerUiStats::default(),
            log_color_map: LogColorMap::default(),
        }
    }
}

impl LoggerUi {
    pub fn set_log_color_map(&mut self, log_color_map: LogColorMap) {
        self.log_color_map = log_color_map;
    }

    pub fn minimal_ui(&mut self, ui: &mut egui::Ui, bg_col: egui::Color32) {
        let mut logs_displayed: usize = 0;

        let scroll_area = egui::ScrollArea::vertical()
            .auto_shrink([false, true])
            .max_height(ui.available_height() - 30.0)
            .stick_to_bottom(true)
            .drag_to_scroll(false)
            .show(ui, |ui| {
                try_get_log(|logs| {
                    let mut rect = ui.available_rect_before_wrap();
                    rect.min.y += self.min_logger_ui_stats.scroll_offset.y;
                    rect.max.y += self.min_logger_ui_stats.scroll_offset.y;
                    let painter = egui::Painter::new(ui.ctx().clone(), ui.layer_id(), rect);
                    painter.rect_filled(rect, egui::Rounding::default(), bg_col);
                    ui.expand_to_include_rect(painter.clip_rect());
                    logs.iter().for_each(|(level, string)| {
                        if (!self.search_term.is_empty() && !self.match_string(string))
                            || !(self.loglevels[*level as usize - 1])
                        {
                            return;
                        }

                        let string_format = format!("[{}]: {}", level, string);
                        ui.colored_label(self.log_color_map.get_color(level), string_format);
                        logs_displayed += 1;
                    });
                });
            });
        self.min_logger_ui_stats.content_size = scroll_area.content_size;
        self.min_logger_ui_stats.scroll_offset = scroll_area.state.offset;
        self.min_logger_ui_stats.visible_size = egui::Vec2 {
            x: scroll_area.inner_rect.width(),
            y: scroll_area.inner_rect.height(),
        };
    }
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        try_mut_log(|logs| {
            let dropped_entries = logs.len().saturating_sub(self.max_log_length);
            drop(logs.drain(..dropped_entries));
        });

        ui.horizontal(|ui| {
            if ui.button("Clear").clicked() {
                try_mut_log(|logs| logs.clear());
            }
            ui.menu_button("Log Levels", |ui| {
                for level in LEVELS {
                    if ui
                        .selectable_label(self.loglevels[level as usize - 1], level.as_str())
                        .clicked()
                    {
                        self.loglevels[level as usize - 1] = !self.loglevels[level as usize - 1];
                    }
                }
            });
        });

        ui.horizontal(|ui| {
            ui.label("Search: ");
            let response = ui.text_edit_singleline(&mut self.search_term);

            let mut config_changed = false;

            if ui
                .selectable_label(self.search_case_sensitive, "Aa")
                .on_hover_text("Case sensitive")
                .clicked()
            {
                self.search_case_sensitive = !self.search_case_sensitive;
                config_changed = true;
            }

            if ui
                .selectable_label(self.search_use_regex, ".*")
                .on_hover_text("Use regex")
                .clicked()
            {
                self.search_use_regex = !self.search_use_regex;
                config_changed = true;
            }

            if self.search_use_regex && (response.changed() || config_changed) {
                self.regex = RegexBuilder::new(&self.search_term)
                    .case_insensitive(!self.search_case_sensitive)
                    .build()
                    .ok()
            }
        });

        ui.horizontal(|ui| {
            ui.label("Max Log output");
            ui.add(egui::widgets::DragValue::new(&mut self.max_log_length).speed(1));
        });

        ui.horizontal(|ui| {
            if ui.button("Sort").clicked() {
                try_mut_log(|logs| logs.sort());
            }
        });

        ui.separator();

        let mut logs_displayed: usize = 0;

        egui::ScrollArea::vertical()
            .auto_shrink([false, true])
            .max_height(ui.available_height() - 30.0)
            .stick_to_bottom(true)
            .show(ui, |ui| {
                try_get_log(|logs| {
                    logs.iter().for_each(|(level, string)| {
                        if (!self.search_term.is_empty() && !self.match_string(string))
                            || !(self.loglevels[*level as usize - 1])
                        {
                            return;
                        }

                        let string_format = format!("[{}]: {}", level, string);

                        match level {
                            log::Level::Warn => ui.colored_label(Color32::YELLOW, string_format),
                            log::Level::Error => ui.colored_label(Color32::RED, string_format),
                            _ => ui.label(string_format),
                        };

                        logs_displayed += 1;
                    });
                });
            });

        ui.horizontal(|ui| {
            ui.label(format!(
                "Log size: {}",
                try_get_log(|logs| logs.len()).unwrap_or_default()
            ));
            ui.label(format!("Displayed: {}", logs_displayed));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Copy").clicked() {
                    ui.output_mut(|o| {
                        try_get_log(|logs| {
                            let mut out_string = String::new();
                            logs.iter()
                                .take(self.max_log_length)
                                .for_each(|(_, string)| {
                                    out_string.push_str(string);
                                    out_string.push_str(" \n");
                                });
                            o.copied_text = out_string;
                        });
                    });
                }
            });
        });
    }

    fn match_string(&self, string: &str) -> bool {
        if self.search_use_regex {
            if let Some(matcher) = &self.regex {
                matcher.is_match(string)
            } else {
                false
            }
        } else if self.search_case_sensitive {
            string.contains(&self.search_term)
        } else {
            string
                .to_lowercase()
                .contains(&self.search_term.to_lowercase())
        }
    }
}

/// Draws the logger ui
/// has to be called after [`init()`];
pub fn logger_ui(ui: &mut egui::Ui) {
    if let Ok(ref mut logger_ui) = LOGGER_UI.lock() {
        logger_ui.ui(ui);
    } else {
        ui.colored_label(Color32::RED, "Something went wrong loading the log");
    }
}
pub fn minimal_logger_ui(ui: &mut egui::Ui, bg_col: egui::Color32) {
    if let Ok(ref mut logger_ui) = LOGGER_UI.lock() {
        logger_ui.minimal_ui(ui, bg_col);
    } else {
        ui.colored_label(Color32::RED, "Something went wrong loading the log");
    }
}
pub fn set_log_color_map(log_color_map: LogColorMap) {
    if let Ok(ref mut logger_ui) = LOGGER_UI.lock() {
        logger_ui.set_log_color_map(log_color_map);
    }
}
