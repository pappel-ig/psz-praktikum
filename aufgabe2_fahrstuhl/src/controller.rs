use std::collections::{HashMap, VecDeque};
use std::fmt::{Debug, Display, Formatter};
use std::time::{Duration, Instant};
use log::{debug, info};
use tokio::select;
use tokio::sync::broadcast::Sender;
use tokio::sync::mpsc::Receiver;
use ControllerToElevatorsMsg::CloseDoors;
use ControllerToPersonsMsg::{TooManyPassengers};
use crate::msg::{ControllerToElevatorsMsg, ControllerToPersonsMsg, ElevatorToControllerMsg, PersonToControllerMsg};
use crate::msg::ControllerToElevatorsMsg::{ElevatorMission, OpenDoors};
use crate::msg::ControllerToPersonsMsg::{ElevatorDeparted, ElevatorHalt, UpdateBoardingStatus};
use crate::msg::ElevatorToControllerMsg::{DoorsClosed, DoorsClosing, DoorsOpened, DoorsOpening, ElevatorArrived, ElevatorMoving};
use crate::msg::PersonToControllerMsg::{PersonChoosingFloor, PersonEnteredElevator, PersonEnteringElevator, PersonLeavingElevator, PersonLeftElevator, PersonRequestElevator};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Floor {
    Ground,
    First,
    Second
}

#[derive(Clone, Debug)]
pub enum BoardingStatus {
    Accepted,
    Rejected
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
    passengers: Vec<String>,
    door: DoorStatus,
}

impl Display for Floor {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(self, f)
    }
}

impl Display for BoardingStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(self, f)
    }
}

impl Display for DoorStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(self, f)
    }
}

impl Debug for ElevatorState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "ElevatorState {{ id: {}, floor: {:?}, missions: {:?}, passengers: {:?}, door: {:?} }}",
               self.id, self.floor, self.missions, self.passengers, self.door)
    }
}

impl Display for ElevatorState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}

impl ElevatorState {
    fn new(id: String) -> Self {
        ElevatorState {
            id,
            floor: Floor::Ground,
            missions: VecDeque::new(),
            passengers: Vec::new(),
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
                                }
                                ElevatorArrived(elevator, dest) => {
                                    self.handle_elevator_arrived(elevator.clone(), dest).await;
                                }
                                DoorsOpening(elevator) => {
                                    self.handle_doors_opening(elevator.clone()).await;
                                }
                                DoorsClosing(elevator) => {
                                    self.handle_doors_closing(elevator.clone()).await;
                                }
                                DoorsOpened(elevator) => {
                                    self.handle_doors_opened(elevator.clone()).await;
                                }
                                DoorsClosed(elevator) => {
                                    self.handle_doors_closed(elevator.clone()).await;
                                }
                            }
                    }
                    Some(msg) = self.from_persons.recv() => {
                        match msg {
                                PersonRequestElevator(floor) => {
                                    self.handle_person_request_elevator(floor).await;
                                }
                                PersonEnteringElevator(person, elevator) => {
                                    self.handle_person_entering_elevator(person, elevator.clone()).await;
                                }
                                PersonEnteredElevator(person, elevator) => {
                                    self.handle_person_entered_elevator(person, elevator.clone()).await;
                                }
                                PersonLeavingElevator(person, elevator) => {
                                    self.handle_person_leaving_elevator(person, elevator.clone()).await;
                                }
                                PersonLeftElevator(person, elevator) => {
                                    self.handle_person_left_elevator(person, elevator.clone()).await;
                                }
                                PersonChoosingFloor(person, elevator, floor) => {
                                    self.handle_person_choosing_floor(person, elevator.clone(), floor).await;
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

        debug!("ElevatorArrived(elevator={}, floor={}, missions={}, passengers={}, door={})",
              elevator,
              dest,
              self.state.get(&elevator).unwrap().missions.len(),
              self.state.get(&elevator).unwrap().passengers.len(),
              self.state.get(&elevator).unwrap().door
        );

        let _ = self.to_elevators.send(OpenDoors(elevator.clone())).unwrap();
    }

    async fn handle_doors_opening(&mut self, elevator: String) {
    }

    async fn handle_doors_closing(&mut self, elevator: String) {
    }

    async fn handle_doors_opened(&mut self, elevator: String) {
        let state = self.state.get_mut(&elevator).unwrap();

        state.door = DoorStatus::Open;

        debug!("DoorsOpened(elevator={}, floor={}, missions={}, passengers={}, door={})",
              elevator,
              state.floor,
              state.missions.len(),
              state.passengers.len(),
              state.door
        );

        let _ = self.to_persons.send(ElevatorHalt(elevator.clone(), state.floor));

        if state.passengers.len() > 2 {
            let person_to_leave = state.passengers.last().unwrap();
            let _ = self.to_persons.send(TooManyPassengers(person_to_leave.clone(), elevator.clone()));
        }
    }

    async fn handle_doors_closed(&mut self, elevator: String) {
        let state = self.state.get_mut(&elevator).unwrap();

        state.door = DoorStatus::Closed;

        debug!("DoorsClosed(elevator={}, floor={}, missions={}, passengers={}, door={})",
              elevator,
              state.floor,
              state.missions.len(),
              state.passengers.len(),
              state.door
        );

        if let Some(target) = state.missions.front() {
            if state.passengers.len() <= 2 {
                let _ = self.to_elevators.send(ElevatorMission(elevator.clone(), *target));
                debug!("ElevatorMission(elevator={}, target={}, floor={}, missions={}, passengers={}, door={})",
                    elevator,
                    target,
                    state.floor,
                    state.missions.len(),
                    state.passengers.len(),
                    state.door
                );
            } else {
                let _ = self.to_elevators.send(OpenDoors(elevator.clone()));
                debug!("TooManyPeopleInElevator(elevator={}, target={}, floor={}, missions={}, passengers={}, door={})",
                    elevator,
                    target,
                    state.floor,
                    state.missions.len(),
                    state.passengers.len(),
                    state.door
                );
            }
        }

        let _ = self.to_persons.send(ElevatorDeparted(elevator.clone(), state.floor));
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
        debug!("PersonRequestElevator(target={}, assigned_elevator={}, floor={}, missions={:?}, passengers={:?}, door={})",
              target,
              elevator.id,
              elevator.floor,
              elevator.missions,
              elevator.passengers,
              elevator.door
        );
    }

    async fn handle_person_entering_elevator(&mut self, person: String, elevator: String) {
        let state = self.state.get_mut(&elevator).unwrap();

        state.passengers.push(person.clone());

        if state.passengers.len() > 2 {
            debug!("Rejected(person={}, elevator={})", person, elevator);
            let _ = self.to_persons.send(UpdateBoardingStatus(person, elevator, BoardingStatus::Rejected));
        } else {
            debug!("Accepted(person={}, elevator={})", person, elevator);
            let _ = self.to_persons.send(UpdateBoardingStatus(person, elevator, BoardingStatus::Accepted));
        }
    }

    async fn handle_person_entered_elevator(&mut self, person: String, elevator: String) {
    }

    async fn handle_person_leaving_elevator(&mut self, person: String, elevator: String) {
        let state = self.state.get_mut(&elevator).unwrap();

        state.passengers.retain(|x| x.ne(&person));
        debug!("LeftElevator(person={}, elevator={})", person, elevator);
    }

    async fn handle_person_left_elevator(&mut self, person: String, elevator: String) {
    }

    async fn handle_person_choosing_floor(&mut self, person: String, elevator: String, dest: Floor) {
        let state = self.state.get_mut(&elevator).unwrap();

        if !state.missions.contains(&dest) {
            state.missions.push_back(dest);
        }

        debug!("PersonChooseFloor(person={}, elevator={}, missions={:?})", person, elevator, state.missions);

        if state.door == DoorStatus::Open {
            debug!("ClosingDoor(person={}, elevator={}, missions={:?})", person, elevator, state.missions);
            let _ = self.to_elevators.send(CloseDoors(elevator.clone()));
        }
    }
}