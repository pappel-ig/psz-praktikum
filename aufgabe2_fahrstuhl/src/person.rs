use rand::prelude::SliceRandom;
use rand::rng;
use crate::controller::Floor;

#[derive(Clone)]
pub enum PersonStatus {
    Idle,
    Entering,
    Choosing,
    InElevator,
    Leaving
}

pub struct Person {
    pub id: String,
    pub status: PersonStatus,
    pub current_floor: Floor,
    pub destination_floor: Floor
}

impl Person {
    pub fn new(id: &str) -> Self {
        let (current_floor, destination_floor) = pick_two_distinct_floors();
        Person {
            id: id.to_string(),
            status: PersonStatus::Idle,
            current_floor,
            destination_floor
        }
    }
}

fn pick_two_distinct_floors() -> (Floor, Floor) {
    let mut rng = rng();
    let mut floors = vec![Floor::Ground, Floor::First, Floor::Second];
    floors.shuffle(&mut rng);
    (floors[0], floors[1])
}