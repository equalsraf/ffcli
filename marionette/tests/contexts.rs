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

#[test]
fn context() {
    let _ = env_logger::init();
    let mut conn = MarionetteConnection::connect(2828).unwrap();

    let ctx = conn.get_context().unwrap();

    conn.set_context(Context::Chrome).unwrap();
    assert_eq!(Context::Chrome, conn.get_context().unwrap());
    conn.set_context(Context::Content).unwrap();
    assert_eq!(Context::Content, conn.get_context().unwrap());

    conn.set_context(ctx).unwrap();
}
