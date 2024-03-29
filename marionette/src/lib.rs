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
            MarionetteError::Call(ref err) => write!(f, "API call failed: {}, {}", err.error, err.message),
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
pub use messages::{LogMsg, QueryMethod, WindowHandle, Script, Timeouts, Cookie};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Compatibility {
    /// The old version of the protocol, pre WebDriver:
    Marionette,
    /// The new marionette protocol - all commands are
    /// prefixed with  WebDriver: or Marionette:
    Webdriver,
}

pub struct MarionetteConnection {
    reader: BufReader<TcpStream>,
    writer: TcpStream,
    msgid: u64,
    timeouts: Option<Timeouts>,
    compatibility: Compatibility,
}

impl MarionetteConnection {
    pub fn compatibility(&self) -> Compatibility { self.compatibility }

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
                timeouts: None,
                compatibility: Compatibility::Webdriver,
            };
            // TODO store the whole capabilities object instead
            let options = NewSessionRequest::new();
            let (resp, compat) = match conn.new_session_webdriver(&options) {
                Ok(resp) => (resp, conn.compatibility),
                Err(err) => {
                    debug!("Failed to establish new session, will retry with old protocol: {}", err);
                    // Retry with the new old protocol
                    (conn.new_session(&options)?, Compatibility::Marionette)
                }
            };

            conn.compatibility = compat;
            conn.timeouts = resp.capabilities.timeouts;

            // Try to make sure the browser is live before returning
            for retry in 0..4 {
                match conn.get_title() {
                    Ok(_) => break,
                    Err(err) => {
                        debug!("#{} Failed to connect to firefox({}): {}", retry, port, err);
                        std::thread::sleep(std::time::Duration::new(retry*2, 0));
                    }
                }
            }

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
    fn new_session(&mut self, options: &NewSessionRequest) -> Result<NewSessionResponse> {
        self.call("newSession", options)
    }

    fn new_session_webdriver(&mut self, options: &NewSessionRequest) -> Result<NewSessionResponse> {
        self.call("WebDriver:NewSession", options)
    }

    /// Refresh the current page
    pub fn refresh(&mut self) -> Result<()> {
        let _: Empty = match self.compatibility {
            Compatibility::Marionette => self.call("refresh", Empty {})?,
            Compatibility::Webdriver => self.call("WebDriver:Refresh", Empty {})?,
        };
        Ok(())
    }

    /// Go back to the previous page
    pub fn go_back(&mut self) -> Result<()> {
        let _: Empty = match self.compatibility {
            Compatibility::Marionette => self.call("goBack", Empty {})?,
            Compatibility::Webdriver => self.call("WebDriver:Back", Empty {})?,
        };
        Ok(())
    }

    /// Go forward to the next page in history
    pub fn go_forward(&mut self) -> Result<()> {
        let _: Empty = match self.compatibility {
            Compatibility::Marionette => self.call("goForward", Empty {})?,
            Compatibility::Webdriver => self.call("WebDriver:Forward", Empty {})?,
        };
        Ok(())
    }

    /// Get the window title
    pub fn get_title(&mut self) -> Result<String> {
        let resp: ResponseValue<_> = match self.compatibility {
            Compatibility::Marionette => self.call("getTitle", Empty {})?,
            Compatibility::Webdriver => self.call("WebDriver:GetTitle", Empty {})?,
        };
        Ok(resp.value)
    }

    /// Navigate to an URL
    pub fn get(&mut self, url: &str) -> Result<()> {
        let url_arg = to_value(GetCommand::from(url))?;
        let _: Empty = match self.compatibility {
            Compatibility::Marionette => self.call("get", url_arg)?,
            Compatibility::Webdriver => self.call("WebDriver:Navigate", url_arg)?,
        };
        Ok(())
    }
    pub fn navigate(&mut self, url: &str) -> Result<()> {
        self.get(url)
    }

    /// Get the page url
    pub fn get_url(&mut self) -> Result<String> {
        let resp: ResponseValue<_> = match self.compatibility {
            Compatibility::Marionette => self.call("getCurrentUrl", Empty {})?,
            Compatibility::Webdriver => self.call("WebDriver:GetCurrentURL", Empty {})?,
        };
        Ok(resp.value)
    }

    /// Returns the handle for the current window
    pub fn get_window_handle(&mut self) -> Result<WindowHandle> {
        let resp: ResponseValue<_> = match self.compatibility {
            Compatibility::Marionette => self.call("getWindowHandle", Empty {})?,
            Compatibility::Webdriver => self.call("WebDriver:GetWindowHandle", Empty {})?,
        };
        Ok(resp.value)
    }

    /// Returns a list of windows in the current context
    pub fn get_window_handles(&mut self) -> Result<Vec<WindowHandle>> {
        match self.compatibility {
            Compatibility::Marionette => self.call("getWindowHandles", Empty {}),
            Compatibility::Webdriver => self.call("WebDriver:GetWindowHandles", Empty {}),
        }
    }

    /// Switch to the specified window
    pub fn switch_to_window(&mut self, win: &WindowHandle) -> Result<()> {
        let _: Empty = match self.compatibility {
            Compatibility::Marionette => self.call("switchToWindow", win)?,
            Compatibility::Webdriver => self.call("WebDriver:SwitchToWindow", win)?,
        };
        Ok(())
    }

    pub fn get_context(&mut self) -> Result<Context> {
        let resp = match self.compatibility {
            Compatibility::Marionette => self.call("getContext", Empty {})?,
            Compatibility::Webdriver => self.call("Marionette:GetContext", Empty {})?,
        };
        Context::from_value(resp)
    }

    pub fn set_context(&mut self, ctx: Context) -> Result<()> {
        let arg: ContextValue = ctx.into();
        let _: Empty = match self.compatibility {
            Compatibility::Marionette => self.call("setContext", arg)?,
            Compatibility::Webdriver => self.call("Marionette:SetContext", arg)?,
        };
        Ok(())
    }

    /// Execute the given script
    ///
    /// The return value is any JSON type returned by the script
    pub fn execute_script(&mut self, script: &Script) -> Result<JsonValue> {
        let resp: ResponseValue<_> = match self.compatibility {
            Compatibility::Marionette => self.call("executeScript", script)?,
            Compatibility::Webdriver => self.call("WebDriver:ExecuteScript", script)?,
        };
        Ok(resp.value)
    }

    /// Sets global timeouts for various operations
    pub fn set_timeouts(&mut self, t: Timeouts) -> Result<()> {
        let _: Empty = match self.compatibility {
            Compatibility::Marionette => self.call("timeouts", t)?,
            Compatibility::Webdriver => self.call("WebDriver:SetTimeouts", t)?,
        };
        self.timeouts = Some(t);
        Ok(())
    }

    pub fn timeouts(&self) -> Option<&Timeouts> {
        self.timeouts.as_ref()
    }

    /// Execute async script
    ///
    /// Scripts executed this way can terminate with a result using the function
    /// `marionetteScriptFinished(result)`.
    pub fn execute_async_script(&mut self, script: &Script) -> Result<JsonValue> {
        let resp: ResponseValue<_> = match self.compatibility {
            Compatibility::Marionette => self.call("executeAsyncScript", script)?,
            Compatibility::Webdriver => self.call("WebDriver:ExecuteAsyncScript", script)?,
        };
        Ok(resp.value)
    }

    /// Returns the page source
    pub fn get_page_source(&mut self) -> Result<String> {
        let resp: ResponseValue<_> = match self.compatibility {
            Compatibility::Marionette => self.call("getPageSource", Empty {})?,
            Compatibility::Webdriver => self.call("WebDriver:GetPageSource", Empty {})?,
        };
        Ok(resp.value)
    }

    /// Returns a list of HTML elements that match the given target
    pub fn find_elements(&mut self, method: QueryMethod, target: &str, inside: Option<&ElementRef>) -> Result<Vec<ElementRef>> {
        let query = FindElementQuery {
            value: target.to_owned(),
            using: method,
            element: inside.map(|elem| elem.reference.to_owned()),
        };
        match self.compatibility {
            Compatibility::Marionette => self.call("findElements", query),
            Compatibility::Webdriver => self.call("WebDriver:FindElements", query),
        }
    }

    pub fn get_element_attribute(&mut self, elem: &ElementRef, attrname: &str) -> Result<Option<String>> {
        let arg = ElementOp {
            id: elem.reference.to_owned(),
            name: Some(attrname.to_owned()),
        };
        let resp: ResponseValue<_> = match self.compatibility {
            Compatibility::Marionette => self.call("getElementAttribute", arg)?,
            Compatibility::Webdriver => self.call("WebDriver:GetElementAttribute", arg)?,
        };
        Ok(resp.value)
    }

    pub fn get_element_property(&mut self, elem: &ElementRef, propname: &str) -> Result<JsonValue> {
        let arg = ElementOp {
            id: elem.reference.to_owned(),
            name: Some(propname.to_owned()),
        };
        let resp: ResponseValue<_> = match self.compatibility {
            Compatibility::Marionette => self.call("getElementProperty", arg)?,
            Compatibility::Webdriver => self.call("WebDriver:GetElementProperty", arg)?,
        };
        Ok(resp.value)
    }

    pub fn get_element_text(&mut self, elem: &ElementRef) -> Result<String> {
        let arg = ElementOp {
            id: elem.reference.to_owned(),
            name: None,
        };
        let resp: ResponseValue<_> = match self.compatibility {
            Compatibility::Marionette => self.call("getElementText", arg)?,
            Compatibility::Webdriver => self.call("WebDriver:GetElementText", arg)?,
        };
        Ok(resp.value)
    }

    pub fn get_active_frame(&mut self) -> Result<Option<ElementRef>> {
        let resp: ResponseValue<_> = match self.compatibility {
            Compatibility::Marionette => self.call("getActiveFrame", Empty {})?,
            Compatibility::Webdriver => self.call("WebDriver:GetActiveFrame", Empty {})?,
        };
        Ok(resp.value)
    }

    /// Switch to the given frame. If None switches to the top frame
    pub fn switch_to_frame(&mut self, elem: Option<ElementRef>) -> Result<()> {
        let arg = FrameSwitch::from_element(false, elem);
        let _: Empty = match self.compatibility {
            Compatibility::Marionette => self.call("switchToFrame", arg)?,
            Compatibility::Webdriver => self.call("WebDriver:SwitchToFrame", arg)?,
        };
        Ok(())
    }

    pub fn switch_to_parent_frame(&mut self) -> Result<()> {
        let _: Empty = match self.compatibility {
            Compatibility::Marionette => self.call("switchToParentFrame", Empty {})?,
            Compatibility::Webdriver => self.call("WebDriver:SwitchToParentFrame", Empty {})?,
        };
        Ok(())
    }

    /// Close the application
    pub fn quit(mut self) -> Result<()> {
        let _: Empty = match self.compatibility {
            Compatibility::Marionette => self.call("quitApplication", Empty {})?,
            Compatibility::Webdriver => self.call("Marionette:Quit", Empty {})?,
        };
        Ok(())
    }

    /// Install XPI from the given path
    pub fn addon_install(mut self, path: &Path) -> Result<()> {
        let abspath = if path.is_relative() {
            let mut absolute_path = env::current_dir()?;
            absolute_path.push(path);
            absolute_path.to_owned()
        } else {
            path.into()
        };

        let arg = AddonInstall { path: &abspath };
        let _: Empty = match self.compatibility {
            Compatibility::Marionette => self.call("addon:install", arg)?,
            Compatibility::Webdriver => self.call("Addon:Install", arg)?,
        };
        Ok(())
    }

    fn with_context<T, F>(&mut self, ctx: Context, f: F) -> Result<T>
            where F: FnOnce(&mut MarionetteConnection) -> Result<T> {
        let prev = self.get_context()?;
        self.set_context(ctx)?;
        let res = f(self);
        if let Err(ctxerr) = self.set_context(prev.clone()) {
            warn!("Error resetting context to {:?}: {}", prev, ctxerr);
        }
        res
    }

    pub fn set_pref(&mut self, name: &str, value: JsonValue) -> Result<()> {
        let mut script = Script::new(r#"
        Components.utils.import("resource://gre/modules/Preferences.jsm");
        let [pref, value, defaultBranch] = arguments;
        prefs = new Preferences({defaultBranch: defaultBranch});
        prefs.set(pref, value);
        "#);
        script.arguments((name, value, false))?;
        script.sandbox("system");

        self.with_context(Context::Chrome, move |conn| {
            conn.execute_script(&script)?;
            Ok(())
        })
    }

    pub fn get_pref(&mut self, name: &str) -> Result<JsonValue> {
        let mut script = Script::new(r#"
        Components.utils.import("resource://gre/modules/Preferences.jsm");
        let [pref, defaultBranch, valueType] = arguments;
        prefs = new Preferences({defaultBranch: defaultBranch});
        return prefs.get(pref, null, valueType=Components.interfaces[valueType]);
        "#);
        script.arguments((name, false, "nsISupportsString"))?;
        script.sandbox("system");

        self.with_context(Context::Chrome, move |conn| {
            conn.execute_script(&script)
        })
    }

    pub fn add_cookie(&mut self, cookie: &Cookie) -> Result<Empty> {
        self.call("WebDriver:AddCookie", AddCookie { cookie: cookie })
    }

    /// Get a list of cookies
    pub fn get_cookies(&mut self) -> Result<Vec<Cookie>> {
        self.call("WebDriver:GetCookies", Empty {})
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
    pub fn attr(&mut self, name: &str) -> Result<Option<String>> {
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
#[derive(Debug, PartialEq, Clone, Copy)]
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
