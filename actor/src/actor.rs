use std::future::Future;

use tokio::spawn;

use crate::{Address, Mailbox};

pub fn async_actor<H, M, F>(handler: H) -> Address<M>
where
    H: Fn(M) -> F + Sync + Send + 'static,
    M: Send + Sync + 'static,
    F: Future<Output = ()> + Send + 'static,
{
    Address::new(move |message| {
        spawn(handler(message));
        Ok(())
    })
}

pub struct Actor<H, M>
where
    H: Send + Sync + 'static + FnMut(M) -> ControlFlow,
    M: Send + Sync + 'static,
{
    mailbox: Mailbox<M>,
    handler: H,
}

impl<H, M> Actor<H, M>
where
    H: Send + Sync + 'static + FnMut(M) -> ControlFlow,
    M: Send + Sync + 'static,
{
    pub fn new(mailbox: Mailbox<M>, handler: H) -> Self {
        Self { mailbox, handler }
    }

    pub async fn run(&mut self) {
        while let Some(message) = self.mailbox.recv().await {
            if let ControlFlow::Stop = (self.handler)(message) {
                return;
            }
        }
    }
}

pub enum ControlFlow {
    Continue,
    Stop,
}
