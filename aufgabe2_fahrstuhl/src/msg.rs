use crate::controller::{BoardingStatus, Floor};

#[derive(Clone, PartialEq)]
#[derive(Debug)]
pub enum PersonToControllerMsg {
    PersonRequestElevator(Floor),                           // Floor
    PersonEnteringElevator(String, String),                 // Person ID, Elevator ID
    PersonEnteredElevator(String, String),                  // Person ID, Elevator ID
    PersonLeavingElevator(String, String),                  // Person ID, Elevator ID
    PersonLeftElevator(String, String),                     // Person ID, Elevator ID
    PersonChoosingFloor(String, String, Floor)              // Person ID, Elevator ID, Floor
}

#[derive(Clone, PartialEq)]
#[derive(Debug)]
pub enum ControllerToPersonsMsg {
    ElevatorHalt(String, Floor),                            // Elevator ID, Floor
    UpdateBoardingStatus(String, String, BoardingStatus)    // Person ID, Elevator ID, Boarding Status
}

#[derive(Clone, PartialEq)]
#[derive(Debug)]
pub enum ControllerToElevatorsMsg {
    ElevatorMission(String, Floor),                         // Elevator ID, Target Floor
    OpenDoors(String),                                      // Elevator ID
    CloseDoors(String)                                      // Elevator ID
}

#[derive(Clone, PartialEq)]
#[derive(Debug)]
pub enum ElevatorToControllerMsg {
    ElevatorMoving(String, Floor, Floor),                   // Elevator ID, From Floor, To Floor
    ElevatorArrived(String, Floor),                         // Elevator ID, Floor
    DoorsOpening(String),                                   // Elevator ID
    DoorsClosing(String),                                   // Elevator ID
    DoorsOpened(String),                                    // Elevator ID
    DoorsClosed(String)                                     // Elevator ID
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::controller::Floor::*;
    use crate::controller::BoardingStatus::*;

    #[test]
    fn test_floor_distance_calculation() {
        // Test distance between floors using the formula from elevator.rs
        let ground = Ground as i8;
        let first = First as i8;
        let second = Second as i8;
        let third = Third as i8;

        // Same floor = 0 distance
        assert_eq!((ground - ground).abs(), 0);
        
        // Adjacent floors = 1 distance
        assert_eq!((first - ground).abs(), 1);
        assert_eq!((ground - first).abs(), 1);
        
        // Two floors apart = 2 distance
        assert_eq!((second - ground).abs(), 2);
        assert_eq!((ground - second).abs(), 2);
        
        // Three floors apart = 3 distance
        assert_eq!((third - ground).abs(), 3);
        assert_eq!((ground - third).abs(), 3);
    }

    #[test]
    fn test_floor_ordering() {
        assert!((Ground as i8) < (First as i8));
        assert!((First as i8) < (Second as i8));
        assert!((Second as i8) < (Third as i8));
    }

    #[test]
    fn test_person_to_controller_msg_clone() {
        let msg = PersonToControllerMsg::PersonRequestElevator(Ground);
        let cloned = msg.clone();
        assert_eq!(msg, cloned);
    }

    #[test]
    fn test_controller_to_elevators_msg_variants() {
        let mission = ControllerToElevatorsMsg::ElevatorMission("E1".to_string(), First);
        let open = ControllerToElevatorsMsg::OpenDoors("E1".to_string());
        let close = ControllerToElevatorsMsg::CloseDoors("E1".to_string());
        
        // Ensure they are different
        assert_ne!(mission, open);
        assert_ne!(open, close);
    }

    #[test]
    fn test_elevator_to_controller_msg_variants() {
        let moving = ElevatorToControllerMsg::ElevatorMoving("E1".to_string(), Ground, First);
        let arrived = ElevatorToControllerMsg::ElevatorArrived("E1".to_string(), First);
        let opening = ElevatorToControllerMsg::DoorsOpening("E1".to_string());
        let opened = ElevatorToControllerMsg::DoorsOpened("E1".to_string());
        let closing = ElevatorToControllerMsg::DoorsClosing("E1".to_string());
        let closed = ElevatorToControllerMsg::DoorsClosed("E1".to_string());
        
        assert_ne!(moving, arrived);
        assert_ne!(opening, opened);
        assert_ne!(closing, closed);
    }

    #[test]
    fn test_boarding_status_clone() {
        let accepted = Accepted;
        let rejected = Rejected;
        
        assert_eq!(accepted.clone(), Accepted);
        assert_eq!(rejected.clone(), Rejected);
        assert_ne!(accepted, rejected);
    }
}