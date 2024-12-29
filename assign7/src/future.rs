#![allow(dead_code, unused_imports)]

use take_mut;

/*
 * Core futures interface.
 */

#[derive(Debug)]
pub enum Poll<T> {
    Ready(T),
    NotReady,
}

pub trait Future: Send {
    type Item: Send;
    fn poll(&mut self) -> Poll<Self::Item>;
}

/*
 * Example implementation of a future for an item that returns immediately.
 */

// Container for the state of the future.
pub struct Immediate<T> {
    t: Option<T>,
}

// Constructor to build the future. Note that the return type just says
// "this produces a future", not specifying concretely the type Immediate.
pub fn immediate<T>(t: T) -> impl Future<Item = T>
where
    T: Send,
{
    Immediate { t: Some(t) }
}

// To treat Immediate as a future, we have to implement poll. Here it's
// relatively simple, since we return immediately with a Poll::Ready.
impl<T> Future for Immediate<T>
where
    T: Send,
{
    type Item = T;

    fn poll(&mut self) -> Poll<Self::Item> {
        Poll::Ready(self.t.take().unwrap())
    }
}

/*
 * Example implementation of a future combinator that applies a function to
 * the output of a future.
 */

struct Map<Fut, Fun> {
    fut: Fut,
    fun: Option<Fun>,
}

pub fn map<T, Fut, Fun>(fut: Fut, fun: Fun) -> impl Future<Item = T>
where
    T: Send,
    Fut: Future,
    Fun: FnOnce(Fut::Item) -> T + Send,
{
    Map {
        fut,
        fun: Some(fun),
    }
}

impl<T, Fut, Fun> Future for Map<Fut, Fun>
where
    T: Send,
    Fut: Future,
    Fun: FnOnce(Fut::Item) -> T + Send,
{
    type Item = T;

    fn poll(&mut self) -> Poll<Self::Item> {
        match self.fut.poll() {
            Poll::NotReady => Poll::NotReady,
            Poll::Ready(s) => {
                let f: Option<Fun> = self.fun.take();
                Poll::Ready(f.unwrap()(s))
            }
        }
    }
}

/*
 * Part 1a - Join
 */

// A join of two futures is a state machine depending on which future is
// completed, represented as an enum.
pub enum Join<F, G>
where
    F: Future,
    G: Future,
{
    BothRunning(F, G),
    FirstDone(F::Item, G),
    SecondDone(F, G::Item),
    Done,
}

// When a join is created, we start by assuming neither child future
// has completed.
pub fn join<F, G>(f: F, g: G) -> impl Future<Item = (F::Item, G::Item)>
where
    F: Future,
    G: Future,
{
    Join::BothRunning(f, g)
}

impl<F, G> Future for Join<F, G>
where
    F: Future,
    G: Future,
{
    type Item = (F::Item, G::Item);

    fn poll(&mut self) -> Poll<Self::Item> {
        // Since we can't return the `Poll` result directly inside
        // the `take` closure, we have to store it in a mutable variable,
        // which we'll modify inside the closure
        let mut poll_result = Poll::NotReady;

        take_mut::take(self, |this| match this {
            Join::FirstDone(f_item, mut g) => match g.poll() {
                Poll::Ready(g_item) => {
                    poll_result = Poll::Ready((f_item, g_item));
                    Join::Done
                }
                Poll::NotReady => Join::FirstDone(f_item, g),
            },
            Join::SecondDone(mut f, g_item) => match f.poll() {
                Poll::Ready(f_item) => {
                    poll_result = Poll::Ready((f_item, g_item));
                    Join::Done
                }
                Poll::NotReady => Join::SecondDone(f, g_item),
            },
            Join::BothRunning(mut f, mut g) => match (f.poll(), g.poll()) {
                (Poll::Ready(f_item), Poll::Ready(g_item)) => {
                    poll_result = Poll::Ready((f_item, g_item));
                    Join::Done
                }
                (Poll::Ready(f_item), Poll::NotReady) => Join::FirstDone(f_item, g),
                (Poll::NotReady, Poll::Ready(g_item)) => Join::SecondDone(f, g_item),
                (Poll::NotReady, Poll::NotReady) => Join::BothRunning(f, g),
            },
            Join::Done => panic!("poll called after future completed"),
        });
        poll_result
    }
}

/*
 * Part 1b - AndThen
 */

// The AndThen state machine depends on which future is currently running.
pub enum AndThen<Fut1, Fut2, Fun> {
    First(Fut1, Fun),
    Second(Fut2),
    Done,
}

pub fn and_then<Fut1, Fut2, Fun>(fut: Fut1, fun: Fun) -> impl Future<Item = Fut2::Item>
where
    Fut1: Future,
    Fut2: Future,
    Fun: FnOnce(Fut1::Item) -> Fut2 + Send,
{
    AndThen::First(fut, fun)
}

impl<Fut1, Fut2, Fun> Future for AndThen<Fut1, Fut2, Fun>
where
    Fut1: Future,
    Fut2: Future,
    Fun: FnOnce(Fut1::Item) -> Fut2 + Send,
{
    type Item = Fut2::Item;

    fn poll(&mut self) -> Poll<Self::Item> {
        let mut poll_result = Poll::NotReady;

        take_mut::take(self, |this| {
            match this {
                // Note the use of the `mut` identifier pattern,
                // which allows the call `first.poll()` to be possible
                // (since it requires mutable ownership of `first`)
                AndThen::First(mut first, f) => {
                    match first.poll() {
                        Poll::Ready(item) => {
                            let mut second = f(item);
                            // Recursively poll the new future
                            poll_result = second.poll();
                            AndThen::Done
                        }
                        Poll::NotReady => AndThen::First(first, f),
                    }
                }
                AndThen::Second(mut second) => match second.poll() {
                    Poll::Ready(item) => {
                        poll_result = Poll::Ready(item);
                        AndThen::Done
                    }
                    Poll::NotReady => AndThen::Second(second),
                },
                AndThen::Done => {
                    panic!("poll called after future completed")
                }
            }
        });

        poll_result
    }
}
