use std::collections::{HashMap, VecDeque};
use std::fmt::{Debug, Display, Formatter};
use std::thread::spawn;
use log::{info, trace};
use serde::{Deserialize, Serialize};
use tokio::{select, sync};
use tokio::sync::broadcast::Sender;
use tokio::sync::mpsc::Receiver;
use tokio::task::JoinHandle;
use ControllerToElevatorsMsg::CloseDoors;
use crate::msg::{ControllerToElevatorsMsg, ControllerToPersonsMsg, ElevatorToControllerMsg, PersonToControllerMsg};
use crate::msg::ControllerToElevatorsMsg::{ElevatorMission, OpenDoors};
use crate::msg::ControllerToPersonsMsg::{ElevatorHalt, UpdateBoardingStatus};
use crate::msg::ElevatorToControllerMsg::{DoorsClosed, DoorsClosing, DoorsOpened, DoorsOpening, ElevatorArrived, ElevatorMoving};
use crate::msg::PersonToControllerMsg::{PersonChoosingFloor, PersonEnteredElevator, PersonEnteringElevator, PersonLeavingElevator, PersonLeftElevator, PersonRequestElevator};
use sync::mpsc;
use DoorStatus::Open;
use crate::controller::DoorStatus::Closed;
use crate::mqtt::ElevatorMsg::{Missions, Moving, Passengers};
use crate::mqtt::Send::ElevatorTopic;

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub enum Floor {
    Ground = 1,
    First = 2,
    Second = 3,
    Third = 4
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum BoardingStatus {
    Accepted,
    Rejected
}

#[derive(PartialEq, Debug, Clone)]
enum DoorStatus {
    Open,
    Closed
}

pub struct ElevatorController {
    from_elevators: Receiver<ElevatorToControllerMsg>,
    from_persons: Receiver<PersonToControllerMsg>,
    to_elevators: Sender<ControllerToElevatorsMsg>,
    to_persons: Sender<ControllerToPersonsMsg>,
    to_mqtt: mpsc::Sender<crate::mqtt::Send>,
    state: HashMap<String, ElevatorState>,
}

struct ElevatorState {
    id: String,
    floor: Floor,
    mission: Option<Floor>,
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
        write!(f, "ElevatorState {{ id: {}, floor: {:?}, mission={:?}, missions: {:?}, passengers: {:?}, door: {:?} }}",
               self.id, self.floor, self.mission, self.missions, self.passengers, self.door)
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
            mission: None,
            missions: VecDeque::new(),
            passengers: Vec::new(),
            door: Closed,
        }
    }
}

impl ElevatorController {

    pub fn init(mut self) -> JoinHandle<()> {
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
                        info!("{:?}", self.state);
                    }
                    Some(msg) = self.from_persons.recv() => {
                        info!("{:?}", msg);
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
                        info!("{:?}", self.state);
                    }
                }
            }
        })
    }

    pub fn new(from_elevators: Receiver<ElevatorToControllerMsg>,
               to_elevators: Sender<ControllerToElevatorsMsg>,
               from_persons: Receiver<PersonToControllerMsg>,
               to_persons: Sender<ControllerToPersonsMsg>,
               to_mqtt: mpsc::Sender<crate::mqtt::Send>,
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
            to_mqtt,
            state,
        }
    }

    // Handlers

    async fn handle_elevator_moving(&mut self, elevator: String, from: Floor, to: Floor) {
        ElevatorController::moving(self.to_mqtt.clone(), elevator, from, to);
    }

    async fn handle_elevator_arrived(&mut self, elevator: String, dest: Floor) {
        let state = self.state.get_mut(&elevator).unwrap();

        state.floor = dest;

        let _ = self.to_elevators.send(OpenDoors(elevator.clone())).unwrap();
    }

    async fn handle_doors_opening(&mut self, elevator: String) {
        let state = self.state.get_mut(&elevator).unwrap();

        state.mission = None;
        state.missions.retain(|&floor| floor != state.floor);
        state.door = Open;

        let _ = self.to_persons.send(ElevatorHalt(elevator.clone(), state.floor));

        let missions = state.missions.iter().cloned().collect();
        let id = elevator.clone();
        ElevatorController::missions(self.to_mqtt.clone(), id, missions);
    }

    async fn handle_doors_closing(&mut self, elevator: String) {

    }

    async fn handle_doors_opened(&mut self, elevator: String) {
        let _ = self.to_elevators.send(CloseDoors(elevator.clone())).unwrap();
    }

    async fn handle_doors_closed(&mut self, elevator: String) {
        let state = self.state.get_mut(&elevator).unwrap();

        state.door = Closed;

        self.start_next_mission_if_idle(&elevator);
    }

    fn start_next_mission_if_idle(&mut self, elevator_id: &str) {
        let state = self.state.get_mut(elevator_id).unwrap();

        if state.mission.is_none() {
            if let Some(next_floor) = state.missions.pop_front() {
                state.mission = Some(next_floor);
                let _ = self.to_elevators.send(ElevatorMission(elevator_id.to_string(), next_floor));
            }
        }
    }

    async fn handle_person_request_elevator(&mut self, target: Floor) {
        let (_, elevator) = self.state
            .iter_mut()
            .min_by_key(|(_, s)| s.missions.len())
            .unwrap();

        if elevator.mission.is_none() && elevator.missions.is_empty() {
            elevator.mission = Some(target);
            let _ = self.to_elevators.send(ElevatorMission(elevator.id.clone(), target));
        } else if !elevator.missions.contains(&target) {
            elevator.missions.push_back(target);
        }
    }

    async fn handle_person_entering_elevator(&mut self, person: String, elevator: String) {

    }

    async fn handle_person_entered_elevator(&mut self, person: String, elevator: String) {
        let state = self.state.get_mut(&elevator).unwrap();
        if state.passengers.len() < 2 && state.door.eq(&Open) {
            state.passengers.push(person.clone());
            let _ = self.to_persons.send(UpdateBoardingStatus(person, elevator.clone(), BoardingStatus::Accepted));
        } else {
            let _ = self.to_persons.send(UpdateBoardingStatus(person, elevator.clone(), BoardingStatus::Rejected));
        }

        let passengers = state.passengers.clone();
        ElevatorController::passengers(self.to_mqtt.clone(), elevator, passengers);
    }

    async fn handle_person_leaving_elevator(&mut self, person: String, elevator: String) {

    }

    async fn handle_person_left_elevator(&mut self, person: String, elevator: String) {
        let state = self.state.get_mut(&elevator).unwrap();

        state.passengers.retain(|x| { x.ne(&person) });

        let passengers = state.passengers.clone();
        ElevatorController::passengers(self.to_mqtt.clone(), elevator, passengers);
    }

    async fn handle_person_choosing_floor(&mut self, person: String, elevator: String, dest: Floor) {
        let state = self.state.get_mut(&elevator).unwrap();

        if !state.missions.contains(&dest) {
            state.missions.push_back(dest);
            let missions = state.missions.iter().cloned().collect();
            ElevatorController::missions(self.to_mqtt.clone(), elevator.clone(), missions);
        }

        if state.door.eq(&Open) {
            let _ = self.to_elevators.send(CloseDoors(elevator));
        } else {
            self.start_next_mission_if_idle(&elevator);
        }
    }

    // MQTT Updates

    fn moving(to_mqtt: mpsc::Sender<crate::mqtt::Send>, elevator: String, from: Floor, to: Floor) {
        tokio::spawn(async move {
            let msg = ElevatorTopic {
                id: elevator,
                msg: Moving {
                    from,
                    to
                },
            };
            let _ = to_mqtt.send(msg).await;
        });
    }

    fn passengers(to_mqtt: mpsc::Sender<crate::mqtt::Send>, elevator: String, passengers: Vec<String>) {
        tokio::spawn(async move {
            let msg = ElevatorTopic {
                id: elevator,
                msg: Passengers {
                    passengers
                },
            };
            let _ = to_mqtt.send(msg).await;
        });
    }

    fn missions(to_mqtt: mpsc::Sender<crate::mqtt::Send>, elevator: String, missions: Vec<Floor>) {
        tokio::spawn(async move {
            let msg = ElevatorTopic {
                id: elevator,
                msg: Missions {
                    missions
                },
            };
            let _ = to_mqtt.send(msg).await;
        });
    }
}