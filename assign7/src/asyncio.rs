#![allow(dead_code, unused_imports, unused_variables)]

use crate::future::*;
use std::clone;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

pub struct FileReader {
    path: PathBuf,
    thread: Option<thread::JoinHandle<io::Result<String>>>,
    done_flag: Arc<AtomicBool>,
}

impl FileReader {
    pub fn new(path: PathBuf) -> FileReader {
        let atomic_bool = AtomicBool::new(false);
        let done_flag = Arc::new(atomic_bool);
        FileReader {
            path,
            thread: None,
            done_flag,
        }
    }
}

impl Future for FileReader {
    type Item = io::Result<String>;

    fn poll(&mut self) -> Poll<Self::Item> {
        // If this is the first poll, spawn a thread that reads the path
        if self.thread.is_none() {
            let cloned_path = self.path.clone();
            let cloned_done_flag = Arc::clone(&self.done_flag);
            let thread = thread::spawn(move || {
                let str = fs::read_to_string(cloned_path);
                // `Release` says everything beforehand must complete before
                // we store `true` into the `done_flag`
                cloned_done_flag.store(true, Ordering::Release);
                str
            });
            self.thread = Some(thread);
            // First poll spawned the thread but hasn't completed
            return Poll::NotReady;
        }
        // // Check if the thread has finished
        // `Acquire` says everything afterwards must wait until
        // we finish reading from the `done_flag`
        let is_done = self.done_flag.load(Ordering::Acquire);
        if !is_done {
            Poll::NotReady
        } else {
            // Thread is done, take ownership of the handle by calling
            // `Option::take` (which stores `None` in `self.thread`)
            if let Some(handle) = self.thread.take() {
                let str = handle.join().expect("Worker thread panicked");
                Poll::Ready(str)
            } else {
                unreachable!()
            }
        }
    }
}
