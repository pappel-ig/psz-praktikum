use std::collections::{HashMap, VecDeque};
use crossbeam_channel::{select, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;
use ElevatorToControllerMsg::{DoorsClosed, DoorsClosing, DoorsOpened, DoorsOpening, ElevatorArrived};
use PersonToControllerMsg::{PersonRequestElevator, PersonEnteringElevator, PersonChoosingFloor};
use crate::msg::{ControllerToElevatorsMsg, ElevatorToControllerMsg, PersonToControllerMsg};
use crate::msg::ElevatorToControllerMsg::ElevatorMoving;
use crate::msg::PersonToControllerMsg::{PersonEnteredElevator, PersonLeavingElevator, PersonLeftElevator};

#[derive(Clone, Copy)]
pub enum Floor {
    Ground,
    First,
    Second
}

pub struct ElevatorController {
    from_elevators: Arc<Mutex<Receiver<ElevatorToControllerMsg>>>,
    from_persons: Arc<Mutex<Receiver<PersonToControllerMsg>>>,
    to_elevators: Arc<Mutex<Sender<ControllerToElevatorsMsg>>>,
    state: HashMap<String, ElevatorState>
}

struct ElevatorState {
    floor: Floor,
    missions: VecDeque<Floor>,
    passengers: usize
}

impl Default for ElevatorState {
    fn default() -> Self {
        ElevatorState {
            floor: Floor::Ground,
            missions: VecDeque::new(),
            passengers: 0
        }
    }
}

impl ElevatorController {

    pub fn init(mut self) -> JoinHandle<()> {
        let from_elevators = Arc::clone(&self.from_elevators);
        let from_persons = Arc::clone(&self.from_persons);
        thread::spawn(move || {
            select! {
                recv(from_elevators.lock().unwrap()) -> msg_result => {
                    if let Ok(msg) = msg_result  {
                        match msg {
                            ElevatorMoving(elevator, from, to)
                                => { self.handle_elevator_moving(elevator, from, to) }
                            ElevatorArrived(elevator, dest)
                                => { self.handle_elevator_arrived(elevator, dest) }
                            DoorsOpening(elevator)
                                => { self.handle_doors_opening(elevator) }
                            DoorsClosing(elevator)
                                => { self.handle_doors_closing(elevator) }
                            DoorsOpened(elevator)
                                => { self.handle_doors_opened(elevator) }
                            DoorsClosed(elevator)
                                => { self.handle_doors_closed(elevator) }
                        }
                    }

                }
                recv(from_persons.lock().unwrap()) -> msg_result => {
                    if let Ok(msg) = msg_result  {
                        match msg {
                            PersonRequestElevator(floor)
                                => { self.handle_person_request_elevator(floor) }
                            PersonEnteringElevator(person, elevator)
                                => { self.handle_person_entering_elevator(person, elevator) }
                            PersonEnteredElevator(person, elevator)
                                => { self.handle_person_entered_elevator(person, elevator) }
                            PersonLeavingElevator(person, elevator)
                                => { self.handle_person_leaving_elevator(person, elevator) }
                            PersonLeftElevator(person, elevator)
                                => { self.handle_person_left_elevator(person, elevator) }
                            PersonChoosingFloor(person, elevator, floor)
                                => { self.handle_person_choosing_floor(person, elevator, floor) }
                        }
                    }
                }
            }
        })
    }

    pub fn new(from_elevators: Arc<Mutex<Receiver<ElevatorToControllerMsg>>>,
               to_elevators: Arc<Mutex<Sender<ControllerToElevatorsMsg>>>,
               from_persons: Arc<Mutex<Receiver<PersonToControllerMsg>>>,
               elevators: Vec<String>) -> Self {
        let mut state = HashMap::new();
        for elevator in elevators {
            state.insert(elevator, Default::default());
        }
        ElevatorController {
            from_elevators,
            from_persons,
            to_elevators,
            state
        }
    }

    // Handler Methods von Elevator -> Controller
    fn handle_elevator_moving(&self, elevator: String, from: Floor, to: Floor) {
        todo!()
    }

    fn handle_elevator_arrived(&mut self, elevator: String, dest: Floor) {
        todo!()
    }

    fn handle_doors_opening(&self, elevator: String) {
        todo!()
    }

    fn handle_doors_closing(&self, elevator: String) {
        todo!()
    }

    fn handle_doors_opened(&self, elevator: String) {
        todo!()
    }

    fn handle_doors_closed(&self, elevator: String) {
        todo!()
    }

    // Handler Methods von Person -> Controller
    fn handle_person_request_elevator(&self, source: Floor) {
        todo!()
    }

    fn handle_person_entering_elevator(&self, person: String, elevator: String) {
        todo!()
    }

    fn handle_person_entered_elevator(&self, person: String, elevator: String) {
        todo!()
    }

    fn handle_person_leaving_elevator(&self, person: String, elevator: String) {
        todo!()
    }

    fn handle_person_left_elevator(&self, person: String, elevator: String) {
        todo!()
    }

    fn handle_person_choosing_floor(&self, person: String, elevator: String, dest: Floor) {
        todo!()
    }
}