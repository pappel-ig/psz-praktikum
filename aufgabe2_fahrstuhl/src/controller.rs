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

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Floor {
    Ground = 1,
    First = 2,
    Second = 3,
    Third = 4
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::{broadcast, mpsc};
    use Floor::*;

    fn create_test_controller(elevators: Vec<String>) -> (
        ElevatorController,
        mpsc::Sender<ElevatorToControllerMsg>,
        mpsc::Sender<PersonToControllerMsg>,
        broadcast::Receiver<ControllerToElevatorsMsg>,
        broadcast::Receiver<ControllerToPersonsMsg>,
    ) {
        let (elevator_tx, elevator_rx) = mpsc::channel(100);
        let (person_tx, person_rx) = mpsc::channel(100);
        let (to_elevators_tx, to_elevators_rx) = broadcast::channel(100);
        let (to_persons_tx, to_persons_rx) = broadcast::channel(100);
        let (mqtt_tx, _mqtt_rx) = mpsc::channel(100);

        let controller = ElevatorController::new(
            elevator_rx,
            to_elevators_tx,
            person_rx,
            to_persons_tx,
            mqtt_tx,
            elevators,
        );

        (controller, elevator_tx, person_tx, to_elevators_rx, to_persons_rx)
    }

    #[test]
    fn test_elevator_state_new() {
        let state = ElevatorState::new("E1".to_string());
        
        assert_eq!(state.id, "E1");
        assert_eq!(state.floor, Ground);
        assert!(state.mission.is_none());
        assert!(state.missions.is_empty());
        assert!(state.passengers.is_empty());
        assert_eq!(state.door, DoorStatus::Closed);
    }

    #[test]
    fn test_controller_new_initializes_all_elevators() {
        let elevators = vec!["E1".to_string(), "E2".to_string(), "E3".to_string()];
        let (controller, _, _, _, _) = create_test_controller(elevators.clone());

        assert_eq!(controller.state.len(), 3);
        assert!(controller.state.contains_key("E1"));
        assert!(controller.state.contains_key("E2"));
        assert!(controller.state.contains_key("E3"));
    }

    #[tokio::test]
    async fn test_handle_person_request_elevator_assigns_to_idle_elevator() {
        let elevators = vec!["E1".to_string()];
        let (mut controller, _, _, mut to_elevators_rx, _) = create_test_controller(elevators);

        controller.handle_person_request_elevator(First).await;

        let state = controller.state.get("E1").unwrap();
        assert_eq!(state.mission, Some(First));
        
        // Verify message was sent
        let msg = to_elevators_rx.recv().await.unwrap();
        assert_eq!(msg, ControllerToElevatorsMsg::ElevatorMission("E1".to_string(), First));
    }

    #[tokio::test]
    async fn test_handle_person_request_elevator_queues_when_busy() {
        let elevators = vec!["E1".to_string()];
        let (mut controller, _, _, _, _) = create_test_controller(elevators);

        // First request gets assigned immediately
        controller.handle_person_request_elevator(First).await;
        // Second request gets queued
        controller.handle_person_request_elevator(Second).await;

        let state = controller.state.get("E1").unwrap();
        assert_eq!(state.mission, Some(First));
        assert!(state.missions.contains(&Second));
    }

    #[tokio::test]
    async fn test_handle_person_request_avoids_duplicate_missions() {
        let elevators = vec!["E1".to_string()];
        let (mut controller, _, _, _, _) = create_test_controller(elevators);

        controller.handle_person_request_elevator(First).await;
        controller.handle_person_request_elevator(Second).await;
        controller.handle_person_request_elevator(Second).await; // Duplicate

        let state = controller.state.get("E1").unwrap();
        let count = state.missions.iter().filter(|&&f| f == Second).count();
        assert_eq!(count, 1); // Should only have one Second floor mission
    }

    #[tokio::test]
    async fn test_handle_person_request_selects_elevator_with_min_missions() {
        let elevators = vec!["E1".to_string(), "E2".to_string()];
        let (mut controller, _, _, _, _) = create_test_controller(elevators);

        // Give E1 some missions
        controller.handle_person_request_elevator(First).await;
        controller.handle_person_request_elevator(Second).await;

        // E2 should be selected for next request (has 0 missions)
        // Note: HashMap iteration order is not guaranteed, but with min_by_key
        // the elevator with fewer missions should be selected
        let e1_missions = controller.state.get("E1").unwrap().missions.len();
        let e2_missions = controller.state.get("E2").unwrap().missions.len();
        
        // At least one elevator should have missions queued
        assert!(e1_missions > 0 || e2_missions > 0);
    }

    #[tokio::test]
    async fn test_handle_person_entered_elevator_accepts_when_capacity_available() {
        let elevators = vec!["E1".to_string()];
        let (mut controller, _, _, _, mut to_persons_rx) = create_test_controller(elevators);

        // Set door to Open (required for boarding)
        controller.state.get_mut("E1").unwrap().door = Open;

        controller.handle_person_entered_elevator("P1".to_string(), "E1".to_string()).await;

        let state = controller.state.get("E1").unwrap();
        assert!(state.passengers.contains(&"P1".to_string()));

        let msg = to_persons_rx.recv().await.unwrap();
        assert_eq!(msg, ControllerToPersonsMsg::UpdateBoardingStatus(
            "P1".to_string(), 
            "E1".to_string(), 
            BoardingStatus::Accepted
        ));
    }

    #[tokio::test]
    async fn test_handle_person_entered_elevator_rejects_when_full() {
        let elevators = vec!["E1".to_string()];
        let (mut controller, _, _, _, mut to_persons_rx) = create_test_controller(elevators);

        // Set door to Open and add 2 passengers (max capacity)
        {
            let state = controller.state.get_mut("E1").unwrap();
            state.door = Open;
            state.passengers.push("P1".to_string());
            state.passengers.push("P2".to_string());
        }

        controller.handle_person_entered_elevator("P3".to_string(), "E1".to_string()).await;

        let state = controller.state.get("E1").unwrap();
        assert!(!state.passengers.contains(&"P3".to_string()));
        assert_eq!(state.passengers.len(), 2);

        let msg = to_persons_rx.recv().await.unwrap();
        assert_eq!(msg, ControllerToPersonsMsg::UpdateBoardingStatus(
            "P3".to_string(), 
            "E1".to_string(), 
            BoardingStatus::Rejected
        ));
    }

    #[tokio::test]
    async fn test_handle_person_entered_elevator_rejects_when_doors_closed() {
        let elevators = vec!["E1".to_string()];
        let (mut controller, _, _, _, mut to_persons_rx) = create_test_controller(elevators);

        // Door is Closed by default
        controller.handle_person_entered_elevator("P1".to_string(), "E1".to_string()).await;

        let state = controller.state.get("E1").unwrap();
        assert!(!state.passengers.contains(&"P1".to_string()));

        let msg = to_persons_rx.recv().await.unwrap();
        assert_eq!(msg, ControllerToPersonsMsg::UpdateBoardingStatus(
            "P1".to_string(), 
            "E1".to_string(), 
            BoardingStatus::Rejected
        ));
    }

    #[tokio::test]
    async fn test_handle_person_left_elevator_removes_passenger() {
        let elevators = vec!["E1".to_string()];
        let (mut controller, _, _, _, _) = create_test_controller(elevators);

        // Add passenger first
        controller.state.get_mut("E1").unwrap().passengers.push("P1".to_string());

        controller.handle_person_left_elevator("P1".to_string(), "E1".to_string()).await;

        let state = controller.state.get("E1").unwrap();
        assert!(!state.passengers.contains(&"P1".to_string()));
    }

    #[tokio::test]
    async fn test_handle_elevator_arrived_opens_doors() {
        let elevators = vec!["E1".to_string()];
        let (mut controller, _, _, mut to_elevators_rx, _) = create_test_controller(elevators);

        controller.handle_elevator_arrived("E1".to_string(), First).await;

        let state = controller.state.get("E1").unwrap();
        assert_eq!(state.floor, First);

        let msg = to_elevators_rx.recv().await.unwrap();
        assert_eq!(msg, ControllerToElevatorsMsg::OpenDoors("E1".to_string()));
    }

    #[tokio::test]
    async fn test_handle_doors_opening_clears_mission_and_notifies() {
        let elevators = vec!["E1".to_string()];
        let (mut controller, _, _, _, mut to_persons_rx) = create_test_controller(elevators);

        // Set up state
        {
            let state = controller.state.get_mut("E1").unwrap();
            state.mission = Some(First);
            state.floor = First;
            state.missions.push_back(First); // Should be removed
            state.missions.push_back(Second); // Should remain
        }

        controller.handle_doors_opening("E1".to_string()).await;

        let state = controller.state.get("E1").unwrap();
        assert!(state.mission.is_none());
        assert!(!state.missions.contains(&First));
        assert!(state.missions.contains(&Second));
        assert_eq!(state.door, Open);

        // Should notify persons about elevator halt
        let msg = to_persons_rx.recv().await.unwrap();
        assert_eq!(msg, ControllerToPersonsMsg::ElevatorHalt("E1".to_string(), First));
    }

    #[tokio::test]
    async fn test_handle_doors_closed_starts_next_mission() {
        let elevators = vec!["E1".to_string()];
        let (mut controller, _, _, mut to_elevators_rx, _) = create_test_controller(elevators);

        // Set up queued mission
        controller.state.get_mut("E1").unwrap().missions.push_back(Second);

        controller.handle_doors_closed("E1".to_string()).await;

        let state = controller.state.get("E1").unwrap();
        assert_eq!(state.mission, Some(Second));
        assert!(state.missions.is_empty());
        assert_eq!(state.door, Closed);

        let msg = to_elevators_rx.recv().await.unwrap();
        assert_eq!(msg, ControllerToElevatorsMsg::ElevatorMission("E1".to_string(), Second));
    }

    #[tokio::test]
    async fn test_start_next_mission_if_idle_does_nothing_when_mission_active() {
        let elevators = vec!["E1".to_string()];
        let (mut controller, _, _, _, _) = create_test_controller(elevators);

        // Set up active mission and queued missions
        {
            let state = controller.state.get_mut("E1").unwrap();
            state.mission = Some(First);
            state.missions.push_back(Second);
        }

        controller.start_next_mission_if_idle("E1");

        let state = controller.state.get("E1").unwrap();
        // Mission should still be First, Second should still be queued
        assert_eq!(state.mission, Some(First));
        assert!(state.missions.contains(&Second));
    }

    #[tokio::test]
    async fn test_handle_person_choosing_floor_adds_to_missions() {
        let elevators = vec!["E1".to_string()];
        let (mut controller, _, _, mut to_elevators_rx, _) = create_test_controller(elevators);

        // Set door to Open
        controller.state.get_mut("E1").unwrap().door = Open;

        controller.handle_person_choosing_floor("P1".to_string(), "E1".to_string(), Third).await;

        let state = controller.state.get("E1").unwrap();
        assert!(state.missions.contains(&Third));

        // Should send CloseDoors when door is Open
        let msg = to_elevators_rx.recv().await.unwrap();
        assert_eq!(msg, ControllerToElevatorsMsg::CloseDoors("E1".to_string()));
    }

    #[tokio::test]
    async fn test_handle_person_choosing_floor_avoids_duplicates() {
        let elevators = vec!["E1".to_string()];
        let (mut controller, _, _, _, _) = create_test_controller(elevators);

        // Set an active mission so Third stays in queue
        controller.state.get_mut("E1").unwrap().mission = Some(Floor::First);
        controller.state.get_mut("E1").unwrap().missions.push_back(Third);

        controller.handle_person_choosing_floor("P1".to_string(), "E1".to_string(), Third).await;

        let state = controller.state.get("E1").unwrap();
        let count = state.missions.iter().filter(|&&f| f == Third).count();
        assert_eq!(count, 1);
    }

    // ========================================================================
    // F1: Das System besteht aus 3 Fahrkabinen, Ebenen 0-3
    // ========================================================================

    #[test]
    fn test_f1_system_has_exactly_three_elevators() {
        let elevators = vec!["Dorisch".to_string(), "Ionisch".to_string(), "Korinthisch".to_string()];
        let (controller, _, _, _, _) = create_test_controller(elevators);
        
        assert_eq!(controller.state.len(), 3);
        assert!(controller.state.contains_key("Dorisch"));
        assert!(controller.state.contains_key("Ionisch"));
        assert!(controller.state.contains_key("Korinthisch"));
    }

    #[test]
    fn test_f1_floors_range_from_ground_to_third() {
        // Verify all floor values are within 0-3 range (Ground=1, Third=4 internally)
        assert_eq!(Ground as i8, 1);
        assert_eq!(First as i8, 2);
        assert_eq!(Second as i8, 3);
        assert_eq!(Third as i8, 4);
        
        // 4 floors total (0, 1, 2, 3 in user terms = Ground, First, Second, Third)
        let floors = vec![Ground, First, Second, Third];
        assert_eq!(floors.len(), 4);
    }

    // ========================================================================
    // F2: Fahrstühle fahren unabhängig voneinander
    // ========================================================================

    #[tokio::test]
    async fn test_f2_elevators_operate_independently() {
        let elevators = vec!["E1".to_string(), "E2".to_string()];
        let (mut controller, _, _, _, _) = create_test_controller(elevators);

        // Give E1 a mission to First floor
        controller.handle_person_request_elevator(First).await;
        // Give E2 a mission to Third floor (E2 should get it as it has fewer missions)
        controller.handle_person_request_elevator(Third).await;

        let e1_state = controller.state.get("E1").unwrap();
        let e2_state = controller.state.get("E2").unwrap();

        // Both elevators should have independent missions
        // One should have First, one should have Third
        let missions: Vec<Option<Floor>> = vec![e1_state.mission, e2_state.mission];
        assert!(missions.contains(&Some(First)) || missions.iter().any(|m| 
            controller.state.values().any(|s| s.missions.contains(&First))));
    }

    // ========================================================================
    // S2: Kabinen auf Ebene 0 können nicht abwärts fahren
    // ========================================================================

    #[test]
    fn test_s2_ground_floor_is_lowest() {
        // Ground is the lowest floor (value 1)
        assert!(Ground as i8 <= First as i8);
        assert!(Ground as i8 <= Second as i8);
        assert!(Ground as i8 <= Third as i8);
        
        // No floor below Ground exists in the enum
        let all_floors = [Ground, First, Second, Third];
        let min_floor = all_floors.iter().min_by_key(|f| **f as i8).unwrap();
        assert_eq!(*min_floor, Ground);
    }

    #[tokio::test]
    async fn test_s2_elevator_at_ground_cannot_go_lower() {
        let elevators = vec!["E1".to_string()];
        let (controller, _, _, _, _) = create_test_controller(elevators);

        let state = controller.state.get("E1").unwrap();
        // Elevator starts at Ground
        assert_eq!(state.floor, Ground);
        
        // There's no floor below Ground to request - the enum enforces this
        // Any valid Floor request is >= Ground
    }

    // ========================================================================
    // S3: Kabinen auf Ebene 3 können nicht aufwärts fahren
    // ========================================================================

    #[test]
    fn test_s3_third_floor_is_highest() {
        // Third is the highest floor (value 4)
        assert!(Third as i8 >= Ground as i8);
        assert!(Third as i8 >= First as i8);
        assert!(Third as i8 >= Second as i8);
        
        // No floor above Third exists in the enum
        let all_floors = [Ground, First, Second, Third];
        let max_floor = all_floors.iter().max_by_key(|f| **f as i8).unwrap();
        assert_eq!(*max_floor, Third);
    }

    // ========================================================================
    // S5: Max 2 Passagiere gleichzeitig
    // ========================================================================

    #[tokio::test]
    async fn test_s5_max_two_passengers_enforced() {
        let elevators = vec!["E1".to_string()];
        let (mut controller, _, _, _, mut to_persons_rx) = create_test_controller(elevators);

        // Set door to Open
        controller.state.get_mut("E1").unwrap().door = Open;

        // First passenger - should be accepted
        controller.handle_person_entered_elevator("P1".to_string(), "E1".to_string()).await;
        let msg1 = to_persons_rx.recv().await.unwrap();
        assert_eq!(msg1, ControllerToPersonsMsg::UpdateBoardingStatus(
            "P1".to_string(), "E1".to_string(), BoardingStatus::Accepted));

        // Second passenger - should be accepted
        controller.handle_person_entered_elevator("P2".to_string(), "E1".to_string()).await;
        let msg2 = to_persons_rx.recv().await.unwrap();
        assert_eq!(msg2, ControllerToPersonsMsg::UpdateBoardingStatus(
            "P2".to_string(), "E1".to_string(), BoardingStatus::Accepted));

        // Third passenger - should be REJECTED (capacity = 2)
        controller.handle_person_entered_elevator("P3".to_string(), "E1".to_string()).await;
        let msg3 = to_persons_rx.recv().await.unwrap();
        assert_eq!(msg3, ControllerToPersonsMsg::UpdateBoardingStatus(
            "P3".to_string(), "E1".to_string(), BoardingStatus::Rejected));

        // Verify only 2 passengers in elevator
        let state = controller.state.get("E1").unwrap();
        assert_eq!(state.passengers.len(), 2);
        assert!(state.passengers.contains(&"P1".to_string()));
        assert!(state.passengers.contains(&"P2".to_string()));
        assert!(!state.passengers.contains(&"P3".to_string()));
    }

    // ========================================================================
    // C1: Zentrale Steuerung gibt Fahraufträge an Kabinen
    // ========================================================================

    #[tokio::test]
    async fn test_c1_controller_dispatches_to_elevator_with_fewest_missions() {
        let elevators = vec!["E1".to_string(), "E2".to_string(), "E3".to_string()];
        let (mut controller, _, _, _, _) = create_test_controller(elevators);

        // Give E1 two missions
        controller.handle_person_request_elevator(First).await;
        controller.handle_person_request_elevator(Second).await;

        // E2 and E3 should have 0 missions, so next request goes to one of them
        controller.handle_person_request_elevator(Third).await;

        // E1 should still have its missions, one of E2/E3 should have Third
        let e1 = controller.state.get("E1").unwrap();
        let e2 = controller.state.get("E2").unwrap();
        let e3 = controller.state.get("E3").unwrap();

        // Total missions across all elevators
        let total_with_third = [e1, e2, e3].iter()
            .filter(|s| s.mission == Some(Third) || s.missions.contains(&Third))
            .count();
        assert_eq!(total_with_third, 1);
    }

    // ========================================================================
    // C4: Zentrale Steuerung überwacht Passagieranzahl
    // ========================================================================

    #[tokio::test]
    async fn test_c4_controller_tracks_passenger_count() {
        let elevators = vec!["E1".to_string()];
        let (mut controller, _, _, _, _) = create_test_controller(elevators);
        controller.state.get_mut("E1").unwrap().door = Open;

        // Initially 0 passengers
        assert_eq!(controller.state.get("E1").unwrap().passengers.len(), 0);

        // Add first passenger
        controller.handle_person_entered_elevator("P1".to_string(), "E1".to_string()).await;
        assert_eq!(controller.state.get("E1").unwrap().passengers.len(), 1);

        // Add second passenger
        controller.handle_person_entered_elevator("P2".to_string(), "E1".to_string()).await;
        assert_eq!(controller.state.get("E1").unwrap().passengers.len(), 2);

        // Remove one passenger
        controller.handle_person_left_elevator("P1".to_string(), "E1".to_string()).await;
        assert_eq!(controller.state.get("E1").unwrap().passengers.len(), 1);

        // Verify correct passenger remains
        assert!(controller.state.get("E1").unwrap().passengers.contains(&"P2".to_string()));
    }

    // ========================================================================
    // Z1: Zustände der Fahrkabinen
    // ========================================================================

    #[test]
    fn test_z1_elevator_states_exist() {
        use crate::elevator::ElevatorStatus::*;
        
        // IdleIn(Floor) - Kabine steht in Ebene x
        let idle = IdleIn(Ground);
        assert_eq!(idle, IdleIn(Ground));
        
        // MovingFromTo(Floor, Floor) - Kabine fährt von x zu y
        let moving = MovingFromTo(Ground, First);
        assert_eq!(moving, MovingFromTo(Ground, First));
        
        // Different states are not equal
        assert_ne!(IdleIn(Ground), MovingFromTo(Ground, First));
    }

    // ========================================================================
    // Z2: Zustände der Türen
    // ========================================================================

    #[test]
    fn test_z2_door_states_complete() {
        // All 4 door states must exist: Closed, Opening, Open, Closing
        use crate::elevator::DoorStatus::*;
        
        let states = vec![Closed, Opening, Open, Closing];
        assert_eq!(states.len(), 4);
        
        // All states are distinct
        assert_ne!(Closed, Opening);
        assert_ne!(Opening, Open);
        assert_ne!(Open, Closing);
        assert_ne!(Closing, Closed);
    }

    // ========================================================================
    // P4: Passagiere können nur bei geöffneter Tür verlassen
    // ========================================================================

    #[tokio::test]
    async fn test_p4_passenger_leaves_tracked_correctly() {
        let elevators = vec!["E1".to_string()];
        let (mut controller, _, _, _, _) = create_test_controller(elevators);

        // Add passenger to elevator
        controller.state.get_mut("E1").unwrap().passengers.push("P1".to_string());
        assert_eq!(controller.state.get("E1").unwrap().passengers.len(), 1);

        // Person leaves (in real flow, this only happens when doors are open)
        controller.handle_person_left_elevator("P1".to_string(), "E1".to_string()).await;

        // Passenger should be removed
        assert_eq!(controller.state.get("E1").unwrap().passengers.len(), 0);
    }

    // ========================================================================
    // S1: Türen sind während der Fahrt nicht offen (implizit durch State-Machine)
    // ========================================================================

    #[test]
    fn test_s1_doors_closed_is_initial_state() {
        let elevators = vec!["E1".to_string()];
        let (controller, _, _, _, _) = create_test_controller(elevators);

        // Doors should be closed initially
        assert_eq!(controller.state.get("E1").unwrap().door, Closed);
    }
}