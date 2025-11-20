use std::sync::mpsc::Receiver;
use rumqttc::{AsyncClient, EventLoop, MqttOptions};
use tokio::sync::mpsc::Sender;
use tokio::task::JoinHandle;
use crate::controller::Floor;

pub enum Send {
    Elevator {
        id: String,
        msg: ElevatorMsg
    },
    Person {
        id: String,
        msg: PersonMsg
    }
}

pub enum ElevatorMsg {
    Position(Floor),
    // ...
}

pub enum PersonMsg {
    Done,
    // ...
}

enum Receive {
    Person {
        id: String,
        current_floor: Floor,
        destination_floor: Floor,
    }
}

pub struct MqttConnector {
    from: Receiver<Send>,
    to: Sender<Send>,
    client: AsyncClient,
    event_loop: EventLoop
}

impl MqttConnector {
    pub fn init(mut self) -> JoinHandle<()> {
        tokio::spawn(async move {
            loop {
                match self.event_loop.poll().await {
                    Ok(notification) => {
                        if let rumqttc::Event::Incoming(rumqttc::Packet::Publish(p)) = notification {
                            let person_id = format!("Person_{}", rand::random::<u16>());
                        }
                    }
                    Err(e) => {
                        
                    }
                }
            }
        })
    }

    pub fn new(from: Receiver<Send>, to: Sender<Send>) -> Self {
        let mqttoptions = MqttOptions::new("elevator-sim", "localhost", 1883);
        let (client, event_loop) = AsyncClient::new(mqttoptions, 10);
        MqttConnector {
            from,
            to,
            client,
            event_loop
        }
    }
}
