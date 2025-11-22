use std::time::Duration;
use rand::{rng, Rng};
use tokio::sync::broadcast::Sender;
use tokio::task::JoinHandle;
use crate::msg::ControllerToElevatorsMsg;
use crate::msg::ControllerToElevatorsMsg::CloseDoors;

static REALISTIC: bool = false;

pub(crate) async fn delay(ms: u64) {
    if REALISTIC {
        tokio::time::sleep(Duration::from_millis(ms)).await;
    } else {
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
}

pub(crate) async fn random_delay_ms(from: u64, to: u64) {
    if REALISTIC {
        let delay = rng().random_range(from..=to);
        tokio::time::sleep(Duration::from_millis(delay)).await;
    } else {
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
}

pub(crate) fn get_closing_task(to_elevators: Sender<ControllerToElevatorsMsg>, elevator: String) -> Option<JoinHandle<()>> {
    Some(tokio::spawn(async move {
        if REALISTIC {
            tokio::time::sleep(Duration::from_millis(5000)).await;
        } else {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        let _ = to_elevators.send(CloseDoors(elevator));
    }))
}