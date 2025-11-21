use std::sync::mpsc::Receiver;
use paho_mqtt::{AsyncClient, ConnectOptions, Token};
use tokio::sync::mpsc::Sender;
use tokio::task::JoinHandle;
use serde::{Deserialize, Deserializer};
use crate::controller::Floor;
use crate::mqtt::Receive::Person;

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

impl Deserialize for Receive {
    fn deserialize<'de, D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>
    {
        deserializer.des
    }
}

pub struct MqttConnector {
    from: Receiver<Send>,
    to: Sender<Send>,
    client: AsyncClient,
    token: Token
}

impl MqttConnector {
    pub fn init(mut self) -> JoinHandle<()> {
        tokio::spawn(async move {

            self.client.subscribe("person/+/introduce", 1);
            for msg in self.client.start_consuming() {
                if let Some(msg) = msg {
                    let payload = msg.payload_str();
                    msg.

                    if let Ok(receive_msg) = serde_json::from_str::<Receive>(&payload) {
                        match receive_msg {
                            Person { id, current_floor, destination_floor } => {
                                let send_msg = Send::Person {
                                    id,
                                    msg: PersonMsg::Done // Example message
                                };
                                self.to.send(send_msg).await.unwrap();
                            }
                        }
                    }
                }
            }
        })
    }

    pub fn new(from: Receiver<Send>, to: Sender<Send>) -> Self {
        let client = AsyncClient::new("mqtt://localhost:1883").unwrap();
        let token = client.connect(ConnectOptions::new());
        MqttConnector {
            from,
            to,
            client,
            token
        }
    }
}
