use std::time::Duration;
use rand::{rng, Rng};
use tokio::sync::broadcast::Sender;
use tokio::task::JoinHandle;
use crate::msg::ControllerToElevatorsMsg;
use crate::msg::ControllerToElevatorsMsg::CloseDoors;

static REALISTIC: bool = true;
static SPEED_FACTOR: f64 = 5.0;

pub(crate) async fn delay(ms: u64) {
    if REALISTIC {
        tokio::time::sleep(Duration::from_millis(ms)).await;
    } else {
        let adjusted_ms = (ms as f64 * SPEED_FACTOR) as u64;
        tokio::time::sleep(Duration::from_millis(adjusted_ms)).await;
    }
}

pub(crate) async fn random_delay_ms(from: u64, to: u64) {
    if REALISTIC {
        let delay = rng().random_range(from..=to);
        let adjusted_delay = (delay as f64 * SPEED_FACTOR) as u64;
        tokio::time::sleep(Duration::from_millis(adjusted_delay)).await;
    } else {
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
}

pub(crate) fn get_closing_task(to_elevators: Sender<ControllerToElevatorsMsg>, elevator: String) -> Option<JoinHandle<()>> {
    Some(tokio::spawn(async move {
        if REALISTIC {
            let adjusted_ms = (5000.0 * SPEED_FACTOR) as u64;
            tokio::time::sleep(Duration::from_millis(adjusted_ms)).await;
        } else {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        let _ = to_elevators.send(CloseDoors(elevator));
    }))
}