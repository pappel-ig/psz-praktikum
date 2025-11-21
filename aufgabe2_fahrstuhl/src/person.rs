use std::fmt::{Debug, Display, Formatter};
use log::{debug, info, trace};
use serde::{Deserialize, Serialize};
use task::JoinHandle;
use tokio::sync::broadcast::Receiver;
use tokio::sync::mpsc::Sender;
use tokio::task;
use BoardingStatus::{Accepted, Rejected};
use ControllerToPersonsMsg::{UpdateBoardingStatus};
use PersonMsg::StatusUpdate;
use PersonStatus::{Done, Entering, Idle, InElevator};
use PersonToControllerMsg::{PersonChoosingFloor, PersonEnteredElevator, PersonEnteringElevator};
use crate::controller::{BoardingStatus, Floor};
use crate::mqtt::PersonMsg;
use crate::mqtt::PersonMsg::{Boarding, Request};
use crate::mqtt::Send::{PersonTopic};
use crate::msg::{ControllerToPersonsMsg, PersonToControllerMsg};
use crate::msg::ControllerToPersonsMsg::{ElevatorHalt};
use crate::msg::PersonToControllerMsg::{PersonLeavingElevator, PersonLeftElevator, PersonRequestElevator};
use crate::person::PersonStatus::Leaving;
use crate::utils::delay;

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum PersonStatus {
    Idle,
    Entering,
    Choosing,
    InElevator,
    Leaving,
    Done,
}

pub struct Person {
    pub id: String,
    from_controller: Receiver<ControllerToPersonsMsg>,
    to_controller: Sender<PersonToControllerMsg>,
    to_mqtt: Sender<crate::mqtt::Send>,
    state: PersonState
}

#[derive(Debug)]
struct PersonState {
    status: PersonStatus,
    current_floor: Floor,
    destination_floor: Floor,
    elevator: Option<String>,
}

impl Display for PersonState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{ status: {:?}, floor: {:?}, dest: {:?}, elevator: {:?} }}", self.status, self.current_floor, self.destination_floor, self.elevator)
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
    pub fn with(id: &str,
                from_controller_to_persons: Receiver<ControllerToPersonsMsg>,
                from_person_to_controller: Sender<PersonToControllerMsg>,
                to_mqtt: Sender<crate::mqtt::Send>,
                current_floor: Floor,
                destination_floor: Floor) -> Self {
        Person {
            id: id.to_string(),
            from_controller: from_controller_to_persons,
            to_controller: from_person_to_controller,
            to_mqtt,
            state: PersonState {
                status: Idle,
                current_floor,
                destination_floor,
                elevator: None
            }
        }
    }

    pub fn init(mut self) -> JoinHandle<()> {
        tokio::spawn(async move {
            self.request_elevator().await;
            loop {
                if let Ok(msg) = self.from_controller.recv().await {
                    info!("{:?}", msg);
                    match msg {
                        ElevatorHalt(elevator, floor) => {
                            self.handle_elevator_halt(elevator.clone(), floor.clone()).await
                        }
                        UpdateBoardingStatus(person, elevator, boarding_status) => {
                            if self.id.eq(&person) {
                                debug!("UpdateBoardingStatus(person={}, elevator={}, boarding_status={})", person, elevator, boarding_status);
                            }
                            self.handle_update_boarding_status(person, elevator, boarding_status).await
                        }
                    }
                    info!("{:?}", self);
                }
            }
        })
    }

    // Handlers

    pub async fn request_elevator(&mut self) {
        let _ = self.to_controller.send(PersonRequestElevator(self.state.current_floor)).await;
        self.request().await;
    }

    async fn handle_elevator_halt(&mut self, elevator: String, floor: Floor) {
        if self.state.current_floor.eq(&floor) && self.state.status.eq(&Idle) {
            self.state.status = Entering;
            self.status().await;
            let _ = self.to_controller.send(PersonEnteringElevator(self.id.clone(), elevator.clone())).await;
            delay(1);
            let _ = self.to_controller.send(PersonEnteredElevator(self.id.clone(), elevator.clone())).await;
        }
        if self.state.destination_floor.eq(&floor) && self.state.status.eq(&InElevator) && self.state.elevator.eq(&Some(elevator.clone())) {
            self.leave_elevator(self.id.clone(), elevator).await;
            self.state.status = Done;
            self.status().await;
            println!("PersonFinished(id={})", self.id);
        }
    }

    async fn handle_update_boarding_status(&mut self, person: String, elevator: String, boarding_status: BoardingStatus) {
        if self.id.eq(&person) {
            self.boarding(&boarding_status).await;
            match boarding_status {
                Accepted => {
                    self.state.status = InElevator;
                    self.status().await;
                    self.state.elevator = Some(elevator.clone());
                    let _ = self.to_controller.send(PersonChoosingFloor(self.id.clone(), elevator, self.state.destination_floor)).await;
                }
                Rejected => {
                    self.leave_elevator(person, elevator).await;
                    delay(1);
                    let _ = self.to_controller.send(PersonRequestElevator(self.state.current_floor)).await;
                }
            }
        }
    }

    async fn leave_elevator(&mut self, person: String, elevator: String) {
        self.state.elevator = None;
        self.state.status = Leaving;
        self.status().await;
        let _ = self.to_controller.send(PersonLeavingElevator(person.clone(), elevator.clone())).await;
        delay(1);
        self.state.status = Idle;
        self.status().await;
        let _ = self.to_controller.send(PersonLeftElevator(person, elevator.clone())).await;
        delay(1);
    }

    // MQTT-Updates

    async fn status(&mut self) {
        let msg = PersonTopic {
            id: self.id.clone(),
            msg: StatusUpdate {
                status: self.state.status.clone(),
            },
        };
        let _ = self.to_mqtt.send(msg).await;
    }

    async fn boarding(&mut self, status: &BoardingStatus) {
        let msg = PersonTopic {
            id: self.id.clone(),
            msg: Boarding {
                status: status.clone(),
            },
        };
        let _ = self.to_mqtt.send(msg).await;
    }

    async fn request(&mut self) {
        let msg = PersonTopic {
            id: self.id.clone(),
            msg: Request {
                floor: self.state.current_floor,
            },
        };
        let _ = self.to_mqtt.send(msg).await;
    }
}

