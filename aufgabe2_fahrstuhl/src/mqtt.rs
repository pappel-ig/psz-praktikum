use paho_mqtt::{AsyncClient, ConnectOptions, Message};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::task::JoinHandle;
use ElevatorMsg::Position;
use PersonMsg::{Boarding, StatusUpdate};
use Send::{ElevatorTopic, PersonTopic};
use crate::controller::{BoardingStatus, Floor};
use crate::elevator::DoorStatus;
use crate::mqtt::ElevatorMsg::{Door, Missions, Moving, Passengers, Request};
use crate::person::PersonStatus;

pub enum Send {
    ElevatorTopic {
        id: String,
        msg: ElevatorMsg
    },
    PersonTopic {
        id: String,
        msg: PersonMsg
    }
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum ElevatorMsg {
    Position { floor: Floor },
    Door { status: DoorStatus },
    Moving { from: Floor, to: Floor },
    Passengers { passengers: Vec<String> },
    Request { floor: Floor },
    Missions { missions: Vec<Floor> }
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum PersonMsg {
    StatusUpdate { status: PersonStatus },
    Boarding { status: BoardingStatus }
    // ...
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum Receive {
    Person {
        id: String,
        curr: Floor,
        dest: Floor,
    }
}

pub struct MqttConnector {
    from: Receiver<Send>,
    to: Sender<Receive>,
    client: AsyncClient,
}

impl MqttConnector {

    pub async fn new(from: Receiver<Send>, to: Sender<Receive>) -> Self {
        let client = AsyncClient::new("mqtt://localhost:1883").unwrap();
        let _ = client.connect(ConnectOptions::new()).await;
        MqttConnector {
            from,
            to,
            client,
        }
    }

    pub fn mqtt_subscriber(&self) -> JoinHandle<()> {
        let mut client = self.client.clone();
        let to = self.to.clone();
        tokio::spawn(async move {
            client.subscribe("person/introduce", 1).await.unwrap();
            let receiver = client.get_stream(100);
            loop {
                if let Ok(Some(msg)) = receiver.recv().await {
                    if let Some(receive) = MqttConnector::parse(&msg) {
                        let _ = to.send(receive).await;
                    }
                }
            }
        })
    }

    pub fn mqtt_publisher(mut self) -> JoinHandle<()> {
        let client = self.client.clone();
        tokio::spawn(async move {
            loop {
                if let Some(send) = self.from.recv().await {
                    match send {
                        ElevatorTopic { id, msg } => {
                            match msg {
                                Position { .. } => {
                                    let _ = client.publish(Message::new(format!("elevator/{}/position", id), serde_json::to_string(&msg).unwrap(), 1)).await;
                                },
                                Door { .. } => {
                                    let _ = client.publish(Message::new(format!("elevator/{}/door", id), serde_json::to_string(&msg).unwrap(), 1)).await;
                                },
                                Moving { .. } => {
                                    let _ = client.publish(Message::new(format!("elevator/{}/moving", id), serde_json::to_string(&msg).unwrap(), 1)).await;
                                },
                                Passengers { .. } => {
                                    let _ = client.publish(Message::new(format!("elevator/{}/passengers", id), serde_json::to_string(&msg).unwrap(), 1)).await;
                                },
                                Request { .. } => {
                                    let _ = client.publish(Message::new(format!("elevator/{}/request", id), serde_json::to_string(&msg).unwrap(), 1)).await;
                                },
                                Missions { .. } => {
                                    let _ = client.publish(Message::new(format!("elevator/{}/missions", id), serde_json::to_string(&msg).unwrap(), 1)).await;
                                },
                            }
                        }
                        PersonTopic { id, msg } => {
                            match msg {
                                StatusUpdate { .. } => {
                                    let _ = client.publish(Message::new(format!("person/{}/status", id), serde_json::to_string(&msg).unwrap(), 1)).await;
                                },
                                Boarding { .. } => {
                                    let _ = client.publish(Message::new(format!("person/{}/boarding", id), serde_json::to_string(&msg).unwrap(), 1)).await;
                                }
                            }
                        }
                    }
                }
            }
        })
    }

    fn parse(msg: &Message) -> Option<Receive> {
        let payload = msg.payload();
        serde_json::from_slice(payload).ok()
    }
}
