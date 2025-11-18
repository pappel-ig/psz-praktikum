use crate::controller::Floor;
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;
use crossbeam_channel::{Receiver, Sender};
use log::{debug, info};
use ControllerToElevatorsMsg::{CloseDoors, ElevatorMission, OpenDoors};
use crate::msg::{ControllerToElevatorsMsg, ElevatorToControllerMsg};
use crate::msg::ElevatorToControllerMsg::{DoorsClosed, DoorsClosing, DoorsOpened, DoorsOpening, ElevatorArrived, ElevatorMoving};

pub enum ElevatorStatus {
    IdleIn(Floor),
    MovingFromTo(Floor, Floor),
    WaitingAt(Floor)
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
    from_controller: Arc<Mutex<Receiver<ControllerToElevatorsMsg>>>,
    to_controller: Arc<Mutex<Sender<ElevatorToControllerMsg>>>,
    state: ElevatorState,
}

struct ElevatorState {
    floor: Floor,
    status: ElevatorStatus,
    doors_status: DoorStatus,
}

impl Elevator {
    pub fn init(mut self) -> JoinHandle<()> {
        let rx = Arc::clone(&self.from_controller);
        thread::spawn(move || {
            loop {
                for msg in rx.lock().unwrap().iter() {
                    match msg {
                        ElevatorMission(elevator, dest)
                        => if self.id.eq(&elevator) { self.handle_mission(dest) },
                        OpenDoors(elevator)
                        => if self.id.eq(&elevator) { self.handle_open_doors() },
                        CloseDoors(elevator)
                        => if self.id.eq(&elevator) { self.handle_close_doors() },
                    }
                }
            }
        })
    }

    pub fn new(id: &str, from_controller: Arc<Mutex<Receiver<ControllerToElevatorsMsg>>>, to_controller: Arc<Mutex<Sender<ElevatorToControllerMsg>>>) -> Self {
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

    fn handle_mission(&mut self, dest: Floor) {
        debug!("Elevator {}: Mission to {:?}", self.id, dest);
        self.state.status = ElevatorStatus::MovingFromTo(self.state.floor, dest);
        self.to_controller.lock().unwrap().send(ElevatorMoving(self.id.clone(), self.state.floor, dest)).unwrap();
        Self::delay(7000);
        debug!("Elevator {}: Arrived at {:?}", self.id, dest);
        self.state.status = ElevatorStatus::IdleIn(dest);
        self.to_controller.lock().unwrap().send(ElevatorArrived(self.id.clone(), dest)).unwrap();
    }

    fn handle_open_doors(&mut self) {
        debug!("Elevator {}: Opening doors", self.id);
        self.state.doors_status = DoorStatus::Opening;
        self.to_controller.lock().unwrap().send(DoorsOpening(self.id.clone())).unwrap();
        Self::delay(500);
        debug!("Elevator {}: Doors opened", self.id);
        self.state.doors_status = DoorStatus::Open;
        self.to_controller.lock().unwrap().send(DoorsOpened(self.id.clone())).unwrap();
    }

    fn handle_close_doors(&mut self) {
        debug!("Elevator {}: Closing doors", self.id);
        self.state.doors_status = DoorStatus::Closing;
        self.to_controller.lock().unwrap().send(DoorsClosing(self.id.clone())).unwrap();
        Self::delay(500);
        debug!("Elevator {}: Doors closed", self.id);
        self.state.doors_status = DoorStatus::Closed;
        self.to_controller.lock().unwrap().send(DoorsClosed(self.id.clone())).unwrap();
    }

    fn delay(ms: u64) {
        thread::sleep(std::time::Duration::from_millis(ms));
    }
}

