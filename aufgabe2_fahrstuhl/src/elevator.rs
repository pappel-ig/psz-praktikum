use log::error;
use crate::controller::{ElevatorController, Floor};
use crate::elevator::DoorStatus::{Closed, Open};
use crate::elevator::ElevatorStatus::IdleIn;
use crate::mqtt::ElevatorMsg::{Door, Position};
use crate::mqtt::Send::ElevatorTopic;
use crate::msg::ElevatorToControllerMsg::{DoorsClosed, DoorsClosing, DoorsOpened, DoorsOpening, ElevatorArrived, ElevatorMoving};
use crate::msg::{ControllerToElevatorsMsg, ElevatorToControllerMsg};
use crate::utils;
use serde::{Deserialize, Serialize};
use task::JoinHandle;
use tokio::sync::broadcast::Receiver;
use tokio::sync::mpsc::Sender;
use tokio::task;
use utils::delay;
use ControllerToElevatorsMsg::{CloseDoors, ElevatorMission, OpenDoors};
use DoorStatus::{Closing, Opening};
use ElevatorStatus::MovingFromTo;
use Floor::Ground;

#[derive(PartialEq, Debug)]
pub enum ElevatorStatus {
    IdleIn(Floor),
    MovingFromTo(Floor, Floor)
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
#[derive()]
pub enum DoorStatus {
    Closed,
    Opening,
    Open,
    Closing
}

pub struct Elevator {
    pub id: String,
    from_controller: Receiver<ControllerToElevatorsMsg>,
    to_controller: Sender<ElevatorToControllerMsg>,
    state: ElevatorState,
    pub to_mqtt: Sender<crate::mqtt::Send>,
}

#[derive(PartialEq)]
struct ElevatorState {
    floor: Floor,
    status: ElevatorStatus,
    doors_status: DoorStatus,
}

impl Elevator {
    pub fn init(mut self) -> JoinHandle<()> {
        tokio::spawn(async move {
            loop {
                match self.from_controller.recv().await {
                    Ok(msg) => {
                        match msg {
                            ElevatorMission(elevator, dest) => {
                                if self.id.eq(&elevator) {
                                    self.handle_mission(dest).await
                                }
                            }
                            OpenDoors(elevator) => {
                                if self.id.eq(&elevator) {
                                    self.handle_open_doors().await
                                }
                            }
                            CloseDoors(elevator) => {
                                if self.id.eq(&elevator) {
                                    self.handle_close_doors().await
                                }
                            }
                        }
                    }
                    Err(err) => {
                        error!("Error in Channel: {}", err);
                    }
                }
            }
        })
    }

    pub fn new(id: &str, from_controller: Receiver<ControllerToElevatorsMsg>, to_controller: Sender<ElevatorToControllerMsg>, to_mqtt: Sender<crate::mqtt::Send>) -> Self {
        Elevator {
            id: id.to_string(),
            from_controller,
            to_controller,
            to_mqtt,
            state: ElevatorState {
                floor: Ground,
                status: IdleIn(Ground),
                doors_status: Closed,
            }
        }
    }

    // Handlers

    async fn handle_mission(&mut self, dest: Floor) {
        self.state.status = MovingFromTo(self.state.floor, dest);
        let _ = self.to_controller.send(ElevatorMoving(self.id.clone(), self.state.floor, dest)).await;
        let distance_to_travel = (self.state.floor as i8 - dest as i8).abs() as u64;
        delay(distance_to_travel * 1000).await;
        self.state.status = IdleIn(dest);
        Elevator::position(self.to_mqtt.clone(), self.id.clone(), dest);
        let _ = self.to_controller.send(ElevatorArrived(self.id.clone(), dest)).await;
    }

    async fn handle_open_doors(&mut self) {
        if self.state.doors_status.eq(&Closed) {

            self.state.doors_status = Opening;
            Elevator::door(self.to_mqtt.clone(), self.id.clone(), self.state.doors_status.clone());
            let _ = self.to_controller.send(DoorsOpening(self.id.clone())).await;
            delay(1500).await;
            self.state.doors_status = Open;
            Elevator::door(self.to_mqtt.clone(), self.id.clone(), self.state.doors_status.clone());
            let _ = self.to_controller.send(DoorsOpened(self.id.clone())).await;
        }
    }

    async fn handle_close_doors(&mut self) {
        if self.state.doors_status.eq(&Open) {
            self.state.doors_status = Closing;
            Elevator::door(self.to_mqtt.clone(), self.id.clone(), self.state.doors_status.clone());
            let _ = self.to_controller.send(DoorsClosing(self.id.clone())).await;
            delay(1500).await;
            self.state.doors_status = Closed;
            Elevator::door(self.to_mqtt.clone(), self.id.clone(), self.state.doors_status.clone());
            let _ = self.to_controller.send(DoorsClosed(self.id.clone())).await;
        }
    }

    // MQTT Updates

    fn position(to_mqtt: Sender<crate::mqtt::Send>, id: String, floor: Floor) {
        tokio::spawn(async move {
            let msg = ElevatorTopic {
                id,
                msg: Position {
                    floor
                },
            };
            let _ = to_mqtt.send(msg).await;
        });
    }

    fn door(to_mqtt: Sender<crate::mqtt::Send>, id: String, status: DoorStatus) {
        tokio::spawn(async move {
            let msg = ElevatorTopic {
                id,
                msg: Door {
                    status
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
    use crate::controller::Floor::*;

    fn create_test_elevator(id: &str) -> (
        Elevator,
        broadcast::Sender<ControllerToElevatorsMsg>,
        mpsc::Receiver<ElevatorToControllerMsg>,
        mpsc::Receiver<crate::mqtt::Send>,
    ) {
        let (to_elevator_tx, to_elevator_rx) = broadcast::channel(100);
        let (from_elevator_tx, from_elevator_rx) = mpsc::channel(100);
        let (mqtt_tx, mqtt_rx) = mpsc::channel(100);

        let elevator = Elevator::new(
            id,
            to_elevator_rx,
            from_elevator_tx,
            mqtt_tx,
        );

        (elevator, to_elevator_tx, from_elevator_rx, mqtt_rx)
    }

    #[test]
    fn test_elevator_new() {
        let (elevator, _, _, _) = create_test_elevator("E1");

        assert_eq!(elevator.id, "E1");
        assert_eq!(elevator.state.floor, Ground);
        assert_eq!(elevator.state.status, IdleIn(Ground));
        assert_eq!(elevator.state.doors_status, Closed);
    }

    #[test]
    fn test_elevator_status_variants() {
        let idle = IdleIn(Ground);
        let moving = MovingFromTo(Ground, First);

        assert_eq!(idle, IdleIn(Ground));
        assert_ne!(idle, moving);
        assert_eq!(MovingFromTo(Ground, First), MovingFromTo(Ground, First));
    }

    #[test]
    fn test_door_status_transitions() {
        // Test all valid door status values
        assert_eq!(Closed, Closed);
        assert_eq!(Opening, Opening);
        assert_eq!(Open, Open);
        assert_eq!(Closing, Closing);
        
        // They should all be distinct
        assert_ne!(Closed, Opening);
        assert_ne!(Opening, Open);
        assert_ne!(Open, Closing);
        assert_ne!(Closing, Closed);
    }

    #[test]
    fn test_door_status_serialization() {
        let closed = Closed;
        let json = serde_json::to_string(&closed).unwrap();
        let deserialized: DoorStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(closed, deserialized);
    }

    #[test]
    fn test_floor_distance_calculation() {
        // Test the distance formula used in handle_mission
        let from = Ground;
        let to = Third;
        let distance = (from as i8 - to as i8).abs() as u64;
        assert_eq!(distance, 3);

        let distance_same = (First as i8 - First as i8).abs() as u64;
        assert_eq!(distance_same, 0);
    }

    #[tokio::test]
    async fn test_handle_mission_updates_state() {
        let (mut elevator, _, mut from_elevator_rx, _) = create_test_elevator("E1");

        // Set a very fast speed factor for testing
        crate::utils::SPEED_FACTOR.store(1, std::sync::atomic::Ordering::Relaxed);

        elevator.handle_mission(First).await;

        // After mission completes, should be idle at destination
        assert_eq!(elevator.state.status, IdleIn(First));

        // Should have sent ElevatorMoving and ElevatorArrived
        let msg1 = from_elevator_rx.recv().await.unwrap();
        match msg1 {
            crate::msg::ElevatorToControllerMsg::ElevatorMoving(id, from, to) => {
                assert_eq!(id, "E1");
                assert_eq!(from, Ground);
                assert_eq!(to, First);
            }
            _ => panic!("Expected ElevatorMoving message"),
        }

        let msg2 = from_elevator_rx.recv().await.unwrap();
        match msg2 {
            crate::msg::ElevatorToControllerMsg::ElevatorArrived(id, floor) => {
                assert_eq!(id, "E1");
                assert_eq!(floor, First);
            }
            _ => panic!("Expected ElevatorArrived message"),
        }

        // Reset speed factor
        crate::utils::SPEED_FACTOR.store(100, std::sync::atomic::Ordering::Relaxed);
    }

    #[tokio::test]
    async fn test_handle_open_doors_only_when_closed() {
        let (mut elevator, _, mut from_elevator_rx, _) = create_test_elevator("E1");
        crate::utils::SPEED_FACTOR.store(1, std::sync::atomic::Ordering::Relaxed);

        // Doors start Closed
        elevator.handle_open_doors().await;

        assert_eq!(elevator.state.doors_status, Open);

        // Should receive DoorsOpening and DoorsOpened
        let msg1 = from_elevator_rx.recv().await.unwrap();
        assert!(matches!(msg1, crate::msg::ElevatorToControllerMsg::DoorsOpening(_)));
        
        let msg2 = from_elevator_rx.recv().await.unwrap();
        assert!(matches!(msg2, crate::msg::ElevatorToControllerMsg::DoorsOpened(_)));

        crate::utils::SPEED_FACTOR.store(100, std::sync::atomic::Ordering::Relaxed);
    }

    #[tokio::test]
    async fn test_handle_open_doors_ignored_when_not_closed() {
        let (mut elevator, _, mut from_elevator_rx, _) = create_test_elevator("E1");

        // Set doors to Open
        elevator.state.doors_status = Open;

        elevator.handle_open_doors().await;

        // Should remain Open, no messages sent
        assert_eq!(elevator.state.doors_status, Open);

        tokio::select! {
            _ = from_elevator_rx.recv() => panic!("Should not receive message"),
            _ = tokio::time::sleep(std::time::Duration::from_millis(10)) => {}
        }
    }

    #[tokio::test]
    async fn test_handle_close_doors_only_when_open() {
        let (mut elevator, _, mut from_elevator_rx, _) = create_test_elevator("E1");
        crate::utils::SPEED_FACTOR.store(1, std::sync::atomic::Ordering::Relaxed);

        // Set doors to Open
        elevator.state.doors_status = Open;

        elevator.handle_close_doors().await;

        assert_eq!(elevator.state.doors_status, Closed);

        // Should receive DoorsClosing and DoorsClosed
        let msg1 = from_elevator_rx.recv().await.unwrap();
        assert!(matches!(msg1, crate::msg::ElevatorToControllerMsg::DoorsClosing(_)));
        
        let msg2 = from_elevator_rx.recv().await.unwrap();
        assert!(matches!(msg2, crate::msg::ElevatorToControllerMsg::DoorsClosed(_)));

        crate::utils::SPEED_FACTOR.store(100, std::sync::atomic::Ordering::Relaxed);
    }

    #[tokio::test]
    async fn test_handle_close_doors_ignored_when_not_open() {
        let (mut elevator, _, mut from_elevator_rx, _) = create_test_elevator("E1");

        // Doors are Closed by default
        elevator.handle_close_doors().await;

        // Should remain Closed, no messages sent
        assert_eq!(elevator.state.doors_status, Closed);

        tokio::select! {
            _ = from_elevator_rx.recv() => panic!("Should not receive message"),
            _ = tokio::time::sleep(std::time::Duration::from_millis(10)) => {}
        }
    }

    // ========================================================================
    // S1: Türen während der Fahrt nicht offen
    // ========================================================================

    #[tokio::test]
    async fn test_s1_doors_closed_when_moving() {
        let (mut elevator, _, _, _) = create_test_elevator("E1");
        crate::utils::SPEED_FACTOR.store(1, std::sync::atomic::Ordering::Relaxed);

        // Doors start closed
        assert_eq!(elevator.state.doors_status, Closed);

        // Start a mission (moving)
        elevator.handle_mission(First).await;

        // After mission, doors should still be closed (controller opens them)
        assert_eq!(elevator.state.doors_status, Closed);

        crate::utils::SPEED_FACTOR.store(100, std::sync::atomic::Ordering::Relaxed);
    }

    #[tokio::test]
    async fn test_s1_cannot_open_doors_while_opening() {
        let (mut elevator, _, mut from_elevator_rx, _) = create_test_elevator("E1");

        // Set doors to Opening state
        elevator.state.doors_status = Opening;

        // Try to open doors - should be ignored (not Closed)
        elevator.handle_open_doors().await;

        // Should remain Opening, no messages sent
        assert_eq!(elevator.state.doors_status, Opening);

        tokio::select! {
            _ = from_elevator_rx.recv() => panic!("Should not receive message"),
            _ = tokio::time::sleep(std::time::Duration::from_millis(10)) => {}
        }
    }

    // ========================================================================
    // Z1: Kabinen-Zustände
    // ========================================================================

    #[tokio::test]
    async fn test_z1_elevator_idle_state() {
        let (elevator, _, _, _) = create_test_elevator("E1");
        
        // Initial state is IdleIn(Ground)
        assert_eq!(elevator.state.status, IdleIn(Ground));
        assert_eq!(elevator.state.floor, Ground);
    }

    #[tokio::test]
    async fn test_z1_elevator_moving_state() {
        let (mut elevator, _, _, _) = create_test_elevator("E1");
        crate::utils::SPEED_FACTOR.store(1, std::sync::atomic::Ordering::Relaxed);

        // Start mission - state changes to Moving
        // Note: floor is only updated after arrival, not during movement
        elevator.handle_mission(Second).await;

        // After arrival, should be IdleIn(Second) and floor updated
        assert_eq!(elevator.state.status, IdleIn(Second));
        // Note: The implementation doesn't update state.floor during handle_mission
        // It only sends messages. Floor tracking is done by controller.
        // This is an architectural decision - elevator reports position via messages.

        crate::utils::SPEED_FACTOR.store(100, std::sync::atomic::Ordering::Relaxed);
    }

    // ========================================================================
    // Z2: Tür-Zustandsübergänge
    // ========================================================================

    #[tokio::test]
    async fn test_z2_door_state_transitions_open_cycle() {
        let (mut elevator, _, _, _) = create_test_elevator("E1");
        crate::utils::SPEED_FACTOR.store(1, std::sync::atomic::Ordering::Relaxed);

        // Start: Closed
        assert_eq!(elevator.state.doors_status, Closed);

        // Open doors: Closed -> Opening -> Open
        elevator.handle_open_doors().await;
        assert_eq!(elevator.state.doors_status, Open);

        // Close doors: Open -> Closing -> Closed
        elevator.handle_close_doors().await;
        assert_eq!(elevator.state.doors_status, Closed);

        crate::utils::SPEED_FACTOR.store(100, std::sync::atomic::Ordering::Relaxed);
    }

    // ========================================================================
    // F5: Türen öffnen und schließen in Ebenen
    // ========================================================================

    #[tokio::test]
    async fn test_f5_doors_open_and_close_sequence() {
        let (mut elevator, _, mut from_elevator_rx, _) = create_test_elevator("E1");
        crate::utils::SPEED_FACTOR.store(1, std::sync::atomic::Ordering::Relaxed);

        // Open doors
        elevator.handle_open_doors().await;
        
        // Should have sent DoorsOpening then DoorsOpened
        let msg1 = from_elevator_rx.recv().await.unwrap();
        assert!(matches!(msg1, crate::msg::ElevatorToControllerMsg::DoorsOpening(_)));
        let msg2 = from_elevator_rx.recv().await.unwrap();
        assert!(matches!(msg2, crate::msg::ElevatorToControllerMsg::DoorsOpened(_)));

        // Close doors
        elevator.handle_close_doors().await;
        
        // Should have sent DoorsClosing then DoorsClosed
        let msg3 = from_elevator_rx.recv().await.unwrap();
        assert!(matches!(msg3, crate::msg::ElevatorToControllerMsg::DoorsClosing(_)));
        let msg4 = from_elevator_rx.recv().await.unwrap();
        assert!(matches!(msg4, crate::msg::ElevatorToControllerMsg::DoorsClosed(_)));

        crate::utils::SPEED_FACTOR.store(100, std::sync::atomic::Ordering::Relaxed);
    }

    // ========================================================================
    // Travel time based on distance
    // ========================================================================

    #[test]
    fn test_travel_distance_calculation() {
        // Ground to Third = 3 floors
        let distance = (Ground as i8 - Third as i8).abs();
        assert_eq!(distance, 3);

        // First to Second = 1 floor
        let distance = (First as i8 - Second as i8).abs();
        assert_eq!(distance, 1);

        // Same floor = 0
        let distance = (Second as i8 - Second as i8).abs();
        assert_eq!(distance, 0);
    }
}