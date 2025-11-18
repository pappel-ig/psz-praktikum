use std::collections::{HashMap, VecDeque};
use std::fmt::{Debug, Formatter};
use crossbeam_channel::{select, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;
use log::info;
use ControllerToElevatorsMsg::CloseDoors;
use ElevatorToControllerMsg::{DoorsClosed, DoorsClosing, DoorsOpened, DoorsOpening, ElevatorArrived};
use PersonToControllerMsg::{PersonRequestElevator, PersonEnteringElevator, PersonChoosingFloor};
use crate::msg::{ControllerToElevatorsMsg, ElevatorToControllerMsg, PersonToControllerMsg};
use crate::msg::ElevatorToControllerMsg::ElevatorMoving;
use crate::msg::PersonToControllerMsg::{PersonEnteredElevator, PersonLeavingElevator, PersonLeftElevator};

#[derive(Clone, Copy)]
#[derive(PartialEq)]
pub enum Floor {
    Ground,
    First,
    Second
}

#[derive(PartialEq)]
enum DoorStatus {
    Open,
    Closed
}

pub struct ElevatorController {
    from_elevators: Arc<Mutex<Receiver<ElevatorToControllerMsg>>>,
    from_persons: Arc<Mutex<Receiver<PersonToControllerMsg>>>,
    to_elevators: Arc<Mutex<Sender<ControllerToElevatorsMsg>>>,
    state: HashMap<String, ElevatorState>
}

struct ElevatorState {
    id: String,
    floor: Floor,
    missions: VecDeque<Floor>,
    passengers: usize,
    door: DoorStatus,
}

impl Debug for Floor {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Floor::Ground => write!(f, "Ground"),
            Floor::First => write!(f, "First"),
            Floor::Second => write!(f, "Second"),
        }
    }
}

impl ElevatorState {
    fn new(id: String) -> Self {
        ElevatorState {
            id,
            floor: Floor::Ground,
            missions: VecDeque::new(),
            passengers: 0,
            door: DoorStatus::Closed,
        }
    }
}

impl ElevatorController {

    pub fn init(mut self) -> JoinHandle<()> {
        let from_elevators = Arc::clone(&self.from_elevators);
        let from_persons = Arc::clone(&self.from_persons);
        thread::spawn(move || {
            loop {
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
            }
        })
    }

    pub fn new(from_elevators: Arc<Mutex<Receiver<ElevatorToControllerMsg>>>,
               to_elevators: Arc<Mutex<Sender<ControllerToElevatorsMsg>>>,
               from_persons: Arc<Mutex<Receiver<PersonToControllerMsg>>>,
               elevators: Vec<String>) -> Self {
        let mut state = HashMap::new();
        for elevator in elevators {
            state.insert(elevator.clone(), ElevatorState::new(elevator.clone()));
        }
        ElevatorController {
            from_elevators,
            from_persons,
            to_elevators,
            state
        }
    }

    // Handler Methods von Elevator -> Controller
    fn handle_elevator_moving(&mut self, elevator: String, from: Floor, to: Floor) {
        info!("Elevator {} moving from {:?} to {:?}", elevator, from, to);
    }

    fn handle_elevator_arrived(&mut self, elevator: String, dest: Floor) {
        info!("Elevator {} arrived at {:?}", elevator.clone(), dest);
        let entry = self.state.entry(elevator);

        entry.and_modify(|elevator_state| {
            elevator_state.floor = dest;
            elevator_state.missions.pop_front().unwrap();
        });

    }

    fn handle_doors_opening(&mut self, elevator: String) {
        info!("Elevator {} doors opening", elevator);
    }

    fn handle_doors_closing(&mut self, elevator: String) {
        info!("Elevator {} doors closing", elevator);
    }

    fn handle_doors_opened(&mut self, elevator: String) {
        info!("Elevator {} doors opened", elevator);
        let entry = self.state.entry(elevator);

        entry.and_modify(|elevator_state| {
            elevator_state.door = DoorStatus::Open;
        });
    }

    fn handle_doors_closed(&mut self, elevator: String) {
        info!("Elevator {} doors closed", elevator);
        let state = self.state.get_mut(&elevator).unwrap();

        state.door = DoorStatus::Closed;

        if let Some(target) = state.missions.front() {
            if state.passengers <= 2 {
                self.to_elevators.lock().unwrap().send(ControllerToElevatorsMsg::ElevatorMission(elevator.clone(), *target)).unwrap();
            }
        }
    }

    // Handler Methods von Person -> Controller
    fn handle_person_request_elevator(&mut self, target: Floor) {
        info!("Person requested elevator to {:?}", target);
        let mut values: Vec<&mut ElevatorState> = self.state.values_mut().collect();
        values.sort_by_key(|x| { x.missions.len() });
        let mut elevator = values.first_mut().unwrap();

        if (elevator.door == DoorStatus::Closed) && (elevator.floor == target) {
            info!("Sending open doors command to elevator {}", elevator.id);
            self.to_elevators.lock().unwrap().send(ControllerToElevatorsMsg::OpenDoors(elevator.id.clone())).unwrap();
        } else if elevator.door == DoorStatus::Closed {
            info!("Sending mission to elevator {} to floor {:?}", elevator.id, target);
            elevator.missions.push_back(target);
            self.to_elevators.lock().unwrap().send(ControllerToElevatorsMsg::ElevatorMission(elevator.id.clone(), target)).unwrap();
        } else {
            info!("Adding mission to elevator {} to floor {:?}", elevator.id, target);
            elevator.missions.push_back(target);
        }
    }

    fn handle_person_entering_elevator(&mut self, person: String, elevator: String) {
        info!("Person {} entering elevator {}", person, elevator);
        let state = self.state.get_mut(&elevator).unwrap();

        state.passengers += 1;
    }

    fn handle_person_entered_elevator(&mut self, person: String, elevator: String) {
        info!("Person {} entered elevator {}", person, elevator);
    }

    fn handle_person_leaving_elevator(&mut self, person: String, elevator: String) {
        info!("Person {} leaving elevator {}", person, elevator);
        let state = self.state.get_mut(&elevator).unwrap();

        state.passengers -= 1;
    }

    fn handle_person_left_elevator(&mut self, person: String, elevator: String) {
        info!("Person {} left elevator {}", person, elevator);
    }

    fn handle_person_choosing_floor(&mut self, person: String, elevator: String, dest: Floor) {
        info!("Person {} chose floor {:?} in elevator {}", person, dest, elevator);
        let state = self.state.get_mut(&elevator).unwrap();

        if !state.missions.contains(&dest) {
            state.missions.push_back(dest);
        }

        self.to_elevators.lock().unwrap().send(CloseDoors(elevator.clone())).unwrap();
    }
}