//! Structures for some of the messages used in the Marionette protocol, these can
//! be used with the traits in serde to convert into the corresponding json.
//!
#![allow(non_snake_case)]

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
#[derive(Deserialize, Debug)]
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
