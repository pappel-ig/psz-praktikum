use crate::controller::Floor;
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;
use crossbeam_channel::{Receiver, Sender};
use ControllerToElevatorsMsg::{CloseDoors, ElevatorMission, OpenDoors};
use crate::msg::{ControllerToElevatorsMsg, ElevatorToControllerMsg};

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
    status: ElevatorStatus,
    doors_status: DoorStatus,
}

impl Elevator {
    pub fn init(mut self) -> JoinHandle<()> {
        let rx = Arc::clone(&self.from_controller);
        thread::spawn(move || {
            for msg in rx.lock().unwrap().iter() {
                match msg {
                    ElevatorMission(_elevator_id, _target_floor) => {},
                    OpenDoors(_elevator_id) => {},
                    CloseDoors(_elevator_id) => {}
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
                status: ElevatorStatus::IdleIn(Floor::Ground),
                doors_status: DoorStatus::Closed
            }
        }
    }
}

