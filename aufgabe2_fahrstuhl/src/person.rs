use rand::prelude::SliceRandom;
use rand::rng;
use tokio::sync::broadcast::Receiver;
use tokio::sync::mpsc::Sender;
use crate::controller::Floor;
use crate::msg::{ControllerToPersonsMsg, PersonToControllerMsg};
use crate::msg::ControllerToPersonsMsg::{ElevatorDeparted, ElevatorHalt, TooManyPassengers};

#[derive(Clone, PartialEq)]
pub enum PersonStatus {
    Idle,
    Entering,
    Choosing,
    InElevator,
    Leaving
}

pub struct Person {
    pub id: String,
    from_controller_to_persons: Receiver<ControllerToPersonsMsg>,
    from_person_to_controller: Sender<PersonToControllerMsg>,
    state: PersonState
}

struct PersonState {
    status: PersonStatus,
    current_floor: Floor,
    destination_floor: Floor,
    elevator: Option<String>
}

impl Person {
    pub fn new(id: &str,
               from_controller_to_persons: Receiver<ControllerToPersonsMsg>,
               from_person_to_controller: Sender<PersonToControllerMsg>) -> Self {
        let (current_floor, destination_floor) = Self::pick_two_distinct_floors();
        Person {
            id: id.to_string(),
            from_controller_to_persons,
            from_person_to_controller,
            state: PersonState {
                status: PersonStatus::Idle,
                current_floor,
                destination_floor,
                elevator: None
            }
        }
    }

    pub fn init(mut self) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            loop {
                if let Ok(msg) = self.from_controller_to_persons.recv().await {
                    match msg {
                        ElevatorHalt(elevator, floor) => {
                            if self.state.current_floor.eq(&floor) { self.handle_elevator_halt(elevator.clone(), floor.clone()).await }
                        }
                        ElevatorDeparted(elevator, floor) => {
                            if self.state.current_floor.eq(&floor) { self.handle_elevator_departed(elevator.clone(), floor.clone()).await }
                        }
                        TooManyPassengers(elevator) => {
                            if matches!(&self.state.elevator, Some(x) if x.eq(&elevator)) { self.handle_too_many_passengers(elevator.clone()).await }
                        }
                    }
                }
            }
        })
    }

    async fn handle_elevator_halt(&mut self, elevator: String, floor: Floor) {

    }

    async fn handle_elevator_departed(&mut self, elevator: String, floor: Floor) {

    }

    async fn handle_too_many_passengers(&mut self, elevator: String) {

    }

    fn pick_two_distinct_floors() -> (Floor, Floor) {
        let mut rng = rng();
        let mut floors = vec![Floor::Ground, Floor::First, Floor::Second];
        floors.shuffle(&mut rng);
        (floors[0], floors[1])
    }
}

