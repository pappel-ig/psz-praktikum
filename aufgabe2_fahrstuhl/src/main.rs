use crate::controller::ElevatorController;
use crate::elevator::Elevator;
use crate::logger::SimpleLogger;
use crate::mqtt::Receive;
use crate::person::Person;
use log::LevelFilter;
use mqtt::MqttConnector;
use tokio::sync::{broadcast, mpsc};
use std::collections::HashSet;
use crate::utils::SPEED_FACTOR;
use std::sync::atomic::Ordering;
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
    log::set_logger(&LOGGER).map(|()| log::set_max_level(LevelFilter::Error)).unwrap();

    // controller -> elevators
    let (controller_to_elevators_tx, _) = broadcast::channel(100000);
    // elevator -> controller
    let (elevator_to_controller_tx, elevator_to_controller_rx) = mpsc::channel(100000);
    // persons -> controller
    let (person_to_controller_tx, person_to_controller_rx) = mpsc::channel(100000);
    // controller -> persons
    let (controller_to_persons_tx, _) = broadcast::channel(100000);
    // components -> mqtt
    let (to_mqtt_tx, to_mqtt_rx) = mpsc::channel(100000);
    // mqtt -> persons
    let (mqtt_to_person_tx, mut mqtt_to_person_rx) = mpsc::channel(100000);

    let controller = ElevatorController::new(
        elevator_to_controller_rx,
        controller_to_elevators_tx.clone(),
        person_to_controller_rx,
        controller_to_persons_tx.clone(),
        to_mqtt_tx.clone(),
        vec!["Dorisch".to_string(), "Ionisch".to_string(), "Korinthisch".to_string()]
    );
    let controller_handle = controller.init();

    let mqtt = MqttConnector::new(to_mqtt_rx, mqtt_to_person_tx).await;

    let mut threads = vec![
        mqtt.mqtt_subscriber(),
        mqtt.mqtt_publisher(),
        Elevator::new("Dorisch", controller_to_elevators_tx.subscribe(), elevator_to_controller_tx.clone(), to_mqtt_tx.clone()).init(),
        Elevator::new("Ionisch", controller_to_elevators_tx.subscribe(), elevator_to_controller_tx.clone(), to_mqtt_tx.clone()).init(),
        Elevator::new("Korinthisch", controller_to_elevators_tx.subscribe(), elevator_to_controller_tx.clone(), to_mqtt_tx.clone()).init(),
        controller_handle,
    ];

    for i in 0..5 {
        let person_id = format!("Student_{}", i);
        threads.push(Person::new(
            &person_id,
            controller_to_persons_tx.subscribe(),
            person_to_controller_tx.clone(),
            to_mqtt_tx.clone()
        ).init());
    }

    let mut created_persons: HashSet<String> = HashSet::new();
    loop {

        if let Some(msg) = mqtt_to_person_rx.recv().await {
            match msg {
                Receive::Person { id, curr: current_floor, dest: destination_floor } => {
                    if created_persons.insert(id.clone()) {
                        println!("Person erstellt via MQTT: id={}, curr={:?}, dest={:?}", id, current_floor, destination_floor);
                        let person = Person::with(
                            &id,
                            controller_to_persons_tx.subscribe(),
                            person_to_controller_tx.clone(),
                            to_mqtt_tx.clone(),
                            current_floor,
                            destination_floor
                        );
                        threads.push(person.init());
                    }
                }
                Receive::Speed { speed } => {
                    SPEED_FACTOR.store(speed, Ordering::Relaxed);
                    println!("Simulationsgeschwindigkeit geändert: {}%", speed);
                }
            }
        }
    }
}


