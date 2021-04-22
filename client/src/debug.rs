use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

use egui::{
    widgets::{
        plot::{Curve, Plot, Value},
        Slider,
    },
    Color32, CtxRef, Window,
};

use crate::renderer::RenderTimestamps;

pub struct DebugContext {
    frame_count: usize,
    frames_per_second: usize,
    last_frame_count: usize,
    last_second: Instant,
    frame_start: Instant,
    frame_times: VecDeque<(Instant, Instant, Option<RenderTimestamps>)>,
    frame_times_secs: f64,
    show_cpu_time: bool,
    show_gpu_time: bool,
    pub enabled: bool,
    pub rotate_cubes: bool,
}

impl DebugContext {
    pub fn new() -> Self {
        Self {
            frame_count: 0,
            frames_per_second: 0,
            last_frame_count: 0,
            last_second: Instant::now(),
            frame_start: Instant::now(),
            frame_times: VecDeque::new(),
            frame_times_secs: 10.0,
            show_cpu_time: true,
            show_gpu_time: true,
            enabled: false,
            rotate_cubes: false,
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

    pub fn render(&mut self, ctx: &CtxRef) {
        if !self.enabled {
            return;
        }
        Window::new("Debug info").show(ctx, |ui| {
            ui.checkbox(&mut self.rotate_cubes, "Rotate cubes");
            ui.label(format!(
                "FPS: {} Frame Count: {}",
                self.frames_per_second, self.frame_count
            ));
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
}
