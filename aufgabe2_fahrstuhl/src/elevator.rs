use crate::controller::Floor;
use crate::elevator::DoorStatus::{Closed, Open};
use crate::elevator::ElevatorStatus::IdleIn;
use crate::mqtt::ElevatorMsg::{Door, Position};
use crate::mqtt::Send::ElevatorTopic;
use crate::msg::ElevatorToControllerMsg::{DoorsClosed, DoorsClosing, DoorsOpened, DoorsOpening, ElevatorArrived, ElevatorMoving};
use crate::msg::{ControllerToElevatorsMsg, ElevatorToControllerMsg};
use crate::utils;
use serde::{Deserialize, Serialize};
use task::JoinHandle;
use tokio::sync::broadcast::Receiver;
use tokio::sync::mpsc::Sender;
use tokio::task;
use utils::delay;
use ControllerToElevatorsMsg::{CloseDoors, ElevatorMission, OpenDoors};
use DoorStatus::{Closing, Opening};
use ElevatorStatus::MovingFromTo;
use Floor::Ground;

#[derive(PartialEq)]
pub enum ElevatorStatus {
    IdleIn(Floor),
    MovingFromTo(Floor, Floor)
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[derive()]
pub enum DoorStatus {
    Closed,
    Opening,
    Open,
    Closing
}

pub struct Elevator {
    pub id: String,
    from_controller: Receiver<ControllerToElevatorsMsg>,
    to_controller: Sender<ElevatorToControllerMsg>,
    state: ElevatorState,
    pub to_mqtt: Sender<crate::mqtt::Send>,
}

#[derive(PartialEq)]
struct ElevatorState {
    floor: Floor,
    status: ElevatorStatus,
    doors_status: DoorStatus,
}

impl Elevator {
    pub fn init(mut self) -> JoinHandle<()> {
        tokio::spawn(async move {
            loop {
                if let Ok(msg) = self.from_controller.recv().await {
                    match msg {
                        ElevatorMission(elevator, dest) => {
                            if self.id.eq(&elevator) {
                                self.handle_mission(dest).await
                            }
                        }
                        OpenDoors(elevator) => {
                            if self.id.eq(&elevator) {
                                self.handle_open_doors().await
                            }
                        }
                        CloseDoors(elevator) => {
                            if self.id.eq(&elevator) {
                                self.handle_close_doors().await
                            }
                        }
                    }
                }
            }
        })
    }

    pub fn new(id: &str, from_controller: Receiver<ControllerToElevatorsMsg>, to_controller: Sender<ElevatorToControllerMsg>, to_mqtt: Sender<crate::mqtt::Send>) -> Self {
        Elevator {
            id: id.to_string(),
            from_controller,
            to_controller,
            to_mqtt,
            state: ElevatorState {
                floor: Ground,
                status: IdleIn(Ground),
                doors_status: Closed,
            }
        }
    }

    // Handlers

    async fn handle_mission(&mut self, dest: Floor) {
        self.state.status = MovingFromTo(self.state.floor, dest);
        let _ = self.to_controller.send(ElevatorMoving(self.id.clone(), self.state.floor, dest)).await;
        delay(1);
        self.state.status = IdleIn(dest);
        self.position().await;
        let _ = self.to_controller.send(ElevatorArrived(self.id.clone(), dest)).await;
    }

    async fn handle_open_doors(&mut self) {
        if self.state.doors_status.eq(&Closed) {

            self.state.doors_status = Opening;
            self.door().await;
            let _ = self.to_controller.send(DoorsOpening(self.id.clone())).await;
            delay(1);
            self.state.doors_status = Open;
            self.door().await;
            let _ = self.to_controller.send(DoorsOpened(self.id.clone())).await;
        }
    }

    async fn handle_close_doors(&mut self) {
        if self.state.doors_status.eq(&Open) {
            self.state.doors_status = Closing;
            self.door().await;
            let _ = self.to_controller.send(DoorsClosing(self.id.clone())).await;
            delay(1);
            self.state.doors_status = Closed;
            self.door().await;
            let _ = self.to_controller.send(DoorsClosed(self.id.clone())).await;
        }
    }

    // MQTT Updates

    async fn position(&mut self) {
        let msg = ElevatorTopic {
            id: self.id.clone(),
            msg: Position {
                floor: self.state.floor
            },
        };
        let _ = self.to_mqtt.send(msg).await;
    }

    async fn door(&mut self) {
        let msg = ElevatorTopic {
            id: self.id.clone(),
            msg: Door {
                status: self.state.doors_status.clone()
            },
        };
        let _ = self.to_mqtt.send(msg).await;
    }
}

