use std::{
    cell::UnsafeCell,
    error::Error,
    fmt::{self, Display, Formatter},
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use futures::Future;
use tokio::sync::Semaphore;
use tokio_util::sync::PollSemaphore;

pub struct Sender<T> {
    value: Option<Arc<UnsafeCell<Option<T>>>>,
    semaphore: Arc<Semaphore>,
}

#[derive(Clone)]
pub struct Receiver<T> {
    value: Arc<UnsafeCell<Option<T>>>,
    semaphore: PollSemaphore,
}
#[derive(Debug, Eq, PartialEq)]
pub struct RecvError;

pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
    let value = Arc::new(UnsafeCell::new(None));
    let semaphore = Arc::new(Semaphore::new(0));
    (
        Sender {
            value: Some(value.clone()),
            semaphore: semaphore.clone(),
        },
        Receiver {
            value,
            semaphore: PollSemaphore::new(semaphore),
        },
    )
}

impl<T> Sender<T> {
    pub fn send(mut self, value: T) {
        *unsafe { self.value.take().unwrap().get().as_mut() }.unwrap() = Some(value);
        self.semaphore.add_permits(1)
    }
}

impl<T> Drop for Sender<T> {
    fn drop(&mut self) {
        if self.value.is_some() {
            self.semaphore.close()
        }
    }
}

impl<T: Clone> Future for Receiver<T> {
    type Output = Result<T, RecvError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.semaphore.poll_acquire(cx) {
            Poll::Ready(permit) => match permit {
                Some(_) => {
                    let value = unsafe { self.value.get().as_ref() }
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .clone();
                    Poll::Ready(Ok(value))
                }
                None => Poll::Ready(Err(RecvError)),
            },
            Poll::Pending => Poll::Pending,
        }
    }
}

impl Display for RecvError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "channel closed")
    }
}

impl Error for RecvError {}
