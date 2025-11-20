use crate::controller::Floor::{First, Ground, Second, Third};
use crate::controller::ElevatorController;
use crate::elevator::Elevator;
use crate::logger::SimpleLogger;
use crate::person::Person;
use crate::utils::delay;
use log::{LevelFilter, info, error};
use tokio::sync::{broadcast, mpsc};
use rumqttc::{MqttOptions, AsyncClient, QoS, EventLoop};
use std::time::Duration;

mod controller;
mod elevator;
mod msg;
mod person;
mod logger;
mod utils;
mod mqtt;

static LOGGER: SimpleLogger = SimpleLogger;

#[tokio::main]
async fn main() {
    log::set_logger(&LOGGER).map(|()| log::set_max_level(LevelFilter::Off)).unwrap();

    // controller -> elevators
    let (controller_to_elevators_tx, _) = broadcast::channel(1000);
    // elevator -> controller
    let (elevator_to_controller_tx, elevator_to_controller_rx) = mpsc::channel(1000);
    // persons -> controller
    let (person_to_controller_tx, person_to_controller_rx) = mpsc::channel(1000);
    // controller -> persons
    let (controller_to_persons_tx, _) = broadcast::channel(1000);



    let controller = ElevatorController::new(
        elevator_to_controller_rx,
        controller_to_elevators_tx.clone(),
        person_to_controller_rx,
        controller_to_persons_tx.clone(),
        //vec!["Dorisch".to_string()],
        //vec!["Dorisch".to_string(), "Ionisch".to_string()],
        vec!["Dorisch".to_string(), "Ionisch".to_string(), "Korinthisch".to_string()]
    );
    let controller_handle = controller.init();

    let mut threads = vec![
        Elevator::new("Dorisch", controller_to_elevators_tx.subscribe(), elevator_to_controller_tx.clone()).init(),
        Elevator::new("Ionisch", controller_to_elevators_tx.subscribe(), elevator_to_controller_tx.clone()).init(),
        Elevator::new("Korinthisch", controller_to_elevators_tx.subscribe(), elevator_to_controller_tx.clone()).init(),
        controller_handle,
    ];

    delay(500);

    for i in 0..1000 {
        let person_id = format!("Person_{}", i);
        threads.push(Person::new(
            &person_id,
            controller_to_persons_tx.subscribe(),
            person_to_controller_tx.clone()
        ).init());
    }

    for _handle in threads {
        _handle.await.unwrap();
    }
}


