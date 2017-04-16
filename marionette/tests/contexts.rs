extern crate marionette;
use marionette::*;
extern crate env_logger;

#[test]
fn windows() {   
    let _ = env_logger::init();
    let mut conn = MarionetteConnection::connect(2828).unwrap();
    let windows = conn.get_window_handles().unwrap();

    for window in windows {
        conn.switch_to_window(&window).unwrap();
        assert_eq!(window, conn.get_window_handle().unwrap());
    }
}

