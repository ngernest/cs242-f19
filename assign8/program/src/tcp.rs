#![allow(unused_imports, unused_variables, unused_parens)]
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

/// Projects the `seqno` field for each packet in a vector of packets
fn get_seq_nums(packets: &Vec<Packet>) -> Vec<usize> {
  packets.iter().map(|pk| pk.seqno).collect()
}

fn get_buffers(packets: &Vec<Packet>) -> Vec<Buffer> {
  packets.iter().map(|pk| pk.buf.clone()).collect()
}

pub fn tcp_server(c: Chan<(), TCPServer>) -> Vec<Buffer> {
  // Handshake
  let (c, syn) = c.recv();
  let c = c.send(SynAck);
  let (c, ack) = c.recv();

  // Data transfer
  let mut c = c.rec_push();
  loop {
    c = {
      // Data transfer process
      let (c, mut packets) = c.recv();
      let seq_nums = get_seq_nums(&packets);
      let c = c.send(seq_nums);

      match c.offer() {
        Branch::Left(c) => {
          // Close connection
          let c = c.send(Ack);
          let c = c.send(Fin);
          let (c, ack) = c.recv();
          c.close();

          // Sort packets in increasing order of `seqno`
          packets.sort_by(|p1, p2| p1.seqno.cmp(&p2.seqno));

          // Project out all the buffers
          let buffers = get_buffers(&packets);
          return buffers;
        }
        Branch::Right(c) => {
          // Restart data transfer process
          // Recurse by popping a session type off the `Env` stakc
          c.rec_pop()
        }
      }
    }
  }
}

pub fn tcp_client(c: Chan<(), TCPClient>, bufs: Vec<Buffer>) {
  // let mut c = c.send(Syn);
  // let (mut c, syn_ack) = c.recv();
  // c = c.send(Ack);

  // for (seqno, buffer) in bufs.iter().enumerate() {
  //   let packet = Packet {
  //     buf: buffer.to_vec(),
  //     seqno,
  //   };
  //   c.send(packet)
  // }
}

#[cfg(test)]
mod test {
  use crate::session::NOISY;
  use crate::session::*;
  use crate::tcp::*;
  use rand;
  use rand::Rng;
  use std::sync::atomic::Ordering;
  use std::thread;

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
