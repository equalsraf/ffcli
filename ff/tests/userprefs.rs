extern crate ff;

extern crate marionette;
use marionette::{MarionetteConnection, JsonValue};
use std::thread;
use std::time::Duration;

#[test]
fn user_js_file() {
    let browser = ff::Browser::start(Some(65333),
                                         None,
                                         None,
                                         Some("tests/data/test-user.js"),
                                         None,
                                         None).unwrap();

    let port = browser.runner.port().unwrap();
    ff::check_connection(port).unwrap();

    thread::sleep(Duration::new(5, 0));
    let mut conn = MarionetteConnection::connect(port).unwrap();
    let res = conn.get_pref("ff.testpref.canary").unwrap();
    assert_eq!(res, JsonValue::String("the canary is dead".to_string()));
    conn.quit().unwrap();
}
