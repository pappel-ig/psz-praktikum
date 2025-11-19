use std::fmt::{Debug, Display, Formatter};
use log::{debug, error, info};
use rand::prelude::SliceRandom;
use rand::rng;
use tokio::sync::broadcast::Receiver;
use tokio::sync::mpsc::Sender;
use ControllerToPersonsMsg::{UpdateBoardingStatus};
use crate::controller::{BoardingStatus, Floor};
use crate::msg::{ControllerToPersonsMsg, PersonToControllerMsg};
use crate::msg::ControllerToPersonsMsg::{ElevatorDeparted, ElevatorHalt, TooManyPassengers};
use crate::msg::PersonToControllerMsg::{PersonLeavingElevator, PersonLeftElevator, PersonRequestElevator};
use crate::person::PersonStatus::Leaving;
use crate::utils::delay;

#[derive(Clone, PartialEq, Debug)]
pub enum PersonStatus {
    Idle,
    Entering,
    Choosing,
    InElevator,
    Leaving
}

pub struct Person {
    pub id: String,
    from_controller: Receiver<ControllerToPersonsMsg>,
    to_controller: Sender<PersonToControllerMsg>,
    state: PersonState
}

#[derive(Debug)]
struct PersonState {
    status: PersonStatus,
    current_floor: Floor,
    destination_floor: Floor,
    elevator: Option<String>,
    finished: bool,
}

impl Display for PersonState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{ status: {:?}, floor: {:?}, dest: {:?}, elevator: {:?}, requested: {:?} }}", self.status, self.current_floor, self.destination_floor, self.elevator, self.finished)
    }
}

impl Display for Person {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{ id: {:?}, {:?} }}", self.id, self.state)
    }
}

impl Debug for Person {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self, f)
    }
}

impl Person {
    pub fn new(id: &str,
               from_controller_to_persons: Receiver<ControllerToPersonsMsg>,
               from_person_to_controller: Sender<PersonToControllerMsg>) -> Self {
        let (current_floor, destination_floor) = Self::pick_two_distinct_floors();
        Person {
            id: id.to_string(),
            from_controller: from_controller_to_persons,
            to_controller: from_person_to_controller,
            state: PersonState {
                finished: false,
                status: PersonStatus::Idle,
                current_floor,
                destination_floor,
                elevator: None
            }
        }
    }

    pub fn with(id: &str,
               from_controller_to_persons: Receiver<ControllerToPersonsMsg>,
               from_person_to_controller: Sender<PersonToControllerMsg>,
                current_floor: Floor,
                destination_floor: Floor) -> Self {
        Person {
            id: id.to_string(),
            from_controller: from_controller_to_persons,
            to_controller: from_person_to_controller,
            state: PersonState {
                finished: false,
                status: PersonStatus::Idle,
                current_floor,
                destination_floor,
                elevator: None
            }
        }
    }

    pub fn init(mut self) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            self.request_elevator().await;
            loop {
                if let Ok(msg) = self.from_controller.recv().await {
                    match msg {
                        ElevatorHalt(elevator, floor) => {
                            self.handle_elevator_halt(elevator.clone(), floor.clone()).await
                        }
                        ElevatorDeparted(elevator, floor) => {
                            self.handle_elevator_departed(elevator.clone(), floor.clone()).await
                        }
                        TooManyPassengers(person, elevator) => {
                             self.handle_too_many_passengers(person, elevator.clone()).await
                        }
                        UpdateBoardingStatus(person, elevator, boarding_status) => {
                            self.handle_update_boarding_status(person, elevator, boarding_status).await
                        }
                    }
                }
            }
        })
    }

    pub async fn request_elevator(&mut self) {
        let _ = self.to_controller.send(PersonRequestElevator(self.state.current_floor)).await;
        debug!("RequestElevator(person={}, floor={}, dest={})", self.id, self.state.current_floor, self.state.destination_floor);
    }

    async fn handle_elevator_halt(&mut self, elevator: String, floor: Floor) {
        if self.state.current_floor.eq(&floor) && self.state.status.eq(&PersonStatus::Idle) && !self.state.finished {
            self.state.status = PersonStatus::Entering;
            let _ = self.to_controller.send(PersonToControllerMsg::PersonEnteringElevator(self.id.clone(), elevator.clone())).await;
            delay(1);
            let _ = self.to_controller.send(PersonToControllerMsg::PersonEnteredElevator(self.id.clone(), elevator.clone())).await;
            self.state.status = PersonStatus::Choosing;
            delay(1);
            let _ = self.to_controller.send(PersonToControllerMsg::PersonChoosingFloor(self.id.clone(), elevator.clone(), self.state.destination_floor)).await;
            debug!("PersonEnteredElevator(person={}, elevator={}, dest={})", self.id.clone(), elevator, self.state.destination_floor);
        }
        if self.state.destination_floor.eq(&floor) && self.state.status.eq(&PersonStatus::InElevator) {
            debug!("PersonLeavingElevator(person={}, elevator={})", self.id.clone(), elevator);
            self.leave_elevator(self.id.clone(), elevator).await;
            self.state.finished = true;
        }
    }

    async fn handle_elevator_departed(&mut self, elevator: String, floor: Floor) {
        debug!("ElevatorDeparted({})", self.state);
    }

    async fn handle_too_many_passengers(&mut self, person: String, elevator: String) {
        debug!("TooManyPassengers(person={})", person);
        self.leave_elevator(person.clone(), elevator).await;
    }

    async fn handle_update_boarding_status(&mut self, person: String, elevator: String, boarding_status: BoardingStatus) {
        if self.id.eq(&person) {
            debug!("Boarding{}(person={}, elevator={})", boarding_status, self.id.clone(), elevator);
            match boarding_status {
                BoardingStatus::Accepted => {
                    self.state.status = PersonStatus::InElevator;
                    self.state.elevator = Some(elevator);
                }
                BoardingStatus::Rejected => {
                    self.leave_elevator(person, elevator).await;
                }
            }
        }
    }

    async fn leave_elevator(&mut self, person: String, elevator: String) {
        self.state.elevator = None;
        self.state.status = Leaving;
        let _ = self.to_controller.send(PersonLeavingElevator(person.clone(), elevator.clone())).await;
        delay(1);
        self.state.status = PersonStatus::Idle;
        let _ = self.to_controller.send(PersonLeftElevator(person, elevator.clone())).await;
        debug!("LeftElevator(person={}, elevator={})", self.id.clone(), elevator);
    }

    fn pick_two_distinct_floors() -> (Floor, Floor) {
        let mut rng = rng();
        let mut floors = vec![Floor::Ground, Floor::First, Floor::Second];
        floors.shuffle(&mut rng);
        (floors[0], floors[1])
    }
}

