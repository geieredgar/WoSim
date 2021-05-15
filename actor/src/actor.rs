use std::{future::Future, marker::PhantomData, sync::Arc};

use tokio::spawn;

use crate::{Address, Mailbox, Sender};

pub struct AsyncActor<H, M>
where
    H: Fn(M) + Send + Sync + 'static,
    M: Send + Sync + 'static,
    H::Output: Future<Output = ()>,
{
    handler: H,
    _phantom: PhantomData<M>,
}

impl<H, M> AsyncActor<H, M>
where
    H: Fn(M) + Send + Sync + 'static,
    M: Send + Sync + 'static,
    H::Output: Future<Output = ()>,
{
    pub fn address(handler: H) -> Address<M> {
        Address::new(Arc::new(Self {
            handler,
            _phantom: PhantomData,
        }))
    }
}

impl<H, M> Sender<M> for AsyncActor<H, M>
where
    H: Fn(M) + Send + Sync + 'static,
    M: Send + Sync + 'static,
    H::Output: Future<Output = ()>,
{
    fn send(&self, message: M) {
        #[allow(clippy::unit_arg)]
        spawn((self.handler)(message));
    }
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
