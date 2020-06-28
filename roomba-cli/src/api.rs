use paho_mqtt as mqtt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::SystemTime;
use structopt::StructOpt;

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case", untagged)]
pub enum Message {
    Cmd {
        command: Command,
        time: u64,
        initiator: String,
        #[serde(flatten)]
        extra: Option<Extra>,
    },
    Delta,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
#[derive(StructOpt)]
pub enum Command {
    Start,
    Clean,
    Pause,
    Stop,
    Resume,
    Dock,
    Evac,
    Train,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case", untagged)]
pub enum Extra {
    StartRegions(StartRegions),
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
#[derive(StructOpt)]
pub struct StartRegions {
    pub pmap_id: String,
    pub user_pmapv_id: String,
    #[structopt(long, parse(from_flag))]
    pub ordered: i64,
    pub regions: Vec<Region>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub struct Region {
    pub region_id: String,
    #[serde(rename = "type")]
    pub type_: String,
}

impl Region {
    fn from_id(id: u64) -> Self {
        Self {
            region_id: id.to_string(),
            type_: "rid".to_string(),
        }
    }
}

impl std::str::FromStr for Region {
    type Err = std::num::ParseIntError;

    fn from_str(src: &str) -> Result<Self, Self::Err> {
        u64::from_str_radix(src, 10).map(|id| Region::from_id(id))
    }
}

impl Message {
    pub fn new_command(command: Command, extra: Option<Extra>) -> Self {
        let time = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self::Cmd {
            command,
            time,
            initiator: "localApp".to_string(),
            extra,
        }
    }

    pub fn topic(&self) -> &'static str {
        match self {
            Self::Cmd { .. } => "cmd",
            _ => todo!(),
        }
    }

    pub fn payload(&self) -> String {
        serde_json::to_string(self).expect("serialization failed")
    }

    pub fn send_message(&self, client: &mqtt::Client) -> mqtt::Result<()> {
        client.publish(
            mqtt::MessageBuilder::new()
                .topic(self.topic())
                .payload(self.payload())
                .qos(0)
                .finalize(),
        )
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Info {
    pub ip: String,
    pub hostname: String,
    #[serde(rename = "robotid")]
    pub robot_id: String,
    #[serde(flatten)]
    pub other: HashMap<String, serde_json::Value>,
}
