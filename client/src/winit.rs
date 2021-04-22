use std::{
    error::Error,
    process::exit,
    sync::{Arc, Mutex},
};

use actor::{Address, Sender};
use log::warn;
use tokio::runtime::Runtime;
use winit::{
    event::Event,
    event_loop::{ControlFlow, EventLoop, EventLoopProxy, EventLoopWindowTarget},
};

pub trait Application: 'static + Sized {
    type Event: 'static + Send;
    type Error: Error;

    fn new(
        event_loop: &EventLoop<Self::Event>,
        address: Address<Self::Event>,
    ) -> Result<Self, Self::Error>;

    fn handle(
        &mut self,
        event: Event<'_, Self::Event>,
        target: &EventLoopWindowTarget<Self::Event>,
    ) -> Result<ControlFlow, Self::Error>;
}

pub fn run<T: Application>(runtime: Runtime) -> ! {
    let event_loop = EventLoop::with_user_event();
    let address = Address::new(Arc::new(EventLoopSender(Mutex::new(
        event_loop.create_proxy(),
    ))));
    let guard = runtime.enter();
    let mut application = match T::new(&event_loop, address) {
        Ok(application) => application,
        Err(error) => {
            log::error!("Failed to setup application: {}", error);
            exit(1);
        }
    };
    drop(guard);
    event_loop.run(move |event, target, control_flow| {
        let guard = runtime.enter();
        match application.handle(event, target) {
            Ok(flow) => *control_flow = flow,
            Err(error) => {
                log::error!("Exiting loop because of error: {}", error);
                *control_flow = ControlFlow::Exit;
            }
        };
        drop(guard)
    })
}

struct EventLoopSender<T: 'static>(Mutex<EventLoopProxy<T>>);

impl<T: 'static + Send> Sender<T> for EventLoopSender<T> {
    fn send(&self, message: T) {
        if let Err(error) = self.0.lock().unwrap().send_event(message) {
            warn!(
                "Sending failed. Event loop already closed. Error: {}",
                error
            );
        }
    }
}
