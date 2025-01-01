use rand::{prelude::ThreadRng, thread_rng, Rng};
use std::any::TypeId;
use std::cell::RefCell;
use std::marker;
use std::marker::PhantomData;
use std::mem::transmute;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};

thread_local!(static RNG: RefCell<ThreadRng> = RefCell::new(thread_rng()));
thread_local!(pub static NOISY: AtomicBool = AtomicBool::new(false));
const DROP_PROB: f32 = 0.4;

/* -------------------------------------------------------------------------- */
/*                                 TCP Packets                                */
/* -------------------------------------------------------------------------- */

pub type Buffer = Vec<u8>;

/// A `Packet` is a `buffer` along with the sequence no. (`seqno`)
/// (i.e. its index in a list of packets that the client wants to send)
#[derive(Debug, Clone)]
pub struct Packet {
  pub buf: Buffer,
  pub seqno: usize,
}

/* -------------------------------------------------------------------------- */
/*                                Session Types                               */
/* -------------------------------------------------------------------------- */

pub struct Send<T, S>(PhantomData<(T, S)>);
pub struct Recv<T, S>(PhantomData<(T, S)>);
pub struct Offer<Left, Right>(PhantomData<(Left, Right)>);
pub struct Choose<Left, Right>(PhantomData<(Left, Right)>);
pub struct Close;
pub struct Rec<S>(PhantomData<S>);

/// Peano numerals
pub struct Z;
pub struct S<N>(PhantomData<N>);

/// De Bruijn indices, where `N` is a Peano numeral
pub struct Var<N>(PhantomData<N>);

/* -------------------------------------------------------------------------- */
/*                             `HasDual` instances                            */
/* -------------------------------------------------------------------------- */

pub trait HasDual {
  type Dual;
}

impl HasDual for Close {
  type Dual = Close;
}

impl<T, S> HasDual for Send<T, S>
where
  S: HasDual,
{
  type Dual = Recv<T, S::Dual>;
}

impl<T, S> HasDual for Recv<T, S>
where
  S: HasDual,
{
  type Dual = Send<T, S::Dual>;
}

impl<Left, Right> HasDual for Choose<Left, Right>
where
  Left: HasDual,
  Right: HasDual,
{
  type Dual = Offer<Left::Dual, Right::Dual>;
}

impl<Left, Right> HasDual for Offer<Left, Right>
where
  Left: HasDual,
  Right: HasDual,
{
  type Dual = Choose<Left::Dual, Right::Dual>;
}

impl<N> HasDual for Var<N> {
  type Dual = Var<N>;
}

impl<N> HasDual for S<N> {
  type Dual = S<N>;
}

impl HasDual for Z {
  type Dual = Z;
}

impl<S> HasDual for Rec<S>
where
  S: HasDual,
{
  type Dual = Rec<S::Dual>;
}

/* -------------------------------------------------------------------------- */
/*                           Session-typed Channels                           */
/* -------------------------------------------------------------------------- */

/// Channels are parameterized by an environment `Env` and a session type `S`
/// (`Env` is a mapping from De Bruijn indices to session types)
pub struct Chan<Env, S> {
  /// The `sender` sends messages to the counterparty
  pub sender: Sender<Box<u8>>,

  /// The `receiver` receives messages
  pub receiver: Receiver<Box<u8>>,
  pub _data: PhantomData<(Env, S)>,
}

impl<Env, S> Chan<Env, S> {
  /// Takes a `T`, converts it to a blob of bits, and sends it to the receiver
  unsafe fn write<T>(&self, x: T)
  where
    T: marker::Send + 'static,
  {
    let sender: &Sender<Box<T>> = transmute(&self.sender);
    sender.send(Box::new(x)).unwrap();
  }

  /// Receives a blob of bits and converts it into a `T`
  unsafe fn read<T>(&self) -> T
  where
    T: marker::Send + 'static,
  {
    let receiver: &Receiver<Box<T>> = transmute(&self.receiver);
    *receiver.recv().unwrap()
  }
}

impl<Env> Chan<Env, Close> {
  pub fn close(self) {}
}

impl<Env, T, S> Chan<Env, Send<T, S>>
where
  T: marker::Send + 'static,
{
  /// When we have a channel w/ session type `Send<T, S>`,
  /// we can `send` a `T` & get a channel of type `Chan<S>`,
  /// where `S` is the rest of the session type
  pub fn send(self, x: T) -> Chan<Env, S> {
    unsafe {
      // Actually send the bits over
      self.write(x);

      // Call `transmute` to transform `Chan<Send<T, S>>` into `Chan<S>`
      transmute(self)
    }
  }
}

impl<Env, T, S> Chan<Env, Recv<T, S>>
where
  T: marker::Send + 'static,
{
  /// Reads a value of type `T`, then calls `transmute` to transform the channel
  /// `Chan<Recv<T, S>>` into `Chan<S>`
  pub fn recv(self) -> (Chan<Env, S>, T) {
    NOISY.with(|noisy| unsafe {
      let mut x = self.read();
      if noisy.load(Ordering::SeqCst) && TypeId::of::<T>() == TypeId::of::<Vec<Packet>>() {
        let xmut = &mut x;
        let xmut: &mut Vec<Packet> = transmute(xmut);

        let mut rng = RNG.with(|gen| gen.borrow().clone());
        let mut idx = (0..xmut.len())
          .filter(|_| rng.gen_bool(DROP_PROB.into()))
          .collect::<Vec<_>>();
        idx.reverse();

        for i in idx {
          xmut.remove(i);
        }
      }

      (transmute(self), x)
    })
  }
}

impl<Env, Left, Right> Chan<Env, Choose<Left, Right>> {
  pub fn left(self) -> Chan<Env, Left> {
    unsafe {
      // We send `true` to indicate we've chosen the `left` branch
      self.write(true);
      transmute(self)
    }
  }

  pub fn right(self) -> Chan<Env, Right> {
    unsafe {
      // We send `false` to indicate we've chosen the `right` branch
      self.write(false);
      transmute(self)
    }
  }
}

pub enum Branch<L, R> {
  Left(L),
  Right(R),
}

impl<Env, Left, Right> Chan<Env, Offer<Left, Right>> {
  pub fn offer(self) -> Branch<Chan<Env, Left>, Chan<Env, Right>> {
    unsafe {
      if self.read() {
        Branch::Left(transmute(self))
      } else {
        Branch::Right(transmute(self))
      }
    }
  }
}

impl<S> Chan<(), S>
where
  S: HasDual,
{
  /// Creates a pair of new channels (one for sending, one for receiving).
  /// Both channels are initialized with the empty environment `()`
  pub fn new() -> (Chan<(), S>, Chan<(), S::Dual>) {
    let (sender1, receiver1) = channel();
    let (sender2, receiver2) = channel();
    let c1 = Chan {
      sender: sender1,
      receiver: receiver2,
      _data: PhantomData,
    };
    let c2 = Chan {
      sender: sender2,
      receiver: receiver1,
      _data: PhantomData,
    };
    (c1, c2)
  }
}

impl<Env, S> Chan<Env, Rec<S>> {
  /// Enters a recursive session
  /// (pushes the crurent session type `S` onto the `Env` stack)
  pub fn rec_push(self) -> Chan<(S, Env), S> {
    unsafe { transmute(self) }
  }
}

impl<Env, S> Chan<(S, Env), Var<Z>> {
  /// Leaves a recursive session when we reach the closest recursion point (`Z`).
  /// Effectively, this removes the current sesison type `S` from the `Env` stack.
  pub fn rec_pop(self) -> Chan<(S, Env), S> {
    unsafe { transmute(self) }
  }
}

impl<Env, Sigma, N> Chan<(Sigma, Env), Var<S<N>>> {
  /// Leaves a recursive session
  /// (note that we go from the De Bruijn index `S<N>` to just `N`)
  pub fn rec_pop(self) -> Chan<Env, Var<N>> {
    unsafe { transmute(self) }
  }
}
