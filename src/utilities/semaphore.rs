// -- semaphore.rs --

pub use v3::{Semaphore, SemaphoreGuard};

// --
/*
#[allow(unused)]
pub mod v1 {
    use crossbeam_channel::{bounded, Receiver, Sender};

    // --

    #[cfg_attr(test, derive(Debug))]
    pub struct Semaphore {
        tx: Sender<()>,
        rx: Receiver<()>,
    }

    impl Semaphore {
        pub fn new(count: usize) -> Self {
            let (tx, rx) = bounded(count);
            for _ in 0..count {
                tx.send(()).expect("send return error in Semaphore::new");
            }
            Self { tx, rx }
        }
        #[inline]
        pub fn lock(&self) -> SemaphoreGuard {
            self.rx
                .recv()
                .expect("recv return error in Semaphore::lock");
            dbg!();
            SemaphoreGuard(self.tx.clone())
        }
    }

    pub struct SemaphoreGuard(Sender<()>);
    impl Drop for SemaphoreGuard {
        #[inline]
        fn drop(&mut self) {
            self.0
                .send(())
                .expect("send return error in SemaphoreGuard::drop");
            dbg!();
        }
    }
}
*/
#[allow(unused)]
pub mod v2 {
    use {
        async_std::task,
        futures::{
            channel::mpsc::{channel, Receiver, Sender},
            sink::SinkExt,
            stream::StreamExt,
        },
    };

    #[cfg_attr(test, derive(Debug))]
    pub struct Semaphore {
        tx: Sender<()>,
        rx: Receiver<()>,
    }

    impl Semaphore {
        pub fn new(count: usize) -> Self {
            let (mut tx, rx) = channel(count);
            for _ in 0..count {
                task::block_on(async {
                    tx.send(()).await.unwrap();
                });
            }
            Self { tx, rx }
        }
        #[inline]
        pub fn lock(&mut self) -> SemaphoreGuard {
            task::block_on(async {
                self.rx.next().await.unwrap(); // .expect("recv return error in Semaphore::lock");
            });
            dbg!();
            SemaphoreGuard(self.tx.clone())
        }
    }
    pub struct SemaphoreGuard(Sender<()>);
    impl SemaphoreGuard {
        pub fn release(&mut self) {
            task::block_on(async {
                self.0.send(()).await.unwrap(); // .expect("send return error in SemaphoreGuard::drop");
            });
            dbg!();
        }
    }

    impl Drop for SemaphoreGuard {
        #[inline]
        fn drop(&mut self) {
            task::block_on(async {
                self.0.send(()).await.unwrap(); // .expect("send return error in SemaphoreGuard::drop");
            });
            dbg!();
        }
    }
}

#[allow(unused)]
pub mod v3 {
    use std::sync::{Arc, Condvar, Mutex};

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
        pub fn acquire(&self) {
            let mut g = self.0.count.lock().unwrap();
            if g.0 == 0 {
                g.1 += 1;
                if let Ok(mut g2) = self.0.cv.wait(g) {
                    g2.1 -= 1;
                    assert!(g2.0 > 0);
                    g2.0 -= 1;
                }
            } else {
                assert!(g.0 > 0);
                g.0 -= 1;
            }
        }
        pub fn release(&self) {
            let mut g = self.0.count.lock().unwrap();
            g.0 += 1;
            if g.1 > 0 {
                self.0.cv.notify_one();
            }
        }
        #[inline]
        pub fn lock(&self) -> SemaphoreGuard {
            self.acquire();
            SemaphoreGuard(self.clone())
        }
    }

    pub struct SemaphoreGuard(Semaphore);
    impl Drop for SemaphoreGuard {
        #[inline]
        fn drop(&mut self) {
            self.0.release();
        }
    }
}

#[allow(unused)]
pub mod v4 {
    use std::{
        sync::{Arc, Condvar, Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard},
    };

    // --

    #[cfg_attr(test, derive(Debug))]
    pub struct Semaphore<T: ?Sized> {
        // count.0: count of semaphore
        // count.1: count of cv.wait
        count: Mutex<(usize, usize)>,
        cv: Condvar,
        t: RwLock<T>,
    }
    impl<T> Semaphore<T> {
        pub fn new(count: usize, t: T) -> Self {
            Self{
                count: Mutex::new((count, 0)),
                cv: Condvar::new(),
                t: RwLock::new(t),
            }
        }
    }
    impl<T: ?Sized> Semaphore<T> {
        fn acquire(&self) {
            let mut g = self.count.lock().unwrap();
            if g.0 == 0 {
                g.1 += 1;
                if let Ok(mut gg) = self.cv.wait(g) {
                    gg.1 -= 1;
                    assert!(gg.0 > 0);
                    gg.0 -= 1;
                }
            } else {
                assert!(g.0 > 0);
                g.0 -= 1;
            }
            dbg!();
        }
        fn release(&self) {
            let mut g = self.count.lock().unwrap();
            g.0 += 1;
            if g.1 > 0 {
                self.cv.notify_one();
            }
            dbg!();
        }
        #[inline]
        pub fn lock(&self) -> SemaphoreGuard<T> {
            self.acquire();
            SemaphoreGuard(self)
        }
    }

    pub struct SemaphoreGuard<'a, T = ()>(&'a Semaphore<T>)
    where
        T: ?Sized;
    impl<'a, T: ?Sized> SemaphoreGuard<'a, T> {
        #[inline]
        pub fn read(&self) -> RwLockReadGuard<'a, T> {
            self.0.t.read().unwrap()
        }
        #[inline]
        pub fn write(&self) -> RwLockWriteGuard<'a, T> {
            self.0.t.write().unwrap()
        }
    }
    impl<T: ?Sized> Drop for SemaphoreGuard<'_, T> {
        #[inline]
        fn drop(&mut self) {
            self.0.release();
        }
    }
}

// --

#[allow(unused)]
#[cfg(test)]
mod tests {
    use super::v4::*;

    #[test]
    fn sem_test1() {
        let sem = Semaphore::new(3, 2_u8);
        let g = sem.lock();
        let g2 = sem.lock();
        {
            let g3 = sem.lock();
        }
        let g4 = sem.lock();
        {
            let rg = g.read();
            dbg!(assert_eq!(2, *rg));
            let rg2 = g.read();
            dbg!(assert_eq!(2, *rg2));
            let rg3 = g.read();
            dbg!(assert_eq!(2, *rg3));
            let rg4 = g.read();
            dbg!(assert_eq!(2, *rg4));
            let rg5 = g.read();
            dbg!(assert_eq!(2, *rg5));
        }
        *g.write() = 23;
        {
            let rg = g.read();
            dbg!(assert_eq!(23, *rg));
            let rg2 = g.read();
            dbg!(assert_eq!(23, *rg2));
            let rg3 = g.read();
            dbg!(assert_eq!(23, *rg3));
            let rg4 = g.read();
            dbg!(assert_eq!(23, *rg4));
            let rg5 = g.read();
            dbg!(assert_eq!(23, *rg5));
        }
    }
    #[test]
    fn sem_test2() {
        let sem = Semaphore::new(3, 2_u8);
        let g = sem.lock();
        let g2 = sem.lock();
        {
            let g3 = sem.lock();
        }
        let g4 = sem.lock();
        // let rg = g.read();
        dbg!(g.read());
        *dbg!(g.write()) = 23;
        dbg!(g.read());
    }
    #[test]
    fn sem_test3() {
        let sem = Semaphore::new(3, 2_u8);
        let g = sem.lock();
        let g2 = sem.lock();
        {
            let g3 = sem.lock();
        }
        let g4 = sem.lock();
        // let rg = g.read();
        dbg!(g.read());
        *dbg!(g.write()) = 23;
        dbg!(g.read());
    }
}
