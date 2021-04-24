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

#[derive(Clone, Copy)]
pub enum EventResult {
    Handled,
    Unhandled,
}

impl EventResult {
    pub fn is_handled(self) -> bool {
        match self {
            EventResult::Handled => true,
            EventResult::Unhandled => false,
        }
    }
}

pub trait Application: 'static + Sized {
    type Message: 'static + Send;
    type Error: Error;

    fn new(
        event_loop: &EventLoop<Self::Message>,
        address: Address<Self::Message>,
    ) -> Result<Self, Self::Error>;

    fn handle_event(
        &mut self,
        event: Event<'_, ()>,
        target: &EventLoopWindowTarget<Self::Message>,
    ) -> Result<ControlFlow, Self::Error>;

    fn handle_message(
        &mut self,
        message: Self::Message,
        target: &EventLoopWindowTarget<Self::Message>,
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
        let _guard = runtime.enter();
        let result = match event.map_nonuser_event::<()>() {
            Ok(event) => application.handle_event(event, target),
            Err(Event::UserEvent(message)) => application.handle_message(message, target),
            Err(_) => panic!("Unexpected error"),
        };
        match result {
            Ok(flow) => *control_flow = flow,
            Err(error) => {
                log::error!("Exiting loop because of error: {}", error);
                *control_flow = ControlFlow::Exit;
            }
        };
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
