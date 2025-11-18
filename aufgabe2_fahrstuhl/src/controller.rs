use crossbeam_channel::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;
use ElevatorToControllerMsg::{DoorsClosed, DoorsClosing, DoorsOpened, DoorsOpening, ElevatorArrived};
use crate::msg::{ControllerToElevatorsMsg, ElevatorToControllerMsg};
use crate::msg::ElevatorToControllerMsg::ElevatorMoving;

#[derive(Clone, Copy)]
pub enum Floor {
    Ground,
    First,
    Second
}

pub struct ElevatorController {
    from_elevators: Arc<Mutex<Receiver<ElevatorToControllerMsg>>>,
    to_elevators: Arc<Mutex<Sender<ControllerToElevatorsMsg>>>,
}

impl ElevatorController {

    pub fn init(mut self) -> JoinHandle<()> {
        let rx = Arc::clone(&self.from_elevators);
        thread::spawn(move || {
            for msg in rx.lock().unwrap().iter() {
                match msg {
                    ElevatorMoving(elevator, from, to) => {}
                    ElevatorArrived(elevator, dest) => {}
                    DoorsOpening(elevator) => {}
                    DoorsClosing(elevator) => {}
                    DoorsOpened(elevator) => {}
                    DoorsClosed(elevator) => {}
                }
            }
        })
    }

    pub fn new(from_elevators: Arc<Mutex<Receiver<ElevatorToControllerMsg>>>, to_elevators: Arc<Mutex<Sender<ControllerToElevatorsMsg>>>) -> Self {
        ElevatorController {
            from_elevators,
            to_elevators,
        }
    }
}

