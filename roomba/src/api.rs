use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::SystemTime;

const POSSIBLE_PREFIXES: &[&str] = &["iRobot", "Roomba"];

#[derive(Serialize, Deserialize, Debug, Clone)]
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

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
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

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case", untagged)]
pub enum Extra {
    StartRegions {
        pmap_id: String,
        user_pmapv_id: String,
        ordered: i64,
        regions: Vec<Region>,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub struct Region {
    pub region_id: String,
    #[serde(rename = "type")]
    pub type_: String,
}

impl From<&str> for Region {
    fn from(s: &str) -> Self {
        Self {
            region_id: s.to_string(),
            type_: "rid".to_string(),
        }
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
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Info {
    pub ip: String,
    pub hostname: String,
    #[serde(rename = "robotid")]
    pub robot_id: Option<String>,
    #[serde(flatten)]
    pub attrs: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseRobotIdError;

impl std::fmt::Display for ParseRobotIdError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "could not parse robot ID in hostname")
    }
}

impl Info {
    pub fn robot_id(&self) -> Result<String, ParseRobotIdError> {
        self.robot_id.clone().map(Ok).unwrap_or_else(|| {
            let mut it = self.hostname.splitn(2, '-');
            let (prefix, suffix) = (it.next().unwrap(), it.next());

            if !(POSSIBLE_PREFIXES.contains(&prefix) && suffix.is_some()) {
                return Err(ParseRobotIdError);
            }

            Ok(suffix.unwrap().to_string())
        })
    }
}
