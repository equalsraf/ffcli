
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
fn timeouts_are_set() {
    let _ = env_logger::init();
    let mut conn = MarionetteConnection::connect(2828).unwrap();
    assert!(conn.timeouts().is_some());

    let t = Timeouts {
        script: 10001,
        pageLoad: 10002,
        implicit: 10003,
    };
    conn.set_timeouts(t).unwrap();
    assert_eq!(conn.timeouts(), Some(&t));
}
