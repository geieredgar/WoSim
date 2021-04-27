use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use crate::{Mailbox, TaskQueue};

pub struct Actor<H: Fn(M) -> T, M, T: Future<Output = ()>> {
    mailbox: Mailbox<M>,
    handler: H,
    queue: TaskQueue<T>,
    closed: bool,
}

impl<
        H: Send + Sync + 'static + Fn(M) -> T,
        M: Send + Sync + 'static,
        T: Future<Output = ()> + Send + Sync + 'static,
    > Actor<H, M, T>
{
    pub fn new(mailbox: Mailbox<M>, handler: H) -> Self {
        Self {
            mailbox,
            handler,
            queue: TaskQueue::new(),
            closed: false,
        }
    }
}

impl<H: Fn(M) -> T, M, T: Future<Output = ()>> Future for Actor<H, M, T> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        let mut pending = match Pin::new(&mut this.queue).poll(cx) {
            Poll::Ready(()) => false,
            Poll::Pending => true,
        };
        loop {
            if !this.closed {
                match this.mailbox.poll_recv(cx) {
                    Poll::Ready(next) => {
                        if let Some(message) = next {
                            match this.queue.push((this.handler)(message)) {
                                Poll::Ready(()) => {}
                                Poll::Pending => pending = true,
                            }
                        } else {
                            this.closed = true;
                        }
                    }
                    Poll::Pending => break Poll::Pending,
                }
            } else if pending {
                break Poll::Pending;
            } else {
                break Poll::Ready(());
            }
        }
    }
}
