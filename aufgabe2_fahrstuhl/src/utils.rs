use std::thread;

pub(crate) fn delay(ms: u64) {
    thread::sleep(std::time::Duration::from_millis(ms));
}