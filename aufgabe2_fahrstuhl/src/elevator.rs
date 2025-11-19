use crate::controller::Floor;
use log::{debug, info};
use tokio::sync::broadcast::Receiver;
use tokio::sync::mpsc::Sender;
use ControllerToElevatorsMsg::{CloseDoors, ElevatorMission, OpenDoors};
use utils::delay;
use crate::msg::{ControllerToElevatorsMsg, ElevatorToControllerMsg};
use crate::msg::ElevatorToControllerMsg::{DoorsClosed, DoorsClosing, DoorsOpened, DoorsOpening, ElevatorArrived, ElevatorMoving};
use crate::utils;

pub enum ElevatorStatus {
    IdleIn(Floor),
    MovingFromTo(Floor, Floor)
}

#[derive(Clone)]
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
}

struct ElevatorState {
    floor: Floor,
    status: ElevatorStatus,
    doors_status: DoorStatus,
}

impl Elevator {
    pub fn init(mut self) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            loop {
                if let Ok(msg) = self.from_controller.recv().await {
                    match msg {
                        ElevatorMission(elevator, dest) => {
                            if self.id.eq(&elevator) { self.handle_mission(dest).await }
                        }
                        OpenDoors(elevator) => {
                            if self.id.eq(&elevator) { self.handle_open_doors().await }
                        }
                        CloseDoors(elevator) => {
                            if self.id.eq(&elevator) { self.handle_close_doors().await }
                        }
                    }
                }
            }
        })
    }

    pub fn new(id: &str, from_controller: Receiver<ControllerToElevatorsMsg>, to_controller: Sender<ElevatorToControllerMsg>) -> Self {
        Elevator {
            id: id.to_string(),
            from_controller,
            to_controller,
            state: ElevatorState {
                floor: Floor::Ground,
                status: ElevatorStatus::IdleIn(Floor::Ground),
                doors_status: DoorStatus::Closed,
            }
        }
    }

    async fn handle_mission(&mut self, dest: Floor) {
        debug!("Elevator {}: Mission to {:?}", self.id, dest);
        self.state.status = ElevatorStatus::MovingFromTo(self.state.floor, dest);
        let _ = self.to_controller.send(ElevatorMoving(self.id.clone(), self.state.floor, dest)).await;
        delay(1);
        debug!("Elevator {}: Arrived at {:?}", self.id, dest);
        self.state.status = ElevatorStatus::IdleIn(dest);
        let _ = self.to_controller.send(ElevatorArrived(self.id.clone(), dest)).await;
    }

    async fn handle_open_doors(&mut self) {
        debug!("Elevator {}: Opening doors", self.id);
        self.state.doors_status = DoorStatus::Opening;
        let _ = self.to_controller.send(DoorsOpening(self.id.clone())).await;
        delay(1);
        debug!("Elevator {}: Doors opened", self.id);
        self.state.doors_status = DoorStatus::Open;
        let _ = self.to_controller.send(DoorsOpened(self.id.clone())).await;
    }

    async fn handle_close_doors(&mut self) {
        debug!("Elevator {}: Closing doors", self.id);
        self.state.doors_status = DoorStatus::Closing;
        let _ = self.to_controller.send(DoorsClosing(self.id.clone())).await;
        delay(1);
        debug!("Elevator {}: Doors closed", self.id);
        self.state.doors_status = DoorStatus::Closed;
        let _ = self.to_controller.send(DoorsClosed(self.id.clone())).await;
    }
}

