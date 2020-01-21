// -- semaphore.rs --

pub use v3::{Semaphore, SemaphoreGuard};

// --

#[allow(unused)]
pub mod v3 {
    use crate::{
        task,
        sync::{Arc, Condvar, Mutex},
    };

    // --

    #[cfg_attr(test, derive(Debug))]
    struct _Semaphore {
        // count.0: count of semaphore
        // count.1: count of cv.wait
        count: Mutex<(usize, usize)>,
        cv: Condvar,
    }
    #[derive(Clone)]
    pub struct Semaphore(Arc<_Semaphore>);
    impl Semaphore {
        pub fn new(count: usize) -> Self {
            Self(Arc::new(_Semaphore {
                count: Mutex::new((count, 0)),
                cv: Condvar::new(),
            }))
        }
        pub async fn acquire(&self) {
            let mut g = self.0.count.lock().await;
            if g.0 == 0 {
                g.1 += 1;
                let mut g = self.0.cv.wait(g).await;
                g.1 -= 1;
                assert!(g.0 > 0);
                g.0 -= 1;
            } else {
                assert!(g.0 > 0);
                g.0 -= 1;
            }
        }
        pub async fn release(&self) {
            let mut g = self.0.count.lock().await;
            g.0 += 1;
            if g.1 > 0 {
                self.0.cv.notify_one();
            }
        }
        #[inline]
        pub async fn lock(&self) -> SemaphoreGuard {
            self.acquire().await;
            SemaphoreGuard(self.clone())
        }
    }

    pub struct SemaphoreGuard(Semaphore);
    impl Drop for SemaphoreGuard {
        #[inline]
        fn drop(&mut self) {
            task::block_on(async {
                self.0.release().await;
            });
        }
    }
}
