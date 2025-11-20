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

static LOGGER: SimpleLogger = SimpleLogger;

#[tokio::main]
async fn main() {
    log::set_logger(&LOGGER).map(|()| log::set_max_level(LevelFilter::Info)).unwrap();

    // controller -> elevators
    let (controller_to_elevators_tx, _) = broadcast::channel(100);
    // elevator -> controller
    let (elevator_to_controller_tx, elevator_to_controller_rx) = mpsc::channel(100);
    // persons -> controller
    let (person_to_controller_tx, person_to_controller_rx) = mpsc::channel(100);
    // controller -> persons
    let (controller_to_persons_tx, _) = broadcast::channel(100);

    let mut mqttoptions = MqttOptions::new("elevator-sim", "localhost", 1883);
    mqttoptions.set_keep_alive(Duration::from_secs(5));

    let (mqtt_client, mut eventloop) = AsyncClient::new(mqttoptions, 10);
    mqtt_client.subscribe("elevator/new_person", QoS::AtLeastOnce).await.unwrap();

    let controller = ElevatorController::new(
        elevator_to_controller_rx,
        controller_to_elevators_tx.clone(),
        person_to_controller_rx,
        controller_to_persons_tx.clone(),
        vec!["Dorisch".to_string(), "Ionisch".to_string(), "Korinthisch".to_string()],
        mqtt_client.clone()
    );
    let controller_handle = controller.init();

    let mut threads = vec![
        Elevator::new("Dorisch", controller_to_elevators_tx.subscribe(), elevator_to_controller_tx.clone()).init(),
        Elevator::new("Ionisch", controller_to_elevators_tx.subscribe(), elevator_to_controller_tx.clone()).init(),
        Elevator::new("Korinthisch", controller_to_elevators_tx.subscribe(), elevator_to_controller_tx.clone()).init(),
        controller_handle,
    ];

    delay(500);
    //Person testen auskommentiert 
    //threads.push(Person::with("Alice", controller_to_persons_tx.subscribe(), person_to_controller_tx.clone(), First, Ground).init());

    // MQTT Task - Person erstellen bei Nachricht
    let person_tx_clone = person_to_controller_tx.clone();
    let persons_rx_clone = controller_to_persons_tx.clone();

    tokio::spawn(async move {
        loop {
            match eventloop.poll().await {
                Ok(notification) => {
                    if let rumqttc::Event::Incoming(rumqttc::Packet::Publish(p)) = notification {
                        let person_id = format!("Person_{}", rand::random::<u16>());
                        info!("Creating new person via MQTT: {}", person_id);

                        let person = Person::new(
                            &person_id,
                            persons_rx_clone.subscribe(),
                            person_tx_clone.clone()
                        );
                        person.init();
                    }
                }
                Err(e) => {
                    error!("MQTT Error: {:?}", e);
                }
            }
        }
    });

    delay(500);



    for _handle in threads {
        _handle.await.unwrap();
    }
}


