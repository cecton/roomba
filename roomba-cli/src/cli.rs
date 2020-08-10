use crate::api;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub struct Cli {
    #[structopt(subcommand)]
    pub command: AnyCommand,
}

#[derive(StructOpt, Debug)]
pub enum AnyCommand {
    #[structopt(name = "command")]
    Authenticated(AuthenticatedCommand),
    #[structopt(flatten)]
    Unauthenticated(UnauthenticatedCommand),
}

#[derive(StructOpt, Debug)]
pub enum Command {
    Start,
    Clean,
    Pause,
    Stop,
    Resume,
    Dock,
    Evac,
    Train,
    StartRegions {
        #[structopt(long)]
        ordered: bool,
        #[structopt(parse(from_str), min_values = 1)]
        regions: Vec<api::Region>,
    },
}

#[derive(StructOpt, Debug)]
pub struct AuthenticatedCommand {
    #[structopt(subcommand)]
    pub command: Option<Command>,
}

#[derive(StructOpt, Debug)]
pub enum UnauthenticatedCommand {
    FindIp {
        #[structopt(long)]
        no_save: bool,
    },
    GetPassword {
        hostname: Option<String>,
        #[structopt(long)]
        no_save: bool,
    },
}

impl Command {
    pub fn into_command_with_extra(
        self,
        pmap_id: &str,
        user_pmapv_id: &str,
    ) -> (api::Command, Option<api::Extra>) {
        match self {
            Command::StartRegions { ordered, regions } => (
                api::Command::Start,
                Some(api::Extra::StartRegions {
                    pmap_id: pmap_id.to_string(),
                    user_pmapv_id: user_pmapv_id.to_string(),
                    ordered: ordered.into(),
                    regions,
                }),
            ),
            Command::Start => (api::Command::Start, None),
            Command::Clean => (api::Command::Clean, None),
            Command::Pause => (api::Command::Pause, None),
            Command::Stop => (api::Command::Stop, None),
            Command::Resume => (api::Command::Resume, None),
            Command::Dock => (api::Command::Dock, None),
            Command::Evac => (api::Command::Evac, None),
            Command::Train => (api::Command::Train, None),
        }
    }
}
