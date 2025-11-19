use std::collections::{HashMap, VecDeque};
use std::fmt::{Debug, Display, Formatter};
use log::info;
use tokio::select;
use tokio::sync::broadcast::Sender;
use tokio::sync::mpsc::Receiver;
use ControllerToElevatorsMsg::CloseDoors;
use ControllerToPersonsMsg::TooManyPassengers;
use crate::msg::{ControllerToElevatorsMsg, ControllerToPersonsMsg, ElevatorToControllerMsg, PersonToControllerMsg};
use crate::msg::ControllerToElevatorsMsg::{ElevatorMission, OpenDoors};
use crate::msg::ControllerToPersonsMsg::ElevatorHalt;
use crate::msg::ElevatorToControllerMsg::{DoorsClosed, DoorsClosing, DoorsOpened, DoorsOpening, ElevatorArrived, ElevatorMoving};
use crate::msg::PersonToControllerMsg::{PersonChoosingFloor, PersonEnteredElevator, PersonEnteringElevator, PersonLeavingElevator, PersonLeftElevator, PersonRequestElevator};

#[derive(Clone, Copy)]
#[derive(PartialEq)]
pub enum Floor {
    Ground,
    First,
    Second
}

#[derive(PartialEq, Debug)]
enum DoorStatus {
    Open,
    Closed
}

pub struct ElevatorController {
    from_elevators: Receiver<ElevatorToControllerMsg>,
    from_persons: Receiver<PersonToControllerMsg>,
    to_elevators: Sender<ControllerToElevatorsMsg>,
    to_persons: Sender<ControllerToPersonsMsg>,
    state: HashMap<String, ElevatorState>,
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

impl Debug for ElevatorState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "ElevatorState {{ id: {}, floor: {:?}, missions: {:?}, passengers: {}, door: {:?} }}",
               self.id, self.floor, self.missions, self.passengers, self.door)
    }
}

impl Display for ElevatorState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "ElevatorState {{ id: {}, floor: {:?}, missions: {:?}, passengers: {}, door: {:?} }}",
               self.id, self.floor, self.missions, self.passengers, self.door)
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
                                ElevatorMoving(elevator, from, to) => {
                                    self.handle_elevator_moving(elevator.clone(), from, to).await;
                                    info!("ElevatorMoving({})", self.state.get(&elevator).unwrap());
                                }
                                ElevatorArrived(elevator, dest) => {
                                    self.handle_elevator_arrived(elevator.clone(), dest).await;
                                    info!("ElevatorArrived({})", self.state.get(&elevator).unwrap());
                                }
                                DoorsOpening(elevator) => {
                                    self.handle_doors_opening(elevator.clone()).await;
                                    info!("DoorsOpening({})", self.state.get(&elevator).unwrap());
                                }
                                DoorsClosing(elevator) => {
                                    self.handle_doors_closing(elevator.clone()).await;
                                    info!("DoorsClosing({})", self.state.get(&elevator).unwrap());
                                }
                                DoorsOpened(elevator) => {
                                    self.handle_doors_opened(elevator.clone()).await;
                                    info!("DoorsOpened({})", self.state.get(&elevator).unwrap());
                                }
                                DoorsClosed(elevator) => {
                                    self.handle_doors_closed(elevator.clone()).await;
                                    info!("DoorsClosed({})", self.state.get(&elevator).unwrap());
                                }
                            }
                    }
                    Some(msg) = self.from_persons.recv() => {
                        match msg {
                                PersonRequestElevator(floor) => {
                                    self.handle_person_request_elevator(floor).await;
                                    info!("PersonRequestElevator({}, {}. {})", self.state.get("Dorisch").unwrap(), self.state.get("Ionisch").unwrap(), self.state.get("Korinthisch").unwrap());
                                }
                                PersonEnteringElevator(person, elevator) => {
                                    self.handle_person_entering_elevator(person, elevator.clone()).await;
                                    info!("PersonEnteringElevator({})", self.state.get(&elevator).unwrap());
                                }
                                PersonEnteredElevator(person, elevator) => {
                                    self.handle_person_entered_elevator(person, elevator.clone()).await;
                                    info!("PersonEnteredElevator({})", self.state.get(&elevator).unwrap());
                                }
                                PersonLeavingElevator(person, elevator) => {
                                    self.handle_person_leaving_elevator(person, elevator.clone()).await;
                                    info!("PersonLeavingElevator({})", self.state.get(&elevator).unwrap());
                                }
                                PersonLeftElevator(person, elevator) => {
                                    self.handle_person_left_elevator(person, elevator.clone()).await;
                                    info!("PersonLeftElevator({})", self.state.get(&elevator).unwrap());
                                }
                                PersonChoosingFloor(person, elevator, floor) => {
                                    self.handle_person_choosing_floor(person, elevator.clone(), floor).await;
                                    info!("PersonChoosingFloor({})", self.state.get(&elevator).unwrap());
                                }
                            }
                    }
                }
            }
        })
    }

    pub fn new(from_elevators: Receiver<ElevatorToControllerMsg>,
               to_elevators: Sender<ControllerToElevatorsMsg>,
               from_persons: Receiver<PersonToControllerMsg>,
               to_persons: Sender<ControllerToPersonsMsg>,
               elevators: Vec<String>) -> Self {
        let mut state = HashMap::new();
        for elevator in elevators {
            state.insert(elevator.clone(), ElevatorState::new(elevator.clone()));
        }
        ElevatorController {
            from_elevators,
            from_persons,
            to_elevators,
            to_persons,
            state
        }
    }

    // Handler Methods von Elevator -> Controller
    async fn handle_elevator_moving(&mut self, elevator: String, from: Floor, to: Floor) {
    }

    async fn handle_elevator_arrived(&mut self, elevator: String, dest: Floor) {
        let entry = self.state.entry(elevator.clone());

        entry.and_modify(|elevator_state| {
            elevator_state.floor = dest;
            elevator_state.missions.pop_front().unwrap();
        });

        let _ = self.to_elevators.send(OpenDoors(elevator.clone())).unwrap();
    }

    async fn handle_doors_opening(&mut self, elevator: String) {
    }

    async fn handle_doors_closing(&mut self, elevator: String) {
    }

    async fn handle_doors_opened(&mut self, elevator: String) {
        let state = self.state.get_mut(&elevator).unwrap();

        state.door = DoorStatus::Open;

        let _ = self.to_persons.send(ElevatorHalt(elevator.clone(), state.floor));
    }

    async fn handle_doors_closed(&mut self, elevator: String) {
        let state = self.state.get_mut(&elevator).unwrap();

        state.door = DoorStatus::Closed;

        if let Some(target) = state.missions.front() {
            if state.passengers <= 2 {
                let _ = self.to_elevators.send(ElevatorMission(elevator.clone(), *target));
            } else {
                let _ = self.to_persons.send(TooManyPassengers(elevator.clone()));
            }
        }

        let _ = self.to_persons.send(ElevatorHalt(elevator.clone(), state.floor));
    }

    // Handler Methods von Person -> Controller
    async fn handle_person_request_elevator(&mut self, target: Floor) {
        let mut values: Vec<&mut ElevatorState> = self.state.values_mut().collect();
        values.sort_by_key(|x| { x.missions.len() });
        let elevator = values.first_mut().unwrap();

        if (elevator.door == DoorStatus::Closed) && (elevator.floor == target) {
            let _ = self.to_elevators.send(OpenDoors(elevator.id.clone()));
        } else if elevator.door == DoorStatus::Closed {
            elevator.missions.push_back(target);
            let _ = self.to_elevators.send(ElevatorMission(elevator.id.clone(), target));
        } else {
            elevator.missions.push_back(target);
        }
    }

    async fn handle_person_entering_elevator(&mut self, person: String, elevator: String) {
        let state = self.state.get_mut(&elevator).unwrap();

        state.passengers += 1;
    }

    async fn handle_person_entered_elevator(&mut self, person: String, elevator: String) {
    }

    async fn handle_person_leaving_elevator(&mut self, person: String, elevator: String) {
        let state = self.state.get_mut(&elevator).unwrap();

        state.passengers -= 1;
    }

    async fn handle_person_left_elevator(&mut self, person: String, elevator: String) {
    }

    async fn handle_person_choosing_floor(&mut self, person: String, elevator: String, dest: Floor) {
        let state = self.state.get_mut(&elevator).unwrap();

        if !state.missions.contains(&dest) {
            state.missions.push_back(dest);
        }

        if state.door == DoorStatus::Open {
            let _ = self.to_elevators.send(CloseDoors(elevator.clone()));
        }
    }
}