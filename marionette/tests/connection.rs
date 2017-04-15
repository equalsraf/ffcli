
extern crate marionette;
use marionette::*;
extern crate env_logger;

#[test]
fn connect() {   
    let _ = env_logger::init();
    let mut conn = MarionetteConnection::connect(2828).unwrap();
    conn.get_title().unwrap();
}

#[test]
fn connect2() {   
    let _ = env_logger::init();
    let mut conn = MarionetteConnection::connect(2828).unwrap();
    conn.get_title().unwrap();
}
