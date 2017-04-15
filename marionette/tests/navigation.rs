
extern crate marionette;
use marionette::*;
extern crate env_logger;

#[test]
fn navigation() {
    let _ = env_logger::init();
    let mut conn = MarionetteConnection::connect(2828).unwrap();
    conn.get("https://www.mozilla.org").unwrap();
    let url0 = conn.get_url().unwrap();
    conn.get("https://github.com/equalsraf/ffcli").unwrap();
    let url1 = conn.get_url().unwrap();

    conn.go_back().unwrap();
    assert_eq!(url0, conn.get_url().unwrap());
    conn.go_forward().unwrap();
    assert_eq!(url1, conn.get_url().unwrap());

    conn.go_forward().unwrap();
    assert_eq!(url1, conn.get_url().unwrap());
}
