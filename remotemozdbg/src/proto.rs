
use std::io::{BufRead, Write, Error, ErrorKind, BufReader, self};
use std::str;
use std::convert::From;
use std::str::FromStr;

use serde_json::{Value, from_str, from_value, to_value, Map, to_string};
use serde_json::Error as JsonError;
use serde::Serialize;
use serde::de::DeserializeOwned;

#[derive(Debug, PartialEq)]
pub enum Packet {
    Json(Value),
    BulkData(String, String, Vec<u8>),
}

pub trait IntoPacket {
    fn into_packet(self) -> Result<Packet, JsonError>;
}

// TODO impl this for bulk data
impl IntoPacket for Packet {
    fn into_packet(self) -> Result<Packet, JsonError> {
        Ok(self)
    }
}

impl<'a, T> IntoPacket for &'a T where T: Serialize {
    fn into_packet(self) -> Result<Packet, JsonError> {
        to_value(&self)
            .map(Packet::Json)
    }
}

pub fn readpacket<R: BufRead>(r: &mut R) -> io::Result<Packet> {
    let mut prefix = Vec::new();
    let bytes = r.read_until(b':', &mut prefix)?;
    if bytes == 0 {
        return Err(Error::new(ErrorKind::InvalidData, "Invalid json packet"));
    }

    // FIXME ignore trailling :
    if prefix.starts_with(b"bulk") {
        let mut iter = prefix[..prefix.len()-1].split(|b| *b == b' ');
        let bulk_hdr = iter.next();
        if bulk_hdr != Some("bulk".as_ref()) {
            return Err(Error::new(ErrorKind::InvalidData, format!("Invalid bulk packet preamble {:?}", bulk_hdr)));
        }

        let actor = match iter.next()
            .map(|v| v.to_vec())
            .map(String::from_utf8) {
                Some(Err(_)) => return Err(Error::new(ErrorKind::InvalidData, "Bulk packet has no invalid actor")),
                Some(Ok(actor)) => actor,
                None => return Err(Error::new(ErrorKind::InvalidData, "Bulk packet has no actor")),
        };

        let pkttype = match iter.next()
            .map(|v| v.to_vec())
            .map(String::from_utf8) {
                Some(Err(_)) => return Err(Error::new(ErrorKind::InvalidData, "Bulk packet has no invalid type")),
                Some(Ok(t)) => t,
                None => return Err(Error::new(ErrorKind::InvalidData, "Bulk packet has no type")),
        };

        let len = match iter.next()
            .map(str::from_utf8) {
                Some(Err(_)) => return Err(Error::new(ErrorKind::InvalidData, "Bulk packet has no invalid utf8 in length")),
                Some(Ok(len_str)) => match usize::from_str_radix(len_str, 10) {
                    Ok(len) => len,
                    Err(_) => return Err(Error::new(ErrorKind::InvalidData, "Bulk packet has invalid length")),
                }
                None => return Err(Error::new(ErrorKind::InvalidData, "Bulk packet has no length")),
        };

        let mut buf = Vec::with_capacity(len);
        buf.resize(len, 0);
        r.read_exact(buf.as_mut_slice())?;
        Ok(Packet::BulkData(actor, pkttype, buf))
    } else {
        let len_str = str::from_utf8(&prefix[..bytes-1])
            .map_err(|_| Error::new(ErrorKind::InvalidData, "Invalid utf8 in packet length"))?;
        let len = usize::from_str_radix(len_str, 10)
            .map_err(|_| Error::new(ErrorKind::InvalidData, "Invalid packet length"))?;

        let mut buf = Vec::with_capacity(len);
        buf.resize(len, 0);
        r.read_exact(buf.as_mut_slice())?;

        let s = match String::from_utf8(buf) {
            Ok(s) => s,
            Err(_) => return Err(Error::new(ErrorKind::InvalidData, "Invalid utf8 in Json payload")),
        };
        to_value(s)
            .map(Packet::Json)
            .map_err(|_| Error::new(ErrorKind::InvalidData, "Invalid json packet"))
    }
}

/// Prepend string with length: and push it down the wire
pub fn sendpacket<W: Write, M: IntoPacket>(w: &mut W, msg: M) -> io::Result<()> {
    let p = msg.into_packet()?;
    debug!("-> {:?}", &p);
    match p {
        Packet::Json(ref v) => {
            let data = v.to_string();
            w.write_all(format!("{}:", data.len()).as_bytes())?;
            w.write_all(data.as_bytes())?;
        }
        Packet::BulkData(ref actor, ref typ, ref data) => {
            w.write_all(format!("bulk {} {} {}:", actor, typ, data.len()).as_bytes())?;
            w.write_all(data)?;
        }
    }
    Ok(())
}

#[derive(Deserialize)]
pub struct ServerPacket {
    pub from: String,
    pub error: Option<String>,
    pub message: Option<String>,
}

#[derive(Serialize)]
pub struct ListTabsRequest {
    to: &'static str,
    #[serde(rename = "type")]
    typ: &'static str,
}

impl ListTabsRequest {
    pub fn new() -> Self {
        ListTabsRequest {
            to: "root",
            typ: "listTabs",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bulk_packet() {
        let mut pkt = b"bulk actor type 3:123".to_vec();
        let mut reader = pkt.as_slice();
        
        let parsed_pkd = readpacket(&mut reader).unwrap();
        assert_eq!(parsed_pkd,
            Packet::BulkData("actor".to_string(), "type".to_string(), b"123".to_vec()));

        let mut out = Vec::new();
        sendpacket(&mut out, &parsed_pkd).unwrap();
        assert_eq!(pkt, out);
    }
    #[test]
    fn json_packet() {
        let mut pkt = b"2:{}".to_vec();
        let mut reader = pkt.as_slice();

        let parsed_pkd = readpacket(&mut reader).unwrap();
        assert_eq!(parsed_pkd,
            Packet::Json(Value::Object(Map::new())));

        let mut out = Vec::new();
        sendpacket(&mut out, &parsed_pkd).unwrap();
        assert_eq!(pkt, out);
    }
}
