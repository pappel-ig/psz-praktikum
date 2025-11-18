use std::collections::{HashMap, VecDeque};
use std::fmt::{Debug, Formatter};
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;
use log::info;
use tokio::select;
use tokio::sync::broadcast::Sender;
use tokio::sync::mpsc::Receiver;
use ControllerToElevatorsMsg::CloseDoors;
use crate::msg::{ControllerToElevatorsMsg, ElevatorToControllerMsg, PersonToControllerMsg};
use crate::msg::ControllerToElevatorsMsg::{ElevatorMission, OpenDoors};
use crate::msg::ElevatorToControllerMsg::{DoorsClosed, DoorsClosing, DoorsOpened, DoorsOpening, ElevatorArrived, ElevatorMoving};
use crate::msg::PersonToControllerMsg::{PersonChoosingFloor, PersonEnteredElevator, PersonEnteringElevator, PersonLeavingElevator, PersonLeftElevator, PersonRequestElevator};

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
    from_elevators: Receiver<ElevatorToControllerMsg>,
    from_persons: Receiver<PersonToControllerMsg>,
    to_elevators: Sender<ControllerToElevatorsMsg>,
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

    pub fn init(mut self) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            loop {
                select! {
                    Some(msg) = self.from_elevators.recv() => {
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
                    Some(msg) = self.from_persons.recv() => {
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

    pub fn new(from_elevators: Receiver<ElevatorToControllerMsg>,
               to_elevators: Sender<ControllerToElevatorsMsg>,
               from_persons: Receiver<PersonToControllerMsg>,
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
                let _ = self.to_elevators.send(ControllerToElevatorsMsg::ElevatorMission(elevator.clone(), *target));
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
            let _ = self.to_elevators.send(ControllerToElevatorsMsg::OpenDoors(elevator.id.clone()));
        } else if elevator.door == DoorStatus::Closed {
            info!("Sending mission to elevator {} to floor {:?}", elevator.id, target);
            elevator.missions.push_back(target);
            let _ = self.to_elevators.send(ControllerToElevatorsMsg::ElevatorMission(elevator.id.clone(), target));
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

        let _ = self.to_elevators.send(CloseDoors(elevator.clone()));
    }
}