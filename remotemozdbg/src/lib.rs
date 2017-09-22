
#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate serde;

pub mod proto;

use std::net::{TcpStream, SocketAddr, ToSocketAddrs};
use std::io::{self, BufRead, BufReader};
use std::thread;
use std::sync::{Mutex, Arc};

pub struct RemoteState {
}

pub struct RemoteDebugger {
    reader: BufReader<TcpStream>,
    writer: TcpStream,
    state: Arc<Mutex<RemoteState>>,
}

impl RemoteDebugger {
    pub fn new(addr: &str) -> io::Result<Arc<Mutex<RemoteState>>> {
        let addr: SocketAddr = addr.parse()
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        let s = TcpStream::connect(addr)?;
        let writer = s.try_clone()?;
        let reader = BufReader::new(s);

        let state = Arc::new(Mutex::new(RemoteState {}));
        let mut dbg = RemoteDebugger {
            reader,
            writer,
            state: state.clone(),
        };

        thread::spawn(move || dbg.run());

        Ok(state)
    }

    pub fn run(&mut self) {
        let first = proto::readpacket(&mut self.reader);
        //self.process_first_pkt(first);
        loop {
            let pkt = proto::readpacket(&mut self.reader);
        }
    }
}
