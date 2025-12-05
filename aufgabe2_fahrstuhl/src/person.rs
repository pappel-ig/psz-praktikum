use std::fmt::{Debug, Display, Formatter};
use log::{debug, error, info, trace};
use rand::rng;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use task::JoinHandle;
use tokio::sync::broadcast::Receiver;
use tokio::sync::mpsc::Sender;
use tokio::{spawn, task};
use BoardingStatus::{Accepted, Rejected};
use ControllerToPersonsMsg::{UpdateBoardingStatus};
use PersonMsg::StatusUpdate;
use PersonStatus::{Done, Entering, Idle, InElevator};
use PersonToControllerMsg::{PersonChoosingFloor, PersonEnteredElevator, PersonEnteringElevator};
use crate::controller::{BoardingStatus, Floor};
use crate::controller::Floor::{First, Ground, Second, Third};
use crate::mqtt::PersonMsg;
use crate::mqtt::PersonMsg::{Boarding, Request};
use crate::mqtt::Send::{PersonTopic};
use crate::msg::{ControllerToPersonsMsg, PersonToControllerMsg};
use crate::msg::ControllerToPersonsMsg::{ElevatorHalt};
use crate::msg::PersonToControllerMsg::{PersonLeavingElevator, PersonLeftElevator, PersonRequestElevator};
use crate::person::PersonStatus::Leaving;
use crate::utils::{delay, random_delay_ms};

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum PersonStatus {
    Idle,
    Entering,
    Choosing,
    InElevator,
    Leaving,
    Done,
}

pub struct Person {
    pub id: String,
    from_controller: Receiver<ControllerToPersonsMsg>,
    to_controller: Sender<PersonToControllerMsg>,
    to_mqtt: Sender<crate::mqtt::Send>,
    state: PersonState
}

#[derive(Debug)]
struct PersonState {
    status: PersonStatus,
    current_floor: Floor,
    destination_floor: Floor,
    elevator: Option<String>,
}

impl Display for PersonState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{ status: {:?}, floor: {:?}, dest: {:?}, elevator: {:?} }}", self.status, self.current_floor, self.destination_floor, self.elevator)
    }
}

impl Display for Person {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{ id: {:?}, {:?} }}", self.id, self.state)
    }
}

impl Debug for Person {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self, f)
    }
}

impl Person {
    pub fn new(id: &str,
               from_controller_to_persons: Receiver<ControllerToPersonsMsg>,
               from_person_to_controller: Sender<PersonToControllerMsg>,
               to_mqtt: Sender<crate::mqtt::Send>) -> Self {
        let (current_floor, destination_floor) = Self::pick_two_distinct_floors();
        Person {
            id: id.to_string(),
            from_controller: from_controller_to_persons,
            to_controller: from_person_to_controller,
            to_mqtt,
            state: PersonState {
                status: Idle,
                current_floor,
                destination_floor,
                elevator: None
            }
        }
    }

    pub fn with(id: &str,
                from_controller_to_persons: Receiver<ControllerToPersonsMsg>,
                from_person_to_controller: Sender<PersonToControllerMsg>,
                to_mqtt: Sender<crate::mqtt::Send>,
                current_floor: Floor,
                destination_floor: Floor) -> Self {
        Person {
            id: id.to_string(),
            from_controller: from_controller_to_persons,
            to_controller: from_person_to_controller,
            to_mqtt,
            state: PersonState {
                status: Idle,
                current_floor,
                destination_floor,
                elevator: None
            }
        }
    }

    pub fn init(mut self) -> JoinHandle<()> {
        tokio::spawn(async move {
            self.request_elevator().await;
            loop {
                match self.from_controller.recv().await {
                    Ok(msg) => {
                        info!("{:?}", msg);
                        match msg {
                            ElevatorHalt(elevator, floor) => {
                                self.handle_elevator_halt(elevator.clone(), floor.clone()).await
                            }
                            UpdateBoardingStatus(person, elevator, boarding_status) => {
                                if self.id.eq(&person) {
                                    debug!("UpdateBoardingStatus(person={}, elevator={}, boarding_status={})", person, elevator, boarding_status);
                                }
                                self.handle_update_boarding_status(person, elevator, boarding_status).await
                            }
                        }
                        info!("{:?}", self);
                    }
                    Err(err) => {
                        error!("Error in Channel: {}", err);
                    }
                }
            }
        })
    }

    // Handlers

    pub async fn request_elevator(&mut self) {
        let _ = self.to_controller.send(PersonRequestElevator(self.state.current_floor)).await;
        Person::request(self.to_mqtt.clone(), self.id.clone(), self.state.current_floor);
    }

    async fn handle_elevator_halt(&mut self, elevator: String, floor: Floor) {
        if self.state.current_floor.eq(&floor) && self.state.status.eq(&Idle) {
            self.state.status = Entering;
            Person::status(self.to_mqtt.clone(), self.id.clone(), self.state.status.clone());
            let _ = self.to_controller.send(PersonEnteringElevator(self.id.clone(), elevator.clone())).await;
            random_delay_ms(200, 1000).await;
            let _ = self.to_controller.send(PersonEnteredElevator(self.id.clone(), elevator.clone())).await;
        }
        if self.state.destination_floor.eq(&floor) && self.state.status.eq(&InElevator) && self.state.elevator.eq(&Some(elevator.clone())) {
            self.leave_elevator(self.id.clone(), elevator).await;
            self.state.status = Done;
            Person::status(self.to_mqtt.clone(), self.id.clone(), self.state.status.clone());
            println!("PersonFinished(id={})", self.id);
        }
    }

    async fn handle_update_boarding_status(&mut self, person: String, elevator: String, boarding_status: BoardingStatus) {
        if self.id.eq(&person) {
            Person::boarding(self.to_mqtt.clone(), person.clone(), boarding_status.clone());
            match boarding_status {
                Accepted => {
                    self.state.status = InElevator;
                    Person::status(self.to_mqtt.clone(), person.clone(), self.state.status.clone());
                    self.state.elevator = Some(elevator.clone());
                    let _ = self.to_controller.send(PersonChoosingFloor(self.id.clone(), elevator, self.state.destination_floor)).await;
                }
                Rejected => {
                    self.leave_elevator(person, elevator).await;
                    random_delay_ms(200, 1000).await;
                    let _ = self.to_controller.send(PersonRequestElevator(self.state.current_floor)).await;
                }
            }
        }
    }

    async fn leave_elevator(&mut self, person: String, elevator: String) {
        self.state.elevator = None;
        self.state.status = Leaving;
        Person::status(self.to_mqtt.clone(), person.clone(), self.state.status.clone());
        let _ = self.to_controller.send(PersonLeavingElevator(person.clone(), elevator.clone())).await;
        random_delay_ms(200, 1000).await;
        self.state.status = Idle;
        Person::status(self.to_mqtt.clone(), person.clone(), self.state.status.clone());
        let _ = self.to_controller.send(PersonLeftElevator(person, elevator.clone())).await;
    }

    // MQTT-Updates

    fn status(to_mqtt: Sender<crate::mqtt::Send>, id: String, status: PersonStatus) {
        spawn(async move{
            let msg = PersonTopic {
                id,
                msg: StatusUpdate {
                    status,
                },
            };
            let _ = to_mqtt.send(msg).await;
        });

    }

    fn boarding(to_mqtt: Sender<crate::mqtt::Send>, id: String, status: BoardingStatus) {
        spawn(async move{
            let msg = PersonTopic {
                id,
                msg: Boarding {
                    status: status.clone(),
                },
            };
            let _ = to_mqtt.send(msg).await;
        });

    }

    fn request(to_mqtt: Sender<crate::mqtt::Send>, id: String, floor: Floor) {
        spawn(async move{
            let msg = PersonTopic {
                id,
                msg: Request {
                    floor,
                },
            };
            let _ = to_mqtt.send(msg).await;
        });
    }

    // Other Methods

    fn pick_two_distinct_floors() -> (Floor, Floor) {
        let mut rng = rng();
        let mut floors = vec![Ground, First, Second, Third];
        floors.shuffle(&mut rng);
        (floors[0], floors[1])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::{broadcast, mpsc};
    use crate::controller::Floor::*;
    use std::collections::HashSet;

    fn create_test_person(id: &str, current: Floor, destination: Floor) -> (
        Person,
        broadcast::Sender<ControllerToPersonsMsg>,
        mpsc::Receiver<PersonToControllerMsg>,
        mpsc::Receiver<crate::mqtt::Send>,
    ) {
        let (to_person_tx, to_person_rx) = broadcast::channel(100);
        let (from_person_tx, from_person_rx) = mpsc::channel(100);
        let (mqtt_tx, mqtt_rx) = mpsc::channel(100);

        let person = Person::with(
            id,
            to_person_rx,
            from_person_tx,
            mqtt_tx,
            current,
            destination,
        );

        (person, to_person_tx, from_person_rx, mqtt_rx)
    }

    #[test]
    fn test_person_status_variants() {
        assert_eq!(Idle, Idle);
        assert_ne!(Idle, Entering);
        assert_ne!(Entering, InElevator);
        assert_ne!(InElevator, Leaving);
        assert_ne!(Leaving, Done);
    }

    #[test]
    fn test_pick_two_distinct_floors_returns_different_floors() {
        // Run multiple times to ensure randomness works correctly
        for _ in 0..100 {
            let (from, to) = Person::pick_two_distinct_floors();
            assert_ne!(from, to, "Current and destination floors should be different");
        }
    }

    #[test]
    fn test_pick_two_distinct_floors_returns_valid_floors() {
        let valid_floors: HashSet<Floor> = vec![Ground, First, Second, Third].into_iter().collect();
        
        for _ in 0..50 {
            let (from, to) = Person::pick_two_distinct_floors();
            assert!(valid_floors.contains(&from), "From floor should be valid");
            assert!(valid_floors.contains(&to), "To floor should be valid");
        }
    }

    #[test]
    fn test_person_with_constructor() {
        let (to_person_tx, to_person_rx) = broadcast::channel(100);
        let (from_person_tx, _from_person_rx) = mpsc::channel(100);
        let (mqtt_tx, _mqtt_rx) = mpsc::channel(100);

        let person = Person::with(
            "P1",
            to_person_rx,
            from_person_tx,
            mqtt_tx,
            Ground,
            Third,
        );

        assert_eq!(person.id, "P1");
        assert_eq!(person.state.current_floor, Ground);
        assert_eq!(person.state.destination_floor, Third);
        assert_eq!(person.state.status, Idle);
        assert!(person.state.elevator.is_none());
    }

    #[test]
    fn test_person_new_constructor() {
        let (to_person_tx, to_person_rx) = broadcast::channel(100);
        let (from_person_tx, _from_person_rx) = mpsc::channel(100);
        let (mqtt_tx, _mqtt_rx) = mpsc::channel(100);

        let person = Person::new(
            "P2",
            to_person_rx,
            from_person_tx,
            mqtt_tx,
        );

        assert_eq!(person.id, "P2");
        assert_eq!(person.state.status, Idle);
        assert!(person.state.elevator.is_none());
        // Floors should be randomly selected but different
        assert_ne!(person.state.current_floor, person.state.destination_floor);
    }

    #[tokio::test]
    async fn test_request_elevator_sends_message() {
        let (mut person, _, mut from_person_rx, _) = create_test_person("P1", Ground, First);

        person.request_elevator().await;

        let msg = from_person_rx.recv().await.unwrap();
        assert_eq!(msg, PersonToControllerMsg::PersonRequestElevator(Ground));
    }

    #[tokio::test]
    async fn test_handle_elevator_halt_when_on_same_floor_and_idle() {
        let (mut person, _, mut from_person_rx, _) = create_test_person("P1", Ground, First);

        // Person is Idle and on Ground floor
        person.handle_elevator_halt("E1".to_string(), Ground).await;

        // Status should change to Entering
        assert_eq!(person.state.status, Entering);

        // Should send PersonEnteringElevator message
        let msg = from_person_rx.recv().await.unwrap();
        match msg {
            PersonToControllerMsg::PersonEnteringElevator(person_id, elevator_id) => {
                assert_eq!(person_id, "P1");
                assert_eq!(elevator_id, "E1");
            }
            _ => panic!("Expected PersonEnteringElevator message"),
        }
    }

    #[tokio::test]
    async fn test_handle_elevator_halt_ignores_different_floor() {
        let (mut person, _, mut from_person_rx, _) = create_test_person("P1", Ground, First);

        // Elevator stops at different floor
        person.handle_elevator_halt("E1".to_string(), Second).await;

        // Status should remain Idle
        assert_eq!(person.state.status, Idle);

        // No message should be sent (channel should be empty after timeout)
        tokio::select! {
            _ = from_person_rx.recv() => panic!("Should not receive message"),
            _ = tokio::time::sleep(std::time::Duration::from_millis(10)) => {}
        }
    }

    #[tokio::test]
    async fn test_handle_elevator_halt_ignores_when_not_idle() {
        let (mut person, _, mut from_person_rx, _) = create_test_person("P1", Ground, First);
        person.state.status = InElevator;

        person.handle_elevator_halt("E1".to_string(), Ground).await;

        // Status should remain InElevator
        assert_eq!(person.state.status, InElevator);

        tokio::select! {
            _ = from_person_rx.recv() => panic!("Should not receive message"),
            _ = tokio::time::sleep(std::time::Duration::from_millis(10)) => {}
        }
    }

    #[tokio::test]
    async fn test_handle_update_boarding_status_accepted() {
        let (mut person, _, mut from_person_rx, _) = create_test_person("P1", Ground, Third);
        person.state.status = Entering;

        person.handle_update_boarding_status("P1".to_string(), "E1".to_string(), BoardingStatus::Accepted).await;

        assert_eq!(person.state.status, InElevator);
        assert_eq!(person.state.elevator, Some("E1".to_string()));

        // Should send PersonChoosingFloor
        let msg = from_person_rx.recv().await.unwrap();
        match msg {
            PersonToControllerMsg::PersonChoosingFloor(person_id, elevator_id, floor) => {
                assert_eq!(person_id, "P1");
                assert_eq!(elevator_id, "E1");
                assert_eq!(floor, Third);
            }
            _ => panic!("Expected PersonChoosingFloor message"),
        }
    }

    #[tokio::test]
    async fn test_handle_update_boarding_status_ignored_for_other_person() {
        let (mut person, _, mut from_person_rx, _) = create_test_person("P1", Ground, First);
        person.state.status = Entering;

        // Message for different person
        person.handle_update_boarding_status("P2".to_string(), "E1".to_string(), BoardingStatus::Accepted).await;

        // Status should remain Entering
        assert_eq!(person.state.status, Entering);
        assert!(person.state.elevator.is_none());

        tokio::select! {
            _ = from_person_rx.recv() => panic!("Should not receive message"),
            _ = tokio::time::sleep(std::time::Duration::from_millis(10)) => {}
        }
    }

    #[test]
    fn test_person_state_display() {
        let state = PersonState {
            status: Idle,
            current_floor: Ground,
            destination_floor: Third,
            elevator: None,
        };

        let display = format!("{}", state);
        assert!(display.contains("Idle"));
        assert!(display.contains("Ground"));
        assert!(display.contains("Third"));
    }

    // ========================================================================
    // Z3: Zustände eines Passagiers
    // ========================================================================

    #[test]
    fn test_z3_all_person_states_exist() {
        // All required states from Z3:
        // - idle auf Ebene x
        // - betritt Fahrkabine
        // - wählt Zielebene (Choosing)
        // - ist in Fahrkabine (InElevator)
        // - verlässt Fahrkabine (Leaving)
        // - Done (completed)
        
        let states = vec![Idle, Entering, PersonStatus::Choosing, InElevator, Leaving, Done];
        assert_eq!(states.len(), 6);
        
        // All states are distinct
        assert_ne!(Idle, Entering);
        assert_ne!(Entering, InElevator);
        assert_ne!(InElevator, Leaving);
        assert_ne!(Leaving, Done);
    }

    #[test]
    fn test_z3_person_starts_idle_on_floor() {
        let (_to_person_tx, to_person_rx) = broadcast::channel(100);
        let (from_person_tx, _from_person_rx) = mpsc::channel(100);
        let (mqtt_tx, _mqtt_rx) = mpsc::channel(100);

        let person = Person::with(
            "P1",
            to_person_rx,
            from_person_tx,
            mqtt_tx,
            First,  // current floor
            Third,  // destination
        );

        // Person starts Idle on their current floor
        assert_eq!(person.state.status, Idle);
        assert_eq!(person.state.current_floor, First);
    }

    // ========================================================================
    // P1: Passagiere werden auf Ebenen erzeugt mit zufälligem Ziel
    // ========================================================================

    #[test]
    fn test_p1_person_created_with_random_floors() {
        // Run multiple times to verify randomness
        let mut seen_combinations = std::collections::HashSet::new();
        
        for _ in 0..20 {
            let (from, to) = Person::pick_two_distinct_floors();
            seen_combinations.insert((from, to));
            
            // Current and destination must be different
            assert_ne!(from, to);
        }
        
        // With 20 attempts, we should see multiple different combinations
        assert!(seen_combinations.len() > 1, "Should have random variety");
    }

    // ========================================================================
    // P3: Passagiere wählen zufällig Zielebene
    // ========================================================================

    #[tokio::test]
    async fn test_p3_person_chooses_destination_floor() {
        let (mut person, _, mut from_person_rx, _) = create_test_person("P1", Ground, Third);
        person.state.status = Entering;

        // Simulate acceptance - person will choose their destination floor
        person.handle_update_boarding_status("P1".to_string(), "E1".to_string(), BoardingStatus::Accepted).await;

        // Person should send PersonChoosingFloor with their destination
        let msg = from_person_rx.recv().await.unwrap();
        match msg {
            PersonToControllerMsg::PersonChoosingFloor(_, _, floor) => {
                assert_eq!(floor, Third); // The destination they were created with
            }
            _ => panic!("Expected PersonChoosingFloor message"),
        }
    }

    // ========================================================================
    // State transitions
    // ========================================================================

    #[tokio::test]
    async fn test_person_state_transition_idle_to_entering() {
        let (mut person, _, _, _) = create_test_person("P1", Ground, First);
        
        assert_eq!(person.state.status, Idle);
        
        // Elevator arrives at person's floor
        person.handle_elevator_halt("E1".to_string(), Ground).await;
        
        assert_eq!(person.state.status, Entering);
    }

    #[tokio::test]
    async fn test_person_state_transition_entering_to_in_elevator() {
        let (mut person, _, _, _) = create_test_person("P1", Ground, First);
        person.state.status = Entering;
        
        // Boarding accepted
        person.handle_update_boarding_status("P1".to_string(), "E1".to_string(), BoardingStatus::Accepted).await;
        
        assert_eq!(person.state.status, InElevator);
        assert_eq!(person.state.elevator, Some("E1".to_string()));
    }

    // ========================================================================
    // P2: Passagiere betreten bei offen/öffnend/schließend (handled by controller)
    // ========================================================================

    #[tokio::test]
    async fn test_p2_person_attempts_to_enter_when_elevator_halts() {
        let (mut person, _, mut from_person_rx, _) = create_test_person("P1", Ground, First);

        // Elevator halts at person's floor
        person.handle_elevator_halt("E1".to_string(), Ground).await;

        // Person should try to enter (sends PersonEnteringElevator)
        let msg = from_person_rx.recv().await.unwrap();
        assert!(matches!(msg, PersonToControllerMsg::PersonEnteringElevator(_, _)));
    }
}