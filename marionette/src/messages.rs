//! Structures for some of the messages used in the Marionette protocol, these can
//! be used with the traits in serde to convert into the corresponding json.
//!
#![allow(non_snake_case)]

use std::fmt;
use std::path::Path;
use std::collections::HashMap;
use serde::{Serialize, Serializer, Deserialize, Deserializer};
use serde::ser::SerializeStruct;
use serde_json::{Value, to_value};
use serde::de::{Visitor, MapAccess};
use serde::de::Error as DeError;
use super::MarionetteError;

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

pub enum Capability {
    PageLoadStrategy(String),
}

#[derive(Serialize, Debug)]
pub struct CapabilityRequest {
    requiredCapabilities: HashMap<String, Value>,
}

#[derive(Deserialize, Debug)]
pub struct Capabilities {
    pub timeouts: Option<Timeouts>,
}

#[derive(Serialize, Debug)]
pub struct NewSessionRequest {
    capabilities: CapabilityRequest,
}

impl NewSessionRequest {
    pub fn new() -> Self {
        NewSessionRequest {
            capabilities: CapabilityRequest {
                requiredCapabilities: HashMap::new(),
            }
        }
    }

    pub fn required(&mut self, cap: Capability) {
        match cap {
            Capability::PageLoadStrategy(s) =>
                self.capabilities.requiredCapabilities.insert("pageLoadStrategy".to_string(), Value::String(s)),
        };
    }
}

#[derive(Deserialize, Debug)]
pub struct NewSessionResponse {
    pub sessionId: String,
    pub capabilities: Capabilities,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct Empty {}

/// Sets various timeout parameters (in ms)
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub struct Timeouts {
    /// when to interrupt a script that is being evaluated
    pub script: u64,
    /// the timeout limit used to interrupt navigation of the browsing context
    pub pageLoad: u64,
    /// the timeout of when to abort when locating an element
    pub implicit: u64,
}

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

impl WindowHandle {
    pub fn from_str(handle: &str) -> Self {
        WindowHandle(handle.to_owned())
    }
}

impl fmt::Display for WindowHandle {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Serialize for WindowHandle {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut ss = s.serialize_struct("WindowHandle", 1)?;
        ss.serialize_field("name", &self.0)?;
        // Starting with firefox 81, name is ignored and
        // handle is used instead
        ss.serialize_field("handle", &self.0)?;
        ss.end()
    }
}

/// The execution context
pub type ContextValue = ResponseValue<String>;

#[derive(Serialize, Debug)]
pub struct Script {
    script: String,
    sandbox: String,
    args: Value,
    scriptTimeout: Option<u64>,
}

impl Script {
    pub fn new(src: &str) -> Self {
        Script {
            script: src.to_owned(),
            sandbox: "default".to_owned(),
            // execute_script accepts null here, but execute_async_script does not
            // default to an empty array
            args: Value::Array(Vec::new()),
            scriptTimeout: None,
        }
    }

    /// Set arguments for this script. This is usually an array that
    /// is used as the `arguments` variable.
    pub fn arguments<S: Serialize>(&mut self, args: S) -> Result<(), MarionetteError>{
        self.args = to_value(args)?;
        Ok(())
    }

    /// Execute the script in a named sandbox
    pub fn sandbox(&mut self, name: &str) {
        self.sandbox = name.to_owned()
    }

    /// Set execution timeout for script (ms)
    ///
    /// This value overrides the global scriptTimeout.
    ///
    /// This option was removed from firefox in Jan/2019, see
    ///
    ///    9ed472d43600ca6ba1ced8a563dbaa4abdef5eaa
    ///
    /// https://bugzilla.mozilla.org/show_bug.cgi?id=1510929
    /// https://phabricator.services.mozilla.com/D15584
    ///
    #[deprecated = "Unsupported since Jan/2009 see bug 1510929"]
    pub fn timeout(&mut self, timeout_ms: u64) {
        self.scriptTimeout = Some(timeout_ms)
    }
}

#[derive(Debug)]
pub enum QueryMethod {
    Id,
    Name,
    ClassName,
    TagName,
    CssSelector,
    LinkText,
    PartialLinkText,
    XPath,
}

impl Serialize for QueryMethod {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            &QueryMethod::Id => s.serialize_str("id"),
            &QueryMethod::Name => s.serialize_str("name"),
            &QueryMethod::ClassName => s.serialize_str("class name"),
            &QueryMethod::TagName => s.serialize_str("tag name"),
            &QueryMethod::CssSelector => s.serialize_str("css selector"),
            &QueryMethod::LinkText => s.serialize_str("link text"),
            &QueryMethod::PartialLinkText => s.serialize_str("partial link text"),
            &QueryMethod::XPath => s.serialize_str("xpath"),
        }
    }
}

#[derive(Serialize, Debug)]
pub struct FindElementQuery {
    /// A query
    pub value: String,
    /// The method use to perform the query
    pub using: QueryMethod,
    // In recent versions of firefox (60) this field must not
    // be set to null, skip it instead
    #[serde(skip_serializing_if = "Option::is_none")]
    pub element: Option<String>,
}

#[derive(PartialEq, Debug, Clone)]
pub struct ElementRef {
    pub reference: String,
}

impl ElementRef {
    pub fn from_str(handle: &str) -> ElementRef {
        ElementRef { reference: handle.to_string() }
    }
}

impl Serialize for ElementRef {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut ss = s.serialize_struct("ElementRef", 2)?;
        ss.serialize_field("ELEMENT", &self.reference)?;
        ss.serialize_field("element-6066-11e4-a52e-4f735466cecf", &self.reference)?;
        ss.end()
    }
}

impl<'a> Deserialize<'a> for ElementRef {
    fn deserialize<D: Deserializer<'a>>(d: D) -> Result<Self, D::Error> {
        enum Field { Reference, Ignored };

        impl<'b> Deserialize<'b> for Field {
            fn deserialize<D: Deserializer<'b>>(d: D) -> Result<Self, D::Error> {
                struct FieldVisitor;
                impl<'c> Visitor<'c> for FieldVisitor {
                    type Value = Field;
                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str("element-6066-11e4-a52e-4f735466cecf")
                    }

                    fn visit_str<E: DeError>(self, value: &str) -> Result<Field, E>
                    {
                        match value {
                            "element-6066-11e4-a52e-4f735466cecf" => Ok(Field::Reference),
                            // Ignore all other fields
                            _ => Ok(Field::Ignored),
                        }
                    }
                }

                d.deserialize_identifier(FieldVisitor)
            }
        }

        struct ElementRefVisitor;
        impl<'d> Visitor<'d> for ElementRefVisitor {
            type Value = ElementRef;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct ElementRef")
            }

            fn visit_map<V>(self, mut visitor: V) -> Result<ElementRef, V::Error>
                where V: MapAccess<'d>
            {
                let mut reference = None;
                while let Some(key) = visitor.next_key()? {
                    match key {
                        Field::Reference => {
                            if reference.is_some() {
                                return Err(DeError::duplicate_field("element-6066-11e4-a52e-4f735466cecf"));
                            }
                            reference = Some(visitor.next_value()?);
                        }
                        Field::Ignored => (),
                    }
                }
                match reference {
                    Some(r) => Ok(ElementRef { reference: r }),
                    None => return Err(DeError::missing_field("element-6066-11e4-a52e-4f735466cecf")),
                }
            }
        }

        const FIELDS: &'static [&'static str] = &["element-6066-11e4-a52e-4f735466cecf"];
        d.deserialize_struct("ElementRef", FIELDS, ElementRefVisitor)
    }
}

/// Element operations are use a named id to select the Element
/// and other attributes to specify the operation.
#[derive(Serialize, Debug)]
pub struct ElementOp {
    /// The element identifier
    pub id: String,
    /// The name of the attribute/property
    pub name: Option<String>,
}

/// A `switchToFrame` request
#[derive(Serialize, Debug)]
pub struct FrameSwitch {
    focus: bool,
    element: Option<String>,
}

impl FrameSwitch {
    /// Switch to the top level frame
    pub fn top(focus: bool) -> Self {
        FrameSwitch {
            focus: focus,
            element: None,
        }
    }

    /// Switch to the frame given by passed element
    pub fn from_element(focus: bool, element: Option<ElementRef>) -> Self {
        FrameSwitch {
            focus: focus,
            element: element.map(|elem| elem.reference.to_owned()),
        }
    }
}

#[derive(Serialize, Debug)]
pub struct AddonInstall<'a> {
    pub path: &'a Path,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Cookie {
    pub name: String,
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secure: Option<bool>,
}

#[derive(Serialize, Debug, PartialEq)]
pub struct AddCookie<'a> {
    pub cookie: &'a Cookie,
}

