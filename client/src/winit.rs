use std::{
    error::Error,
    process::exit,
    sync::{Arc, Mutex},
};

use actor::{Address, Sender};
use log::warn;
use tokio::runtime::Runtime;
use winit::{
    dpi::PhysicalSize,
    event::WindowEvent,
    event_loop::{ControlFlow, EventLoopProxy},
    window::WindowId,
};

pub type EventLoop = winit::event_loop::EventLoop<Request>;

pub type Event = winit::event::Event<'static, UserEvent>;

type InternalEvent<'a> = winit::event::Event<'a, Request>;

#[derive(Debug)]
pub enum Request {
    Exit,
}

#[derive(Debug)]
pub enum UserEvent {
    ScaleFactorChanged {
        window_id: WindowId,
        scale_factor: f64,
        new_inner_size: PhysicalSize<u32>,
    },
}

pub fn run<F: FnOnce(&EventLoop, Address<Request>) -> Result<Address<Event>, E>, E: Error>(
    runtime: Runtime,
    factory: F,
) -> ! {
    let event_loop = EventLoop::with_user_event();
    let address = Address::new(Arc::new(EventLoopSender(Mutex::new(
        event_loop.create_proxy(),
    ))));
    let guard = runtime.enter();
    let address = match factory(&event_loop, address) {
        Ok(address) => address,
        Err(error) => {
            log::error!("Failed to setup application: {}", error);
            exit(1);
        }
    };
    drop(guard);
    event_loop.run(move |event, _, control_flow| {
        let _guard = runtime.enter();
        if let InternalEvent::UserEvent(Request::Exit) = &event {
            *control_flow = ControlFlow::Exit;
            return;
        }
        if let Some(event) = map(event) {
            address.send(event);
        }
        *control_flow = ControlFlow::Wait
    })
}

fn map(event: InternalEvent<'_>) -> Option<Event> {
    if let InternalEvent::WindowEvent {
        window_id,
        event:
            WindowEvent::ScaleFactorChanged {
                scale_factor,
                new_inner_size,
            },
    } = &event
    {
        return Some(Event::UserEvent(UserEvent::ScaleFactorChanged {
            window_id: *window_id,
            scale_factor: *scale_factor,
            new_inner_size: **new_inner_size,
        }));
    }
    event.to_static()?.map_nonuser_event().ok()
}

struct EventLoopSender(Mutex<EventLoopProxy<Request>>);

impl Sender<Request> for EventLoopSender {
    fn send(&self, message: Request) {
        if let Err(error) = self.0.lock().unwrap().send_event(message) {
            warn!(
                "Sending failed. Event loop already closed. Error: {}",
                error
            );
        }
    }
}
