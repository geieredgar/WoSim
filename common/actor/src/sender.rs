use std::marker::PhantomData;

use log::warn;
use tokio::sync::mpsc::UnboundedSender;

use crate::Address;

pub trait Sender<T>: Send + Sync + 'static {
    fn send(&self, message: T);
}

impl<T: Send + 'static> Sender<T> for UnboundedSender<T> {
    fn send(&self, message: T) {
        match self.send(message) {
            Ok(()) => {}
            Err(error) => {
                warn!("Sending failed. Receiver already closed. Error: {}", error)
            }
        }
    }
}

pub(super) struct MapSender<M: 'static, N, F: Fn(N) -> M> {
    address: Address<M>,
    transform: F,
    _phantom: PhantomData<N>,
}

impl<M: 'static, N, F: Fn(N) -> M> MapSender<M, N, F> {
    pub(super) fn new(address: Address<M>, transform: F) -> Self {
        Self {
            address,
            transform,
            _phantom: PhantomData,
        }
    }
}

impl<M: 'static, N: Send + Sync + 'static, F: Send + Sync + 'static + Fn(N) -> M> Sender<N>
    for MapSender<M, N, F>
{
    fn send(&self, message: N) {
        self.address.send((self.transform)(message))
    }
}

pub(super) struct FilterMapSender<M: 'static, N, F: Fn(N) -> Option<M>> {
    address: Address<M>,
    transform: F,
    _phantom: PhantomData<N>,
}

impl<M: 'static, N, F: Fn(N) -> Option<M>> FilterMapSender<M, N, F> {
    pub(super) fn new(address: Address<M>, transform: F) -> Self {
        Self {
            address,
            transform,
            _phantom: PhantomData,
        }
    }
}

impl<M: 'static, N: Send + Sync + 'static, F: Send + Sync + 'static + Fn(N) -> Option<M>> Sender<N>
    for FilterMapSender<M, N, F>
{
    fn send(&self, message: N) {
        if let Some(message) = (self.transform)(message) {
            self.address.send(message)
        }
    }
}
