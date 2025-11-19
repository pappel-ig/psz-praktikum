use crate::controller::Floor::{Ground, Second};
use crate::controller::ElevatorController;
use crate::elevator::Elevator;
use crate::logger::SimpleLogger;
use crate::person::Person;
use crate::utils::delay;
use log::LevelFilter;
use tokio::sync::{broadcast, mpsc};

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
    let (from_controller_to_elevators_tx, _) = broadcast::channel(100);
    // elevator -> controller
    let (elevator_to_controller_tx, elevator_to_controller_rx) = mpsc::channel(100);
    // persons -> controller
    let (person_to_controller_tx, person_to_controller_rx) = mpsc::channel(100);
    // controller -> persons
    let (from_controller_to_persons_tx, _) = broadcast::channel(100);

    let controller = ElevatorController::new(
        elevator_to_controller_rx,
        from_controller_to_elevators_tx.clone(),
        person_to_controller_rx,
        from_controller_to_persons_tx.clone(),
        vec!["Dorisch".to_string(), "Ionisch".to_string(), "Korinthisch".to_string()]
    );
    let controller_handle = controller.init();



    let mut threads = vec![
        Elevator::new("Dorisch", from_controller_to_elevators_tx.subscribe(), elevator_to_controller_tx.clone()).init(),
        Elevator::new("Ionisch", from_controller_to_elevators_tx.subscribe(), elevator_to_controller_tx.clone()).init(),
        Elevator::new("Korinthisch", from_controller_to_elevators_tx.subscribe(), elevator_to_controller_tx.clone()).init(),
        controller_handle,
    ];

    delay(500);

    //threads.push(Person::with("Alice", from_controller_to_persons_tx.subscribe(), person_to_controller_tx.clone(), Ground, Second).init());
    //threads.push(Person::with("Bob", from_controller_to_persons_tx.subscribe(), person_to_controller_tx.clone(), First, Ground).init());
    //threads.push(Person::with("Mallory", from_controller_to_persons_tx.subscribe(), person_to_controller_tx.clone(), Second, Ground).init());

    threads.push(Person::with("Alice", from_controller_to_persons_tx.subscribe(), person_to_controller_tx.clone(), Ground, Second).init());
    threads.push(Person::with("Bob", from_controller_to_persons_tx.subscribe(), person_to_controller_tx.clone(), Ground, Second).init());


    delay(500);



    for _handle in threads {
        _handle.await.unwrap();
    }
}


