
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use rand::{rng, Rng};
use tokio::sync::broadcast::Sender;
use tokio::task::JoinHandle;
use crate::msg::ControllerToElevatorsMsg;
use crate::msg::ControllerToElevatorsMsg::CloseDoors;

pub static SPEED_FACTOR: AtomicU64 = AtomicU64::new(100);

pub(crate) async fn delay(ms: u64) {
    let factor = SPEED_FACTOR.load(Ordering::Relaxed);
    let adjusted = (ms * factor) / 100;
    tokio::time::sleep(Duration::from_millis(adjusted)).await;
}

pub(crate) async fn random_delay_ms(from: u64, to: u64) {
    let factor = SPEED_FACTOR.load(Ordering::Relaxed);
    let base_delay = rng().random_range(from..=to);
    let adjusted = (base_delay * factor) / 100;
    tokio::time::sleep(Duration::from_millis(adjusted)).await;
}

pub(crate) fn get_closing_task(to_elevators: Sender<ControllerToElevatorsMsg>, elevator: String) -> Option<JoinHandle<()>> {
    Some(tokio::spawn(async move {
        let factor = SPEED_FACTOR.load(Ordering::Relaxed);
        let adjusted = (5000 * factor) / 100;
        tokio::time::sleep(Duration::from_millis(adjusted)).await;
        let _ = to_elevators.send(CloseDoors(elevator));
    }))
}
