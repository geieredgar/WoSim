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
    Color32, CtxRef, Window,
};

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
}

#[derive(Default)]
pub struct DebugWindows {
    pub frame_times: bool,
    pub configuration: bool,
    pub information: bool,
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
        }
    }

    pub fn begin_frame(&mut self) {
        self.frame_start = Instant::now();
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

    fn render_information(&mut self, ctx: &CtxRef, open: &mut bool) {
        Window::new("Information").open(open).show(ctx, |ui| {
            ui.label(format!(
                "FPS: {} Frame Count: {}",
                self.frames_per_second, self.frame_count
            ));
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
                let plot = Plot::default()
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

    pub fn render(&mut self, ctx: &CtxRef, windows: &mut DebugWindows) {
        self.render_configuration(ctx, &mut windows.configuration);
        self.render_information(ctx, &mut windows.information);
        self.render_frame_times(ctx, &mut windows.frame_times);
    }
}
