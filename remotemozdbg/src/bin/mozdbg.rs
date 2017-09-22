
use std::net::{TcpStream, SocketAddr};
use std::io::BufReader;

extern crate remotemozdbg;
use remotemozdbg::proto::*;

extern crate env_logger;

fn main() {
    let _ = env_logger::init();
    let addr: SocketAddr = "127.0.0.1:1234".parse()
        .unwrap();
    let s = TcpStream::connect(addr)
        .unwrap();

    let mut sender = s.try_clone()
        .unwrap();
    let mut reader = BufReader::new(s);

    let pkt = readpacket(&mut reader).unwrap();
    println!("start packet {:?}", pkt);

    sendpacket(&mut sender, &ListTabsRequest::new()).unwrap();

    loop {
        let pkt = readpacket(&mut reader)
            .unwrap();

        println!("{:?}", pkt);
    }
}
