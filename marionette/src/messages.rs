//! Structures for some of the messages used in the Marionette protocol, these can
//! be used with the traits in serde to convert into the corresponding json.
//!
#![allow(non_snake_case)]

use serde::{Serialize, Serializer};
use serde::ser::SerializeStruct;
use serde_json::Value;

#[derive(Deserialize, Debug)]
pub struct ServerInfo {
    pub marionetteProtocol: u64,
}

#[derive(Deserialize, Debug)]
pub struct ErrorObject {
    pub error: String,
    pub message: String,
    pub stacktrace: String,
}

#[derive(Deserialize, Debug)]
pub struct NewSessionResponse {
    pub sessionId: String,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct Empty {}

/// Some responses use a type wrapped in a json object
/// with the value attribute
#[derive(Deserialize, Serialize, Debug)]
pub struct ResponseValue<T> {
    pub value: T,
}

#[derive(Serialize, Debug)]
pub struct GetCommand {
    pub url: String,
}
impl GetCommand {
    pub fn from(url: &str) -> Self {
        Self { url: url.to_owned() }
    }
}

/// A log message to push to the marionette server. The message
/// includes an arbitrary level (INFO, DEBUG, etc).
#[derive(Serialize, Debug)]
pub struct LogMsg {
    value: String,
    level: String,
}

impl LogMsg {
    pub fn new(msg: &str, lvl: &str) -> Self {
        LogMsg {
            value: msg.to_owned(),
            level: lvl.to_owned(),
        }
    }
}

/// A log entry as returned by the getLogs command. This includes a message,
/// an arbitrary log level and a date.
#[derive(Deserialize, Debug)]
pub struct LogEntry(String, String, String);

impl LogEntry {
    pub fn level(&self) -> &str { &self.0 }
    pub fn msg(&self) -> &str { &self.1 }
}

/// An opaque handle to a window
///
/// This is deserialized from a regular string. But serialization creates
/// an object `{'name': 'handle'}`.
#[derive(Deserialize, Debug, PartialEq)]
pub struct WindowHandle(String);

impl Serialize for WindowHandle {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut ss = s.serialize_struct("WindowHandle", 1)?;
        ss.serialize_field("name", &self.0)?;
        ss.end()
    }
}

/// The execution context
pub type ContextValue = ResponseValue<String>;

#[derive(Serialize, Debug)]
pub struct Script {
    script: String,
    args: Value,
}

impl Script {
    pub fn new(src: &str) -> Self {
        Script {
            script: src.to_owned(),
            args: Value::Null,
        }
    }

    /// Set arguments for this script. This is usually an array that
    /// is used as the `arguments` variable.
    pub fn arguments<V: Into<Value>>(&mut self, args: V) {
        self.args = args.into();
    }
}
