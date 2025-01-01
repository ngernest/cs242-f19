#![allow(unused_imports, unused_variables)]
extern crate rand;

use crate::session::*;
use std::collections::{HashMap, HashSet};

/// Synchronize
pub struct Syn;

/// Synchronize-Acknowledgement
pub struct SynAck;

/// Acknowledgement
pub struct Ack;

/// Close connection
pub struct Fin;

/// Handshake phase
pub type TCPHandshake<TCPRecv> = Recv<Syn, Send<SynAck, Recv<Ack, TCPRecv>>>;

/// Data transfer phase
pub type TCPRecv<TCPClose> = Rec<Recv<Vec<Packet>, Send<Vec<usize>, Offer<TCPClose, Var<Z>>>>>;

/// Close phase
pub type TCPClose = Send<Ack, Send<Fin, Recv<Ack, Close>>>;

pub type TCPServer = TCPHandshake<TCPRecv<TCPClose>>;

pub type TCPClient = <TCPServer as HasDual>::Dual;

pub fn tcp_server(c: Chan<(), TCPServer>) -> Vec<Buffer> {
  unimplemented!();
}

pub fn tcp_client(c: Chan<(), TCPClient>, bufs: Vec<Buffer>) {
  unimplemented!();
}

#[cfg(test)]
mod test {
  use rand;
  use rand::Rng;
  use session::NOISY;
  use session::*;
  use std::sync::atomic::Ordering;
  use std::thread;
  use tcp::*;

  fn gen_bufs() -> Vec<Buffer> {
    let mut bufs: Vec<Buffer> = Vec::new();
    let mut rng = rand::thread_rng();
    for _ in 0usize..20 {
      let buf: Buffer = vec![0; rng.gen_range(1, 10)];
      let buf: Buffer = buf.into_iter().map(|_| rng.gen()).collect();
      bufs.push(buf);
    }
    bufs
  }

  #[test]
  fn test_basic() {
    let bufs = gen_bufs();
    let bufs_copy = bufs.clone();
    let (s, c): ((Chan<(), TCPServer>), (Chan<(), TCPClient>)) = Chan::new();
    let thread = thread::spawn(move || {
      tcp_client(c, bufs);
    });

    let recvd = tcp_server(s);
    thread.join().unwrap();

    assert_eq!(recvd, bufs_copy);
  }

  #[test]
  fn test_lossy() {
    let bufs = gen_bufs();
    let bufs_copy = bufs.clone();

    NOISY.with(|noisy| {
      noisy.store(true, Ordering::SeqCst);
    });

    let (s, c): ((Chan<(), TCPServer>), (Chan<(), TCPClient>)) = Chan::new();
    let thread = thread::spawn(move || {
      tcp_client(c, bufs);
    });

    let recvd = tcp_server(s);
    thread.join().unwrap();

    assert_eq!(recvd, bufs_copy);
  }
}
