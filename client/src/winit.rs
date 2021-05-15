use std::{error::Error, process::exit, sync::Mutex};

use actor::Address;
use tokio::runtime::Runtime;
use winit::{dpi::PhysicalSize, event::WindowEvent, event_loop::ControlFlow, window::WindowId};

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
    let proxy = Mutex::new(event_loop.create_proxy());
    let address = Address::new(move |message| {
        proxy
            .lock()
            .unwrap()
            .send_event(message)
            .map_err(|e| e.into())
    });
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
