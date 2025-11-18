use std::sync::{Arc, Mutex};
use std::thread;
use crossbeam_channel::{unbounded, Receiver, Sender};
use log::{info, LevelFilter};
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

fn main() {
    log::set_logger(&LOGGER).map(|()| log::set_max_level(LevelFilter::Info)).unwrap();

    // controller -> elevators
    let (from_controller_to_elevators_tx, from_controller_to_elevators_rx) = unbounded();
    // elevator -> controller
    let (elevator_to_controller_tx, elevator_to_controller_rx) = unbounded();
    // persons -> controller
    let (person_to_controller_tx, person_to_controller_rx) = unbounded();

    let controller = ElevatorController::new(
        Arc::new(Mutex::new(elevator_to_controller_rx)),
        Arc::new(Mutex::new(from_controller_to_elevators_tx)),
        Arc::new(Mutex::new(person_to_controller_rx)),
        vec!["Dorisch".to_string(), "Ionisch".to_string(), "Korinthisch".to_string()]
    );

    let threads = vec![
        create_elevator("Dorisch", from_controller_to_elevators_rx.clone(), elevator_to_controller_tx.clone()).init(),
        create_elevator("Ionisch", from_controller_to_elevators_rx.clone(), elevator_to_controller_tx.clone()).init(),
        create_elevator("Korinthisch", from_controller_to_elevators_rx.clone(), elevator_to_controller_tx.clone()).init(),
        controller.init()
    ];

    thread::sleep(std::time::Duration::from_millis(500));
    person_to_controller_tx.send(PersonRequestElevator(Floor::Second)).unwrap();
    thread::sleep(std::time::Duration::from_millis(500));
    person_to_controller_tx.send(PersonEnteringElevator("1".to_string(), "Dorisch".to_string())).unwrap();
    thread::sleep(std::time::Duration::from_millis(500));
    person_to_controller_tx.send(PersonEnteredElevator("1".to_string(), "Dorisch".to_string())).unwrap();
    thread::sleep(std::time::Duration::from_millis(500));
    person_to_controller_tx.send(PersonChoosingFloor("1".to_string(), "Dorisch".to_string(), Floor::First)).unwrap();

    for _handle in threads {
        _handle.join().unwrap();
    }
}

fn create_elevator(name: &str, from_controller_to_elevators_rx: Receiver<ControllerToElevatorsMsg>, elevator_to_controller_tx: Sender<ElevatorToControllerMsg>) -> Elevator {
    Elevator::new(
        name,
        Arc::new(Mutex::new(from_controller_to_elevators_rx)),
        Arc::new(Mutex::new(elevator_to_controller_tx)))
}


