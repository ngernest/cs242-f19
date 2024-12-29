#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use crate::future::{Future, Poll};
use crate::future_util::*;
use std::mem;
use std::sync::{mpsc, Arc, Mutex, MutexGuard};
use std::thread;

/*
 * Core executor interface.
 */

pub trait Executor {
    fn spawn<F>(&mut self, f: F)
    where
        F: Future<Item = ()> + 'static;
    fn wait(&mut self);
}

/*
 * Example implementation of a naive executor that executes futures
 * in sequence.
 */

pub struct BlockingExecutor;

impl BlockingExecutor {
    pub fn new() -> BlockingExecutor {
        BlockingExecutor
    }
}

impl Executor for BlockingExecutor {
    fn spawn<F>(&mut self, mut f: F)
    where
        F: Future<Item = ()>,
    {
        loop {
            if let Poll::Ready(()) = f.poll() {
                break;
            }
        }
    }

    fn wait(&mut self) {}
}

/*
 * Part 2a - Single threaded executor
 */

pub struct SingleThreadExecutor {
    futures: Vec<Box<dyn Future<Item = ()>>>,
}

impl SingleThreadExecutor {
    pub fn new() -> SingleThreadExecutor {
        SingleThreadExecutor { futures: vec![] }
    }
}

impl Executor for SingleThreadExecutor {
    fn spawn<F>(&mut self, mut f: F)
    where
        F: Future<Item = ()> + 'static,
    {
        match f.poll() {
            Poll::NotReady => self.futures.push(Box::new(f)),
            Poll::Ready(_) => (),
        }
    }

    fn wait(&mut self) {
        let n = self.futures.len();
        let mut num_completed = 0;
        while num_completed < n {
            for (i, fut) in self.futures.iter_mut().enumerate() {
                if let Poll::Ready(_) = fut.poll() {
                    num_completed += 1;
                    continue;
                }
            }
        }
        self.futures.clear()
    }
}

pub struct MultiThreadExecutor {
    sender: mpsc::Sender<Option<Box<dyn Future<Item = ()>>>>,
    threads: Vec<thread::JoinHandle<()>>,
}

impl MultiThreadExecutor {
    pub fn new(num_threads: i32) -> MultiThreadExecutor {
        let (sender, rcvr) = mpsc::channel();
        // Wrap the receiver in `Arc<Mutex<_>>` so it can be shared across thread
        let receiver = Arc::new(Mutex::new(rcvr));
        let mut threads: Vec<thread::JoinHandle<()>> = Vec::new();
        for _ in 0..num_threads {
            let thread_receiver = Arc::clone(&receiver);
            // We need the `move` keyword in front of the closure so that the closure
            // take ownership of `thread_receiver`
            let thread_handle = thread::spawn(move || {
                // Create a thread-local executor
                let mut local_executor = SingleThreadExecutor::new();
                loop {
                    // Loop receiving futures form the channel
                    let future: Option<Box<dyn Future<Item = ()>>> = thread_receiver
                        .lock()
                        .expect("Failed to acquire lock")
                        .recv()
                        .expect("Channel closed unexpectedly");
                    if let Some(fut) = future {
                        // Got a future, spawn it on local executor
                        local_executor.spawn(fut);
                    } else {
                        // Wait on their single-thread executor when we receive `None`,
                        // then shutdown afterwards
                        local_executor.wait();
                        break;
                    }
                }
            });
            threads.push(thread_handle)
        }
        MultiThreadExecutor { sender, threads }
    }
}

impl Executor for MultiThreadExecutor {
    /// Spawning a future sends the future over a channel
    fn spawn<F>(&mut self, f: F)
    where
        F: Future<Item = ()> + 'static,
    {
        let future: Option<Box<dyn Future<Item = ()>>> = Some(Box::new(f));
        self.sender
            .send(future)
            .expect("Failed to send future to worker thread");
    }

    fn wait(&mut self) {
        let n = self.threads.len();
        // Send `None` to each thread as a shutdown signal
        for _ in 0..n {
            self.sender
                .send(None)
                .expect("Failed to send shutdown signal");
        }
        // Take ownership of `self.threads`, then join all the worker thread
        take_mut::take(&mut self.threads, |threads| {
            for t in threads {
                t.join().expect("Worker thread panicked")
            }
            Vec::new()
        });
    }
}
