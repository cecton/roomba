use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::SystemTime;

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
    StartRegions {
        pmap_id: String,
        user_pmapv_id: String,
        ordered: i64,
        regions: Vec<Region>,
    },
}

#[derive(Serialize, Deserialize, Debug)]
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

#[derive(Serialize, Deserialize, Debug)]
pub struct Info {
    pub ip: String,
    pub hostname: String,
    #[serde(rename = "robotid")]
    pub robot_id: Option<String>,
    #[serde(flatten)]
    pub attrs: HashMap<String, serde_json::Value>,
}

impl Info {
    pub fn robot_id(&self) -> String {
        self.robot_id
            .clone()
            .unwrap_or_else(|| {
                let prefixes = &[ "iRobot-", "Roomba-" ];

                for prefix in prefixes {
                    if self.hostname.starts_with(prefix) {
                        return self.hostname[prefix.len()..].to_string()
                    }
                }
                // TODO: Should we return a retult/error or option instead of this?
                "<unknown>".to_string()
            })
    }
}

/////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_robot_id() {
        const ROBOT_ID: &str = "12345";

        // Valid robot_id field

        let info = Info {
            ip: "192.168.0.251".to_string(),
            hostname: "iRobot-67890".to_string(),
            robot_id: Some(ROBOT_ID.to_string()),
            attrs: HashMap::new(),
        };

        assert_eq!(ROBOT_ID, &info.robot_id());

        // With "iRobot-" hostname

        let info = Info {
            ip: "192.168.0.251".to_string(),
            hostname: format!("iRobot-{}", ROBOT_ID),
            robot_id: None,
            attrs: HashMap::new(),
        };

        assert_eq!(ROBOT_ID, &info.robot_id());

        // With "Roomba-" hostname

        let info = Info {
            ip: "192.168.0.251".to_string(),
            hostname: format!("Roomba-{}", ROBOT_ID),
            robot_id: None,
            attrs: HashMap::new(),
        };

        assert_eq!(ROBOT_ID, &info.robot_id());

    }
}
