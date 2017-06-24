//! Marionette v3 protocol, see https://developer.mozilla.org/en-US/docs/Mozilla/QA/Marionette/Protocol
//!
//! This is a very simple synchronous implementation of the protocol.

use std::io;
use std::io::{BufRead, Write, Error, ErrorKind, BufReader};
use std::net::TcpStream;
use std::str;
use std::convert::From;
use std::str::FromStr;
use std::fmt;
use std::path::Path;
use std::env;

#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
use serde_json::{Value, from_str, from_value, to_value};
use serde_json::Error as JsonError;
extern crate serde;
use serde::Serialize;
use serde::de::DeserializeOwned;

pub use serde_json::Value as JsonValue;

#[derive(Debug)]
pub enum MarionetteError {
    Io(io::Error),
    JSON(JsonError),
    Call(ErrorObject),
    UnexpectedType,
    InvalidMsgId,
    InvalidResponseArray,
    UnsupportedProtocolVersion,
    UnsupportedContext(String),
}

impl MarionetteError {
    pub fn is_fatal(&self) -> bool {
        match *self {
            MarionetteError::Call(_) => false,
            MarionetteError::UnsupportedContext(_) => false,
            // Other errors are either Io errors or messages that do not follow the
            // protocol
            _ => true,
        }
    }
}

impl From<Error> for MarionetteError {
    fn from(err: Error) -> Self {
        MarionetteError::Io(err)
    }
}
impl From<JsonError> for MarionetteError {
    fn from(err: JsonError) -> Self {
        MarionetteError::JSON(err)
    }
}
impl fmt::Display for MarionetteError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            MarionetteError::Io(ref err) => err.fmt(f),
            MarionetteError::JSON(ref err) => err.fmt(f),
            MarionetteError::Call(ref err) => write!(f, "{}", err.message),
            MarionetteError::UnexpectedType => write!(f, "Found unexpected type in marionette message"),
            MarionetteError::InvalidMsgId => write!(f, "Invalid msg id in marionette message"),
            MarionetteError::InvalidResponseArray => write!(f, "Invalid response array in marionette message"),
            MarionetteError::UnsupportedProtocolVersion => write!(f, "Browser uses unsupported protocol version"),
            MarionetteError::UnsupportedContext(ref c) => write!(f, "Unsupported context: {}", c),
        }
    }
}

impl std::error::Error for MarionetteError {
    fn description(&self) -> &str {
        match *self {
            MarionetteError::Io(ref err) => err.description(),
            MarionetteError::JSON(ref err) => err.description(),
            MarionetteError::Call(_) => "The marionette API call failed",
            MarionetteError::UnexpectedType => "Found unexpected type in marionette message",
            MarionetteError::InvalidMsgId => "Invalid msg id in marionette message",
            MarionetteError::InvalidResponseArray => "Invalid response array in marionette message",
            MarionetteError::UnsupportedProtocolVersion => "Browser uses unsupported protocol version",
            MarionetteError::UnsupportedContext(_) => "Unsupported context",
        }
    }
}

pub type Result<T> = std::result::Result<T, MarionetteError>;

pub mod messages;
use messages::*;
pub use messages::{LogMsg, QueryMethod, WindowHandle, Script};

pub struct MarionetteConnection {
    reader: BufReader<TcpStream>,
    writer: TcpStream,
    msgid: u64,
}

impl MarionetteConnection {
    pub fn connect(port: u16) -> Result<Self> {
        let stream = TcpStream::connect(("127.0.0.1", port))?;
        let mut reader = BufReader::new(stream.try_clone()?);
        let frame = readframe(&mut reader)?;
        debug!("ServerInfo frame: {}", frame);
        let info: ServerInfo = from_str(&frame)?;
        if info.marionetteProtocol == 3 {
            let mut conn = MarionetteConnection {
                reader: reader,
                writer: stream,
                msgid: 0,
            };
            conn.new_session()?;

            Ok(conn)
        } else {
            Err(MarionetteError::UnsupportedProtocolVersion)
        }
    }

    fn next_msgid(&mut self) -> u64 {
        let next = self.msgid;
        self.msgid += 1;
        next
    }

    fn call<D, S>(&mut self, name: &str, args: S) -> Result<D> 
            where D: DeserializeOwned, S: Serialize {
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
                    _ => return Err(MarionetteError::UnexpectedType),
                }

                let resp_msgid = match drain.next().and_then(|v| Value::as_u64(&v)) {
                    Some(val) => val,
                    _ => return Err(MarionetteError::InvalidMsgId),
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
                        let err = from_value(err)?;
                        return Err(MarionetteError::Call(err));
                    }
                    None => return Err(MarionetteError::InvalidResponseArray),
                }

                match drain.next() {
                    None => return Err(MarionetteError::InvalidResponseArray),
                    Some(val) => return Ok(from_value(val)?),
                }
            } else {
               return Err(MarionetteError::UnexpectedType)
            }
        }
    }

    // AFAIK the semantics for newSession is that it should be called for each connection
    fn new_session(&mut self) -> Result<NewSessionResponse> {
        self.call("newSession", Empty {})
    }

    /// Refresh the current page
    pub fn refresh(&mut self) -> Result<()> {
        let _: Empty = self.call("refresh", Empty {})?;
        Ok(())
    }

    /// Go back to the previous page
    pub fn go_back(&mut self) -> Result<()> {
        let _: Empty = self.call("goBack", Empty {})?;
        Ok(())
    }

    /// Go forward to the next page in history
    pub fn go_forward(&mut self) -> Result<()> {
        let _: Empty = self.call("goForward", Empty {})?;
        Ok(())
    }

    /// Get the window title
    pub fn get_title(&mut self) -> Result<String> {
        let resp: ResponseValue<_> = self.call("getTitle", Empty {})?;
        Ok(resp.value)
    }

    /// Navigate to an URL
    pub fn get(&mut self, url: &str) -> Result<()> {
        let url_arg = to_value(GetCommand::from(url))?;
        let _: Empty = self.call("get", url_arg)?;
        Ok(())
    }

    /// Get the page url
    pub fn get_url(&mut self) -> Result<String> {
        let resp: ResponseValue<_> = self.call("getCurrentUrl", Empty {})?;
        Ok(resp.value)
    }

    /// Store a log in the marionette server
    pub fn log(&mut self, msg: LogMsg) -> Result<()> {
        let _: Empty = self.call("log", msg)?;
        Ok(())
    }

    /// Get all log entries from the server
    pub fn get_logs(&mut self) -> Result<Vec<LogEntry>> {
        self.call("getLogs", Empty {})
    }

    /// Returns the handle for the current window
    pub fn get_window_handle(&mut self) -> Result<WindowHandle> {
        let resp: ResponseValue<_> = self.call("getWindowHandle", Empty {})?;
        Ok(resp.value)
    }

    /// Returns a list of windows in the current context
    pub fn get_window_handles(&mut self) -> Result<Vec<WindowHandle>> {
        self.call("getWindowHandles", Empty {})
    }

    /// Switch to the specified window
    pub fn switch_to_window(&mut self, win: &WindowHandle) -> Result<()> {
        let _: Empty = self.call("switchToWindow", win)?;
        Ok(())
    }

    pub fn get_context(&mut self) -> Result<Context> {
        let resp = self.call("getContext", Empty {})?;
        Context::from_value(resp)
    }

    pub fn set_context(&mut self, ctx: Context) -> Result<()> {
        let arg: ContextValue = ctx.into();
        let _: Empty = self.call("setContext", arg)?;
        Ok(())
    }

    /// Execute the given script
    ///
    /// The return value is any JSON type returned by the script
    pub fn execute_script(&mut self, script: &Script) -> Result<JsonValue> {
        let resp: ResponseValue<_> = self.call("executeScript", script)?;
        Ok(resp.value)
    }

    /// Execute async script
    ///
    /// Scripts executed this way can terminate with a result using the function
    /// `marionetteScriptFinished(result)`.
    pub fn execute_async_script(&mut self, script: &Script) -> Result<JsonValue> {
        let resp: ResponseValue<_> = self.call("executeAsyncScript", script)?;
        Ok(resp.value)
    }

    /// Returns the page source
    pub fn get_page_source(&mut self) -> Result<String> {
        let resp: ResponseValue<_> = self.call("getPageSource", Empty {})?;
        Ok(resp.value)
    }

    /// Returns a list of HTML elements that match the given target
    pub fn find_elements(&mut self, method: QueryMethod, target: &str, inside: Option<&ElementRef>) -> Result<Vec<ElementRef>> {
        let query = FindElementQuery {
            value: target.to_owned(),
            using: method,
            element: inside.map(|elem| elem.reference.to_owned()),
        };
        self.call("findElements", query)
    }

    pub fn get_element_attribute(&mut self, elem: &ElementRef, attrname: &str) -> Result<String> {
        let arg = ElementOp {
            id: elem.reference.to_owned(),
            name: Some(attrname.to_owned()),
        };
        let resp: ResponseValue<_> = self.call("getElementAttribute", arg)?;
        Ok(resp.value)
    }

    pub fn get_element_property(&mut self, elem: &ElementRef, propname: &str) -> Result<JsonValue> {
        let arg = ElementOp {
            id: elem.reference.to_owned(),
            name: Some(propname.to_owned()),
        };
        let resp: ResponseValue<_> = self.call("getElementProperty", arg)?;
        Ok(resp.value)
    }

    pub fn get_element_text(&mut self, elem: &ElementRef) -> Result<String> {
        let arg = ElementOp {
            id: elem.reference.to_owned(),
            name: None,
        };
        let resp: ResponseValue<_> = self.call("getElementText", arg)?;
        Ok(resp.value)
    }

    pub fn get_active_frame(&mut self) -> Result<Option<ElementRef>> {
        let resp: ResponseValue<_> = self.call("getActiveFrame", Empty {})?;
        Ok(resp.value)
    }

    /// Switch to the given frame. If None switches to the top frame
    pub fn switch_to_frame(&mut self, elem: Option<ElementRef>) -> Result<()> {
        let arg = FrameSwitch::from_element(false, elem);
        let _: Empty = self.call("switchToFrame", arg)?;
        Ok(())
    }

    pub fn switch_to_parent_frame(&mut self) -> Result<()> {
        let _: Empty = self.call("switchToParentFrame", Empty {})?;
        Ok(())
    }

    /// Close the application
    pub fn quit(mut self) -> Result<()> {
        let _: Empty = self.call("quitApplication", Empty {})?;
        Ok(())
    }

    /// Install XPI from the given path
    pub fn addon_install(mut self, path: &Path) -> Result<()> {
        let abspath = if path.is_relative() {
            let mut absolute_path = try!(env::current_dir());
            absolute_path.push(path);
            absolute_path.to_owned()
        } else {
            path.into()
        };

        let _: Empty = self.call("addon:install", AddonInstall { path: &abspath })?;
        Ok(())
    }
}

/// A helper struct to work with `ElementRef`
pub struct Element<'a> {
    connection: &'a mut MarionetteConnection,
    id: ElementRef,
}

impl<'a> Element<'a> {
    pub fn new(connection: &'a mut MarionetteConnection, element: &ElementRef) -> Self {
        Element {
            connection: connection,
            id: element.clone(),
        }
    }

    /// Get element attribute
    pub fn attr(&mut self, name: &str) -> Result<String> {
        self.connection.get_element_attribute(&self.id, name)
    }

    /// Get element property
    pub fn property(&mut self, name: &str) -> Result<JsonValue> {
        self.connection.get_element_property(&self.id, name)
    }

    /// Get visible text for this element
    pub fn text(&mut self) -> Result<String> {
        self.connection.get_element_text(&self.id)
    }

    /// Find elements inside this element
    pub fn find_elements(&mut self, method: QueryMethod, target: &str) -> Result<Vec<ElementRef>> {
        self.connection.find_elements(method, target, Some(&self.id))
    }
}

/// Execution context
#[derive(Debug, PartialEq)]
pub enum Context {
    /// Web content, such as a frame
    Content,
    /// Browser specific context, alert dialogs and other windows
    Chrome,
}

impl Context {
    fn from_value(val: ContextValue) -> Result<Self> {
        match val.value.as_ref() {
            "chrome" => Ok(Context::Chrome),
            "content" => Ok(Context::Content),
            other => Err(MarionetteError::UnsupportedContext(other.to_owned())),
        }
    }
}

impl Into<ContextValue> for Context {
    fn into(self) -> ContextValue {
        match self {
            Context::Content => ContextValue { value: "content".to_owned() },
            Context::Chrome => ContextValue { value: "chrome".to_owned() },
        }
    }
}

/// Read data in the format `length:data`. The entire frame must be valid UTF8.
fn readframe<R: BufRead>(r: &mut R) -> io::Result<String> {
    let mut lenbuf = Vec::new();
    // Read length prefix
    let bytes = r.read_until(b':', &mut lenbuf)?;
    if bytes == 0 {
        return Err(Error::new(ErrorKind::InvalidData, "Invalid frame"));
    }

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
fn sendframe<W: Write>(w: &mut W, data: &str) -> io::Result<()> {
    debug!("-> {}", data);
    w.write_all(format!("{}:", data.len()).as_bytes())?;
    w.write_all(data.as_bytes())?;
    Ok(())
}
