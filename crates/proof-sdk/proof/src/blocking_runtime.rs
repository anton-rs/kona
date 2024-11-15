//! This module contains a blocking runtime for futures, allowing for synchronous execution of async
//! code in an embedded environment.

use alloc::boxed::Box;
use core::{
    future::Future,
    task::{Context, Poll, RawWaker, RawWakerVTable, Waker},
};

/// This function busy waits on a future until it is ready. It uses a no-op waker to poll the future
/// in a thread-blocking loop.
pub fn block_on<T>(f: impl Future<Output = T>) -> T {
    let mut f = Box::pin(f);

    // Construct a no-op waker.
    fn noop_clone(_: *const ()) -> RawWaker {
        noop_raw_waker()
    }
    const fn noop(_: *const ()) {}
    fn noop_raw_waker() -> RawWaker {
        let vtable = &RawWakerVTable::new(noop_clone, noop, noop, noop);
        RawWaker::new(core::ptr::null(), vtable)
    }
    let waker = unsafe { Waker::from_raw(noop_raw_waker()) };
    let mut context = Context::from_waker(&waker);

    loop {
        // Safety: This is safe because we only poll the future once per loop iteration,
        // and we do not move the future after pinning it.
        if let Poll::Ready(v) = f.as_mut().poll(&mut context) {
            return v;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::future::ready;

    #[test]
    fn test_block_on() {
        let f = async { 42 };
        assert_eq!(block_on(f), 42);
    }

    #[test]
    fn test_block_on_ready() {
        let f = ready(42);
        assert_eq!(block_on(f), 42);
    }
}
