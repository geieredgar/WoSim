use std::{
    alloc::{alloc, dealloc, Layout},
    future::Future,
    mem::MaybeUninit,
    pin::Pin,
    ptr::{self, NonNull},
    sync::{
        atomic::{self, AtomicU64, AtomicUsize, Ordering},
        Arc,
    },
    task::{Context, Poll, RawWaker, RawWakerVTable, Waker},
};

use futures::task::AtomicWaker;

pub(super) struct TaskQueue<T: Future<Output = ()>>(TaskBlockRef<T>, Arc<AtomicWaker>);

impl<T: Future<Output = ()>> TaskQueue<T> {
    pub fn new() -> Self {
        let shared_waker = Arc::new(AtomicWaker::new());
        Self(
            unsafe { TaskBlockRef::new(shared_waker.clone()) },
            shared_waker,
        )
    }

    pub fn push(&mut self, task: T) -> Poll<()> {
        unsafe { self.0.push(task) }
    }
}

impl<T: Future<Output = ()>> Unpin for TaskQueue<T> {}

impl<T: Future<Output = ()>> Future for TaskQueue<T> {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.1.register(cx.waker());
        unsafe { self.0.poll() }
    }
}

struct TaskBlock<T: Future<Output = ()>> {
    tasks: [MaybeUninit<T>; 64],
    wakers: [TaskBlockWaker<T>; 64],
    shared_waker: Arc<AtomicWaker>,
    pending: u64,
    poll: AtomicU64,
    ref_count: AtomicUsize,
    next: Option<TaskBlockRef<T>>,
}

impl<T: Future<Output = ()>> TaskBlock<T> {
    unsafe fn poll(&mut self) -> Poll<()> {
        let mut poll = 0;
        loop {
            match self
                .poll
                .compare_exchange_weak(poll, 0, Ordering::SeqCst, Ordering::Relaxed)
            {
                Ok(_) => break,
                Err(current) => poll = current,
            }
        }
        while poll != 0 {
            let index = poll.leading_zeros() as usize;
            let mask = 1 << (63 - index);
            if self.pending & mask != 0 {
                let task = Pin::new_unchecked(&mut *self.tasks[index].as_mut_ptr());
                let waker = Waker::from_raw(self.wakers[index].raw_waker());
                let mut context = Context::from_waker(&waker);
                match task.poll(&mut context) {
                    Poll::Ready(()) => {
                        ptr::drop_in_place(self.tasks[index].as_mut_ptr());
                        self.pending ^= mask;
                    }
                    Poll::Pending => {}
                }
            }
            poll ^= mask;
        }
        if let Some(next) = self.next.as_mut() {
            match next.poll() {
                Poll::Ready(()) => self.next = None,
                Poll::Pending => return Poll::Pending,
            }
        }
        if self.pending == 0 {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }

    unsafe fn push(&mut self, task: T) -> Poll<()> {
        let index = self.pending.leading_ones() as usize;
        if index < 64 {
            self.tasks[index] = MaybeUninit::new(task);
            let task = Pin::new_unchecked(&mut *self.tasks[index].as_mut_ptr());
            let waker = Waker::from_raw(self.wakers[index].raw_waker());
            let mut context = Context::from_waker(&waker);
            match task.poll(&mut context) {
                Poll::Ready(()) => {
                    ptr::drop_in_place(self.tasks[index].as_mut_ptr());
                    Poll::Ready(())
                }
                Poll::Pending => {
                    self.pending ^= 1 << (63 - index);
                    Poll::Pending
                }
            }
        } else if let Some(next) = self.next.as_mut() {
            next.push(task)
        } else {
            let mut next = TaskBlockRef::new(self.shared_waker.clone());
            let result = next.push(task);
            self.next = Some(next);
            result
        }
    }
}

struct TaskBlockRef<T: Future<Output = ()>>(NonNull<TaskBlock<T>>);

impl<T: Future<Output = ()>> TaskBlockRef<T> {
    unsafe fn new(shared_waker: Arc<AtomicWaker>) -> Self {
        let inner =
            NonNull::new(alloc(Layout::new::<TaskBlock<T>>()) as *mut TaskBlock<T>).unwrap();
        inner.as_ptr().write(TaskBlock {
            tasks: MaybeUninit::uninit().assume_init(),
            wakers: [
                TaskBlockWaker::new(inner, 0),
                TaskBlockWaker::new(inner, 1),
                TaskBlockWaker::new(inner, 2),
                TaskBlockWaker::new(inner, 3),
                TaskBlockWaker::new(inner, 4),
                TaskBlockWaker::new(inner, 5),
                TaskBlockWaker::new(inner, 6),
                TaskBlockWaker::new(inner, 7),
                TaskBlockWaker::new(inner, 8),
                TaskBlockWaker::new(inner, 9),
                TaskBlockWaker::new(inner, 10),
                TaskBlockWaker::new(inner, 11),
                TaskBlockWaker::new(inner, 12),
                TaskBlockWaker::new(inner, 13),
                TaskBlockWaker::new(inner, 14),
                TaskBlockWaker::new(inner, 15),
                TaskBlockWaker::new(inner, 16),
                TaskBlockWaker::new(inner, 17),
                TaskBlockWaker::new(inner, 18),
                TaskBlockWaker::new(inner, 19),
                TaskBlockWaker::new(inner, 20),
                TaskBlockWaker::new(inner, 21),
                TaskBlockWaker::new(inner, 22),
                TaskBlockWaker::new(inner, 23),
                TaskBlockWaker::new(inner, 24),
                TaskBlockWaker::new(inner, 25),
                TaskBlockWaker::new(inner, 26),
                TaskBlockWaker::new(inner, 27),
                TaskBlockWaker::new(inner, 28),
                TaskBlockWaker::new(inner, 29),
                TaskBlockWaker::new(inner, 30),
                TaskBlockWaker::new(inner, 31),
                TaskBlockWaker::new(inner, 32),
                TaskBlockWaker::new(inner, 33),
                TaskBlockWaker::new(inner, 34),
                TaskBlockWaker::new(inner, 35),
                TaskBlockWaker::new(inner, 36),
                TaskBlockWaker::new(inner, 37),
                TaskBlockWaker::new(inner, 38),
                TaskBlockWaker::new(inner, 39),
                TaskBlockWaker::new(inner, 40),
                TaskBlockWaker::new(inner, 41),
                TaskBlockWaker::new(inner, 42),
                TaskBlockWaker::new(inner, 43),
                TaskBlockWaker::new(inner, 44),
                TaskBlockWaker::new(inner, 45),
                TaskBlockWaker::new(inner, 46),
                TaskBlockWaker::new(inner, 47),
                TaskBlockWaker::new(inner, 48),
                TaskBlockWaker::new(inner, 49),
                TaskBlockWaker::new(inner, 50),
                TaskBlockWaker::new(inner, 51),
                TaskBlockWaker::new(inner, 52),
                TaskBlockWaker::new(inner, 53),
                TaskBlockWaker::new(inner, 54),
                TaskBlockWaker::new(inner, 55),
                TaskBlockWaker::new(inner, 56),
                TaskBlockWaker::new(inner, 57),
                TaskBlockWaker::new(inner, 58),
                TaskBlockWaker::new(inner, 59),
                TaskBlockWaker::new(inner, 60),
                TaskBlockWaker::new(inner, 61),
                TaskBlockWaker::new(inner, 62),
                TaskBlockWaker::new(inner, 63),
            ],
            shared_waker,
            pending: 0,
            poll: AtomicU64::new(0),
            ref_count: AtomicUsize::new(1),
            next: None,
        });
        Self(inner)
    }

    unsafe fn poll(&mut self) -> Poll<()> {
        self.0.as_mut().poll()
    }

    unsafe fn push(&mut self, task: T) -> Poll<()> {
        self.0.as_mut().push(task)
    }

    unsafe fn clone_inner(inner: NonNull<TaskBlock<T>>) {
        inner.as_ref().ref_count.fetch_add(1, Ordering::Relaxed);
    }

    unsafe fn drop_inner(inner: NonNull<TaskBlock<T>>) {
        if inner.as_ref().ref_count.fetch_sub(1, Ordering::Release) == 1 {
            atomic::fence(Ordering::Acquire);
            ptr::drop_in_place(inner.as_ptr());
            dealloc(inner.as_ptr() as *mut u8, Layout::new::<TaskBlock<T>>())
        }
    }
}

impl<T: Future<Output = ()>> Drop for TaskBlockRef<T> {
    fn drop(&mut self) {
        unsafe { Self::drop_inner(self.0) }
    }
}

unsafe impl<T: Future<Output = ()> + Send + Sync> Send for TaskBlockRef<T> {}
unsafe impl<T: Future<Output = ()> + Send + Sync> Sync for TaskBlockRef<T> {}

struct TaskBlockWaker<T: Future<Output = ()>> {
    block: NonNull<TaskBlock<T>>,
    poll_mask: u64,
}

impl<T: Future<Output = ()>> TaskBlockWaker<T> {
    fn new(block: NonNull<TaskBlock<T>>, index: usize) -> Self {
        Self {
            block,
            poll_mask: 1 << (63 - index),
        }
    }

    unsafe fn ptr_to_ref<'a>(data: *const ()) -> &'a Self {
        (data as *const TaskBlockWaker<T>).as_ref().unwrap()
    }

    unsafe fn raw_waker(&self) -> RawWaker {
        TaskBlockRef::clone_inner(self.block);
        RawWaker::new(self as *const Self as *const (), &Self::VTABLE)
    }

    unsafe fn clone(data: *const ()) -> RawWaker {
        let waker = Self::ptr_to_ref(data);
        TaskBlockRef::clone_inner(waker.block);
        RawWaker::new(data, &Self::VTABLE)
    }

    unsafe fn wake(data: *const ()) {
        let waker = Self::ptr_to_ref(data);
        let block = waker.block.as_ref();
        block.poll.fetch_or(waker.poll_mask, Ordering::SeqCst);
        block.shared_waker.wake();
        TaskBlockRef::drop_inner(waker.block);
    }

    unsafe fn wake_by_ref(data: *const ()) {
        let waker = Self::ptr_to_ref(data);
        let block = waker.block.as_ref();
        block.poll.fetch_or(waker.poll_mask, Ordering::SeqCst);
        block.shared_waker.wake();
    }

    unsafe fn drop(data: *const ()) {
        let waker = Self::ptr_to_ref(data);
        TaskBlockRef::drop_inner(waker.block);
    }

    const VTABLE: RawWakerVTable =
        RawWakerVTable::new(Self::clone, Self::wake, Self::wake_by_ref, Self::drop);
}
