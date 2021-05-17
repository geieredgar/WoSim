use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

use actor::Address;
use egui::{
    widgets::{
        plot::{Curve, Plot, Value},
        Slider,
    },
    Align, Color32, CtxRef, DragValue, RadioButton, ScrollArea, Window,
};
use log::{set_max_level, Level, LevelFilter};
use server::Connection;

use crate::{renderer::RenderTimestamps, ApplicationMessage};

pub struct DebugContext {
    address: Address<ApplicationMessage>,
    frame_count: usize,
    frames_per_second: usize,
    last_frame_count: usize,
    last_second: Instant,
    frame_start: Instant,
    frame_times: VecDeque<(Instant, Instant, Option<RenderTimestamps>)>,
    frame_times_secs: f64,
    show_cpu_time: bool,
    show_gpu_time: bool,
    server_address: String,
    username: String,
    connected: bool,
    log_records: VecDeque<(Level, String, String)>,
    log_scroll_to_bottom: bool,
    log_limit_entries: bool,
    log_entry_limit: usize,
}

#[derive(Default)]
pub struct DebugWindows {
    pub frame_times: bool,
    pub configuration: bool,
    pub information: bool,
    pub log: bool,
}

impl DebugContext {
    pub fn new(address: Address<ApplicationMessage>) -> Self {
        Self {
            address,
            frame_count: 0,
            frames_per_second: 0,
            last_frame_count: 0,
            last_second: Instant::now(),
            frame_start: Instant::now(),
            frame_times: VecDeque::new(),
            frame_times_secs: 10.0,
            show_cpu_time: true,
            show_gpu_time: true,
            server_address: "localhost".to_owned(),
            username: "anonymous".to_owned(),
            connected: false,
            log_records: VecDeque::new(),
            log_scroll_to_bottom: true,
            log_limit_entries: false,
            log_entry_limit: 20,
        }
    }

    pub fn begin_frame(&mut self) {
        self.frame_start = Instant::now();
    }

    pub fn log(&mut self, level: Level, target: String, args: String) {
        self.log_records.push_back((level, target, args));
        if self.log_limit_entries && self.log_entry_limit < self.log_records.len() {
            self.log_records.pop_front();
        }
    }

    pub fn end_frame(&mut self, last_timestamps: Option<RenderTimestamps>) {
        self.frame_count += 1;
        let now = Instant::now();
        if now.duration_since(self.last_second).as_secs() >= 1 {
            self.frames_per_second = self.frame_count - self.last_frame_count;
            self.last_frame_count = self.frame_count;
            self.last_second += Duration::from_secs(1);
        }
        if let Some((_, _, timestamps)) = self.frame_times.back_mut() {
            *timestamps = last_timestamps;
        }
        self.frame_times.push_back((self.frame_start, now, None));
        while let Some(front) = self.frame_times.front() {
            if now.duration_since(front.1).as_secs_f64() > self.frame_times_secs {
                self.frame_times.pop_front();
            } else {
                break;
            }
        }
    }

    pub fn connection_status_changed(&mut self, connected: bool) {
        self.connected = connected;
    }

    fn render_configuration(&mut self, ctx: &CtxRef, open: &mut bool) {
        Window::new("Configuration").open(open).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.add(::egui::TextEdit::singleline(&mut self.server_address));
                ui.add(::egui::TextEdit::singleline(&mut self.username));
                if !self.connected {
                    if ui.button("Connect").clicked() {
                        self.address.send(ApplicationMessage::Connect {
                            address: self.server_address.clone(),
                            username: self.username.clone(),
                        });
                    }
                } else if ui.button("Disconnect").clicked() {
                    self.address.send(ApplicationMessage::Disconnect);
                }
            });
        });
    }

    fn render_information(
        &mut self,
        ctx: &CtxRef,
        open: &mut bool,
        connection: Option<&Connection>,
    ) {
        Window::new("Information").open(open).show(ctx, |ui| {
            ui.label(format!(
                "FPS: {} Frame Count: {}",
                self.frames_per_second, self.frame_count
            ));
            if let Some(connection) = connection {
                ui.label(format!("RTT: {}", connection.rtt().as_millis()));
            }
        });
    }

    fn render_frame_times(&mut self, ctx: &CtxRef, open: &mut bool) {
        Window::new("Frame times").open(open).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Frame time storage duration [s]");
                ui.add(Slider::new(&mut self.frame_times_secs, 0.0f64..=120.0f64));
            });
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.show_cpu_time, "Show CPU Time");
                ui.checkbox(&mut self.show_gpu_time, "Show GPU Time");
            });
            if let Some(front) = self.frame_times.front() {
                let origin = front.0;
                let plot = Plot::new("frame_times")
                    .include_x(0.0)
                    .include_x(self.frame_times_secs)
                    .include_y(0.0);
                let plot = if self.show_cpu_time {
                    plot.curve(
                        Curve::from_values_iter(self.frame_times.iter().map(|(begin, end, _)| {
                            Value {
                                x: begin.duration_since(origin).as_secs_f64(),
                                y: end.duration_since(*begin).as_secs_f64() * 1000.0,
                            }
                        }))
                        .color(Color32::RED)
                        .name("CPU Time"),
                    )
                } else {
                    plot
                };
                let plot = if self.show_gpu_time {
                    plot.curve(
                        Curve::from_values_iter(self.frame_times.iter().filter_map(
                            |(begin, _, timestamps)| {
                                timestamps.as_ref().map(|timestamps| Value {
                                    x: begin.duration_since(origin).as_secs_f64(),
                                    y: timestamps.end - timestamps.begin,
                                })
                            },
                        ))
                        .color(Color32::BLUE)
                        .name("GPU Time"),
                    )
                } else {
                    plot
                };
                ui.add(plot);
            }
        });
    }

    fn level_color(level: Level) -> Color32 {
        match level {
            Level::Error => Color32::RED,
            Level::Warn => Color32::YELLOW,
            Level::Info => Color32::GREEN,
            Level::Debug => Color32::BLUE,
            Level::Trace => Color32::GRAY,
        }
    }

    fn level_text(level: Level) -> &'static str {
        match level {
            Level::Error => "ERROR",
            Level::Warn => "WARN",
            Level::Info => "INFO",
            Level::Debug => "DEBUG",
            Level::Trace => "TRACE",
        }
    }

    fn render_log(&mut self, ctx: &CtxRef, open: &mut bool) {
        Window::new("Log").open(open).show(ctx, |ui| {
            ui.horizontal(|ui| {
                let level = log::max_level();
                if ui
                    .add(RadioButton::new(level == LevelFilter::Off, "Off"))
                    .clicked()
                {
                    set_max_level(LevelFilter::Off);
                }
                if ui
                    .add(RadioButton::new(level == LevelFilter::Error, "Error"))
                    .clicked()
                {
                    set_max_level(LevelFilter::Error);
                }
                if ui
                    .add(RadioButton::new(level == LevelFilter::Warn, "Warn"))
                    .clicked()
                {
                    set_max_level(LevelFilter::Warn);
                }
                if ui
                    .add(RadioButton::new(level == LevelFilter::Info, "Info"))
                    .clicked()
                {
                    set_max_level(LevelFilter::Info);
                }
                if ui
                    .add(RadioButton::new(level == LevelFilter::Debug, "Debug"))
                    .clicked()
                {
                    set_max_level(LevelFilter::Debug);
                }
                if ui
                    .add(RadioButton::new(level == LevelFilter::Trace, "Trace"))
                    .clicked()
                {
                    set_max_level(LevelFilter::Trace);
                }
                ui.checkbox(&mut self.log_scroll_to_bottom, "Scroll to bottom");
                ui.checkbox(&mut self.log_limit_entries, "Limit log entries");
                if self.log_limit_entries {
                    ui.add(
                        DragValue::new(&mut self.log_entry_limit)
                            .speed(1)
                            .prefix("Entry limit: "),
                    );
                    while self.log_records.len() > self.log_entry_limit {
                        self.log_records.pop_front();
                    }
                }
                if ui.button("Clear log").clicked() {
                    self.log_records.clear();
                }
            });
            ScrollArea::from_max_height(600.0).show(ui, |ui| {
                for (level, target, args) in self.log_records.iter() {
                    ui.horizontal_wrapped(|ui| {
                        ui.colored_label(Self::level_color(*level), Self::level_text(*level));
                        ui.label(target);
                        ui.label(args);
                    });
                }
                if self.log_scroll_to_bottom {
                    ui.scroll_to_cursor(Align::BOTTOM);
                }
            });
        });
    }

    pub fn render(
        &mut self,
        ctx: &CtxRef,
        windows: &mut DebugWindows,
        connection: Option<&Connection>,
    ) {
        self.render_configuration(ctx, &mut windows.configuration);
        self.render_information(ctx, &mut windows.information, connection);
        self.render_frame_times(ctx, &mut windows.frame_times);
        self.render_log(ctx, &mut windows.log);
    }
}
