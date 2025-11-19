use crate::controller::{BoardingStatus, Floor};

#[derive(Clone)]
#[derive(Debug)]
pub enum PersonToControllerMsg {
    PersonRequestElevator(Floor),                           // Floor
    PersonEnteringElevator(String, String),                 // Person ID, Elevator ID
    PersonEnteredElevator(String, String),                  // Person ID, Elevator ID
    PersonLeavingElevator(String, String),                  // Person ID, Elevator ID
    PersonLeftElevator(String, String),                     // Person ID, Elevator ID
    PersonChoosingFloor(String, String, Floor)              // Person ID, Elevator ID, Floor
}

#[derive(Clone)]
#[derive(Debug)]
pub enum ControllerToPersonsMsg {
    ElevatorDeparted(String, Floor),                        // Elevator ID, Floor
    ElevatorHalt(String, Floor),                            // Elevator ID, Floor
    TooManyPassengers(String, String),                      // Person ID, Elevator ID
    UpdateBoardingStatus(String, String, BoardingStatus)    // Person ID, Elevator ID, Boarding Status
}

#[derive(Clone)]
#[derive(Debug)]
pub enum ControllerToElevatorsMsg {
    ElevatorMission(String, Floor),                         // Elevator ID, Target Floor
    OpenDoors(String),                                      // Elevator ID
    CloseDoors(String)                                      // Elevator ID
}

#[derive(Clone)]
pub enum ElevatorToControllerMsg {
    ElevatorMoving(String, Floor, Floor),                   // Elevator ID, From Floor, To Floor
    ElevatorArrived(String, Floor),                         // Elevator ID, Floor
    DoorsOpening(String),                                   // Elevator ID
    DoorsClosing(String),                                   // Elevator ID
    DoorsOpened(String),                                    // Elevator ID
    DoorsClosed(String)                                     // Elevator ID
}