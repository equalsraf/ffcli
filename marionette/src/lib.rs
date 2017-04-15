//! Marionette v3 protocol, see https://developer.mozilla.org/en-US/docs/Mozilla/QA/Marionette/Protocol
//!
//! This is a very simple synchronous implementation of the protocol.

use std::io;
use std::io::{BufRead, Write, Error, ErrorKind, BufReader};
use std::net::TcpStream;
use std::str;
use std::convert::From;
use std::str::FromStr;

#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
use serde_json::{Value, from_str, from_value, to_value};
use serde_json::Error as JsonError;
extern crate serde;
use serde::{Deserialize, Serialize};

#[derive(Debug)]
enum CallError {
    Io(io::Error),
    JSON(JsonError),
    Call(ErrorObject),
    UnexpectedType,
    InvalidMsgId,
    InvalidResponseArray,
    InvalidErrorObject,
}

impl CallError {
    /// Convert a CallError into an std::io::Error
    fn into_err(self) -> Error {
        match self {
            CallError::Io(err) => err,
            CallError::JSON(err) => Error::new(ErrorKind::InvalidData, err),
            CallError::Call(err) =>
                Error::new(ErrorKind::InvalidData, format!("{}: {}", err.error, err.message)),
            _ => Error::new(ErrorKind::InvalidData, "Invalid response message"),
        }
    }
}

impl From<Error> for CallError {
    fn from(err: Error) -> Self {
        CallError::Io(err)
    }
}
impl From<JsonError> for CallError {
    fn from(err: JsonError) -> Self {
        CallError::JSON(err)
    }
}

pub mod messages;
use messages::*;
pub use messages::{LogMsg};

pub struct MarionetteConnection {
    reader: BufReader<TcpStream>,
    writer: TcpStream,
    msgid: u64,
}

impl MarionetteConnection {
    pub fn connect(port: u16) -> io::Result<Self> {
        let stream = TcpStream::connect(("127.0.0.1", port))?;
        let mut reader = BufReader::new(stream.try_clone()?);
        let frame = readframe(&mut reader)?;
        debug!("ServerInfo frame: {}", frame);
        let info: ServerInfo = from_str(&frame)
            .map_err(|_| Error::new(ErrorKind::InvalidData, "Invalid JSON in server info"))?;
        if info.marionetteProtocol == 3 {
            let mut conn = MarionetteConnection {
                reader: reader,
                writer: stream,
                msgid: 0,
            };
            conn.new_session()?;

            Ok(conn)
        } else {
            Err(Error::new(ErrorKind::InvalidData, "Unsupported marionette protocol version"))
        }
    }

    fn next_msgid(&mut self) -> u64 {
        let next = self.msgid;
        self.msgid += 1;
        next
    }

    fn call<D, S>(&mut self, name: &str, args: S) -> Result<D, CallError> 
            where D: Deserialize, S: Serialize {
        let mut cmdarr = Vec::new();
        let msgid = self.next_msgid();
        cmdarr.push(Value::from(0));
        cmdarr.push(Value::from(msgid));
        cmdarr.push(Value::from(name));
        cmdarr.push(to_value(args)?);
        let msg = Value::Array(cmdarr);

        sendframe(&mut self.writer, &msg.to_string())?;

        loop {
            let resp = readframe(&mut self.reader)?;
            debug!("<- {}", resp);
            if let Value::Array(mut arr) = Value::from_str(&resp)? {

                let mut drain = arr.drain(0..);

                match drain.next().and_then(|v| Value::as_u64(&v)) {
                    // Only command responses(1) are valid
                    Some(1) => (),
                    _ => return Err(CallError::UnexpectedType),
                }

                let resp_msgid = match drain.next().and_then(|v| Value::as_u64(&v)) {
                    Some(val) => val,
                    _ => return Err(CallError::InvalidMsgId),
                };

                if resp_msgid != msgid {
                    // For some reason we got a response with a mismatching id,
                    // strange since this is a synchronous client
                    debug!("Received unexpected msgid({}): {}", resp_msgid, resp);
                    continue;
                }

                match drain.next() {
                    Some(Value::Null) => (),
                    Some(err) => {
                        let err = from_value(err)
                            .map_err(|_| CallError::InvalidErrorObject)?;
                        return Err(CallError::Call(err));
                    }
                    None => return Err(CallError::InvalidResponseArray),
                }

                match drain.next() {
                    None => return Err(CallError::InvalidResponseArray),
                    Some(val) => return Ok(from_value(val)?),
                }
            } else {
               return Err(CallError::UnexpectedType)
            }
        }
    }

    // AFAIK the semantics for newSession is that it should be called for each connection
    fn new_session(&mut self) -> io::Result<NewSessionResponse> {
        let resp = self.call("newSession", Empty {}).map_err(CallError::into_err)?;
        Ok(resp)
    }

    /// Refresh the current page
    pub fn refresh(&mut self) -> io::Result<()> {
        let _: Empty = self.call("refresh", Empty {}).map_err(CallError::into_err)?;
        Ok(())
    }

    /// Go back to the previous page
    pub fn go_back(&mut self) -> io::Result<()> {
        let _: Empty = self.call("goBack", Empty {}).map_err(CallError::into_err)?;
        Ok(())
    }

    /// Go forward to the next page in history
    pub fn go_forward(&mut self) -> io::Result<()> {
        let _: Empty = self.call("goForward", Empty {}).map_err(CallError::into_err)?;
        Ok(())
    }

    /// Get the page title
    pub fn get_title(&mut self) -> io::Result<String> {
        let resp: ResponseValue<_> = self.call("getTitle", Empty {}).map_err(CallError::into_err)?;
        Ok(resp.value)
    }

    /// Navigate to an URL
    pub fn get(&mut self, url: &str) -> io::Result<()> {
        let url_arg = to_value(GetCommand::from(url))
            .map_err(|err| Error::new(ErrorKind::Other, err))?;
        let _: Empty = self.call("get", url_arg)
            .map_err(CallError::into_err)?;
        Ok(())
    }

    /// Get the page url
    pub fn get_url(&mut self) -> io::Result<String> {
        let resp: ResponseValue<_> = self.call("getCurrentUrl", Empty {}).map_err(CallError::into_err)?;
        Ok(resp.value)
    }

    /// Store a log in the marionette server
    pub fn log(&mut self, msg: LogMsg) -> io::Result<()> {
        let _: Empty = self.call("log", msg).map_err(CallError::into_err)?;
        Ok(())
    }

    /// Get all log entries from the server
    pub fn get_logs(&mut self) -> io::Result<Vec<LogEntry>> {
        let resp = self.call("getLogs", Empty {}).map_err(CallError::into_err)?;
        Ok(resp)
    }
}


/// Read data in the format `length:data`. The entire frame must be valid UTF8.
fn readframe<R: BufRead>(r: &mut R) -> Result<String, io::Error> {
    let mut lenbuf = Vec::new();
    // Read length prefix
    let bytes = r.read_until(b':', &mut lenbuf)?;
    let len_str = str::from_utf8(&lenbuf[..bytes-1])
        .map_err(|_| Error::new(ErrorKind::InvalidData, "Invalid utf8 in frame length"))?;
    let len = usize::from_str_radix(len_str, 10)
        .map_err(|_| Error::new(ErrorKind::InvalidData, "Invalid frame length"))?;

    let mut buf = Vec::with_capacity(len);
    buf.resize(len, 0);
    r.read_exact(buf.as_mut_slice())?;
    String::from_utf8(buf)
        .map_err(|_| Error::new(ErrorKind::InvalidData, "Invalid utf8 in frame data"))
}

/// Prepend string with length: and push it down the wire
fn sendframe<W: Write>(w: &mut W, data: &str) -> Result<(), io::Error> {
    debug!("-> {}", data);
    w.write_all(format!("{}:", data.len()).as_bytes())?;
    w.write_all(data.as_bytes())?;
    Ok(())
}
