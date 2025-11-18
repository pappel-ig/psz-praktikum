use std::sync::{Arc, Mutex};
use crossbeam_channel::{unbounded, Receiver, Sender};
use crate::elevator::Elevator;
use crate::msg::{ControllerToElevatorsMsg, ElevatorToControllerMsg};

mod controller;
mod elevator;
mod msg;
mod person;

fn main() {
    // controller -> elevators
    let (from_controller_to_elevators_tx, from_controller_to_elevators_rx) = unbounded();
    // elevator -> controller
    let (elevator_to_controller_tx, elevator_to_controller_rx) = unbounded();

    let threads = vec![
        create_elevator("Dorisch", from_controller_to_elevators_rx.clone(), elevator_to_controller_tx.clone()).init(),
        create_elevator("Ionisch", from_controller_to_elevators_rx.clone(), elevator_to_controller_tx.clone()).init(),
        create_elevator("Korinthisch", from_controller_to_elevators_rx.clone(), elevator_to_controller_tx.clone()).init(),
    ];

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


