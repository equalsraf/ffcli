
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
fn get_logs_drains_the_logs() {
    let _ = env_logger::init();

    let mut conn = MarionetteConnection::connect(2828).unwrap();
    conn.log(LogMsg::new("Hi", "RUST")).unwrap();
    let logs = conn.get_logs().unwrap();
    assert_eq!(logs[0].msg(), "Hi");
    assert_eq!(logs[0].level(), "RUST");

    let logs = conn.get_logs().unwrap();
    assert!(logs.is_empty());
}

#[test]
fn logs_do_not_persist_accross_connections() {
    let _ = env_logger::init();

    {
    let mut conn = MarionetteConnection::connect(2828).unwrap();
    conn.log(LogMsg::new("Hi", "RUST")).unwrap();
    }

    let mut conn = MarionetteConnection::connect(2828).unwrap();
    let logs = conn.get_logs().unwrap();
    assert!(logs.is_empty());
}
