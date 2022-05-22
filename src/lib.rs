use std::ops::Sub;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Condvar, Mutex};

// 通过计数，和condvar，来通知主线程以实现同步
struct Inner {
    cvar: Condvar,
    counter: Mutex<usize>,
}

pub struct WaitGroup {
    inner: Arc<Inner>,
}

impl Drop for WaitGroup {
    fn drop(&mut self) {
        self.done()
    }
}
impl Clone for WaitGroup {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl WaitGroup {
    fn new() -> Self {
        Self {
            inner: Arc::new(Inner {
                cvar: Condvar::new(),
                counter: Mutex::new(0),
            }),
        }
    }

    fn add(&self, num: usize) -> Self {
        let mut counter = self.inner.counter.lock().unwrap();
        *counter += num;
        Self {
            inner: self.inner.clone(),
        }
    }

    fn done(&self) {
        let mut counter = self.inner.counter.lock().unwrap();
        *counter = if counter.eq(&1) {
            self.inner.cvar.notify_all();
            0
        } else if counter.eq(&0) {
            0
        } else {
            counter.sub(1)
        };
    }

    pub fn wait(&self) {
        let mut counter = self.inner.counter.lock().unwrap();
        if counter.eq(&0) {
            return;
        }
        while counter.gt(&0) {
            counter = self.inner.cvar.wait(counter).unwrap();
        }
    }
}

struct AsyncInner {
    cvar: Condvar,
    counter: AtomicUsize,
}

impl Default for AsyncInner {
    fn default() -> Self {
        Self {
            cvar: Condvar::new(),
            counter: AtomicUsize::new(0),
        }
    }
}

struct AsyncWaitGroup {
    inner: Arc<AsyncInner>,
}

impl Default for AsyncWaitGroup {
    fn default() -> Self {
        Self {
            inner: Arc::new(AsyncInner::default()),
        }
    }
}

impl AsyncWaitGroup {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&self, num: usize) -> Self {
        self.inner.counter.fetch_add(num, Ordering::SeqCst);
        Self {
            inner: self.inner.clone(),
        }
    }

    pub fn done(&self) {
        let _ = self
            .inner
            .counter
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |counter| {
                if counter == 1 {
                    self.inner.cvar.notify_all();
                    Some(0)
                } else if counter == 0 {
                    None
                } else {
                    Some(counter - 1)
                }
            });
    }
    pub fn wait(&self) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::{self, sleep};
    use std::time::Duration;

    #[test]
    fn it_works() {
        let wg = WaitGroup::new();

        for i in 0..5 {
            let wg = wg.add(1);
            thread::spawn(move || {
                sleep(Duration::from_secs(1));
                println!("{} work work work!", i);
                wg.done();
            });
        }
        wg.wait();
        println!("over");
    }

    #[test]
    fn test_main() {
        let pair = Arc::new((Mutex::new(false), Condvar::new()));
        let pair2 = Arc::clone(&pair);
        thread::spawn(move || {
            let (lock, cvar) = &*pair2;
            {
                let mut started = lock.lock().unwrap();
                *started = true;
            }
            println!("I'm a happy worker!");
            // 通知主线程
            cvar.notify_all();
            loop {
                thread::sleep(Duration::from_secs(1));
                println!("work..!");
            }
        });
        // 等待工作线程的通知
        let (lock, cvar) = &*pair;
        let mut started = lock.lock().unwrap();
        while !*started {
            println!("loop wait the started: {}", started);
            started = cvar.wait(started).unwrap();
            println!("get notify started: {}", started);
        }
        println!("Worker started!");
    }
}
