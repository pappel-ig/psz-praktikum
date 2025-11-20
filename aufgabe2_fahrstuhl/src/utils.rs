use std::thread;
use std::time::Duration;
use tokio::sync::broadcast::Sender;
use tokio::task::JoinHandle;
use crate::msg::ControllerToElevatorsMsg;
use crate::msg::ControllerToElevatorsMsg::CloseDoors;

pub(crate) fn delay(ms: u64) {
    thread::sleep(Duration::from_millis(ms));
}

pub(crate) fn get_closing_task(to_elevators: Sender<ControllerToElevatorsMsg>, elevator: String) -> Option<JoinHandle<()>> {
    Some(tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(100)).await;
        let _ = to_elevators.send(CloseDoors(elevator));
    }))
}