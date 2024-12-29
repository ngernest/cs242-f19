/* Add UseCounter struct, methods, and trait implementations here */

use std::{
    ops::{Deref, DerefMut},
    sync::Mutex,
};

pub struct UseCounter<T> {
    value: T,
    counter: Mutex<u32>,
}

impl<T> UseCounter<T> {
    pub fn new(value: T) -> Self {
        UseCounter {
            value,
            counter: Mutex::new(0),
        }
    }

    pub fn count(&self) -> u32 {
        *(self.counter.lock().unwrap())
    }
}

impl<T> Deref for UseCounter<T> {
    type Target = T;
    fn deref(&self) -> &T {
        // NB: the count is automatically released when `count` goes out of scope
        let mut count = self.counter.lock().unwrap();
        *count += 1;
        &self.value
    }
}

impl<T> DerefMut for UseCounter<T> {
    fn deref_mut(&mut self) -> &mut T {
        let mut count = self.counter.lock().unwrap();
        *count += 1;
        &mut self.value
    }
}
