use std::{error::Error, process::exit};

use log::error;
use tokio::runtime::Runtime;
use winit::{
    event::Event,
    event_loop::{ControlFlow, EventLoop},
};

pub trait Application: Sized + 'static {
    type Message: 'static;
    type Error: Error;
    type Args;

    fn new(event_loop: &EventLoop<Self::Message>, args: Self::Args) -> Result<Self, Self::Error>;

    fn handle(&mut self, event: Event<Self::Message>) -> Result<ControlFlow, Self::Error>;

    fn shutdown(&mut self);
}

struct Handler<A: Application> {
    application: A,
    runtime: Runtime,
}

impl<A: Application> Drop for Handler<A> {
    fn drop(&mut self) {
        let _guard = self.runtime.enter();
        self.application.shutdown()
    }
}

pub fn run<A: Application>(runtime: Runtime, args: A::Args) -> ! {
    let event_loop = EventLoop::with_user_event();
    let guard = runtime.enter();
    let application = match A::new(&event_loop, args) {
        Ok(application) => application,
        Err(error) => {
            log::error!("Failed to setup application: {}", error);
            exit(1);
        }
    };
    drop(guard);
    let mut state = Handler {
        application,
        runtime,
    };
    event_loop.run(move |event, _, control_flow| {
        let _guard = state.runtime.enter();
        match state.application.handle(event) {
            Ok(flow) => *control_flow = flow,
            Err(error) => {
                error!("{}", error);
                *control_flow = ControlFlow::Exit;
            }
        }
    })
}
