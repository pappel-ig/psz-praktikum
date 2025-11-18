use std::thread;
use log::{info, LevelFilter};
use tokio::sync::{broadcast, mpsc};
use tokio::sync::broadcast::Receiver;
use tokio::sync::mpsc::Sender;
use crate::controller::{ElevatorController, Floor};
use crate::elevator::Elevator;
use crate::logger::SimpleLogger;
use crate::msg::{ControllerToElevatorsMsg, ElevatorToControllerMsg, PersonToControllerMsg};
use crate::msg::PersonToControllerMsg::{PersonChoosingFloor, PersonEnteredElevator, PersonEnteringElevator, PersonRequestElevator};

mod controller;
mod elevator;
mod msg;
mod person;
mod logger;

static LOGGER: SimpleLogger = SimpleLogger;

#[tokio::main]
async fn main() {
    log::set_logger(&LOGGER).map(|()| log::set_max_level(LevelFilter::Info)).unwrap();

    // controller -> elevators
    let (from_controller_to_elevators_tx, _) = broadcast::channel(100);
    // elevator -> controller
    let (elevator_to_controller_tx, elevator_to_controller_rx) = mpsc::channel(100);
    // persons -> controller
    let (person_to_controller_tx, person_to_controller_rx) = mpsc::channel(100);

    let controller = ElevatorController::new(
        elevator_to_controller_rx,
        from_controller_to_elevators_tx.clone(),
        person_to_controller_rx,
        vec!["Dorisch".to_string(), "Ionisch".to_string(), "Korinthisch".to_string()]
    );

    let threads = vec![
        Elevator::new("Dorisch", from_controller_to_elevators_tx.subscribe(), elevator_to_controller_tx.clone()).init(),
        Elevator::new("Ionisch", from_controller_to_elevators_tx.subscribe(), elevator_to_controller_tx.clone()).init(),
        Elevator::new("Korinthisch", from_controller_to_elevators_tx.subscribe(), elevator_to_controller_tx.clone()).init(),
        controller.init()
    ];

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let _ = person_to_controller_tx.send(PersonRequestElevator(Floor::Second)).await;
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let _ = person_to_controller_tx.send(PersonEnteringElevator("1".to_string(), "Dorisch".to_string())).await;
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let _ = person_to_controller_tx.send(PersonEnteredElevator("1".to_string(), "Dorisch".to_string())).await;
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let _ = person_to_controller_tx.send(PersonChoosingFloor("1".to_string(), "Dorisch".to_string(), Floor::First)).await;

    for _handle in threads {
        _handle.await.unwrap();
    }
}

fn create_elevator(name: &str, from_controller_to_elevators_rx: Receiver<ControllerToElevatorsMsg>, elevator_to_controller_tx: Sender<ElevatorToControllerMsg>) -> Elevator {
    Elevator::new(
        name,
        from_controller_to_elevators_rx,
        elevator_to_controller_tx)
}


