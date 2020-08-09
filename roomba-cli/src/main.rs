mod cli;

use async_std::task::block_on;
use futures::stream::StreamExt;
use roomba::{api, Client};
use serde::{Deserialize, Serialize};
use std::io::Write;
use structopt::StructOpt;

const ROOMBA_CFG: &str = "roomba.toml";

#[derive(Serialize, Deserialize, Default)]
struct Config {
    hostname: Option<String>,
    username: Option<String>,
    password: Option<String>,
    pmap_id: Option<String>,
    user_pmapv_id: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    rooms: Vec<Room>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Room {
    name: String,
    #[serde(flatten)]
    region: api::Region,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Default to "error" log level unless overridden by environment
    env_logger::init_from_env(env_logger::Env::default().filter_or("RUST_LOG", "error"));

    let config_file = dirs::config_dir().unwrap_or_default().join(ROOMBA_CFG);
    let mut config = if config_file.exists() {
        std::fs::read_to_string(&config_file)
            .map_err(|err| err.to_string())
            .and_then(|ref content| toml::from_str(content).map_err(|err| err.to_string()))
            .unwrap_or_else(|err| {
                eprintln!(
                    "Could not read configuration file `{}`: {}",
                    config_file.display(),
                    err
                );
                Config::default()
            })
    } else {
        Config::default()
    };
    let save_config = |config| {
        toml::to_string_pretty(&config)
            .map_err(|err| err.to_string())
            .and_then(|x| std::fs::write(&config_file, x).map_err(|err| err.to_string()))
            .unwrap_or_else(|err| {
                eprintln!(
                    "Could not write configuration file `{}`: {}",
                    config_file.display(),
                    err
                )
            })
    };
    let cli = cli::Cli::from_args();

    match cli.command {
        cli::AnyCommand::Unauthenticated(cli::UnauthenticatedCommand::FindIp { no_save }) => {
            let stdout = std::io::stdout();

            for res in Client::find_ip_address()? {
                let mut fh = stdout.lock();
                if let Ok(info) = res {
                    let _ = writeln!(
                        fh,
                        "found.\nIP address: {}\nBLID/Username/Robot ID: {}",
                        info.ip,
                        info.robot_id()
                            .unwrap_or_else(|err| panic!("{}: {:?}", err, info)),
                    );
                    if !no_save {
                        config.hostname = Some(info.ip.clone());
                        config.username = info.robot_id().ok();
                        save_config(config);
                        break;
                    }
                } else {
                    let _ = fh.write(b".");
                }
                let _ = fh.flush();
            }

            Ok(())
        }
        cli::AnyCommand::Unauthenticated(cli::UnauthenticatedCommand::GetPassword {
            hostname,
            no_save,
        }) => {
            let hostname = match hostname {
                Some(ref x) => x,
                None => config.hostname.as_ref().ok_or_else(|| format!("Missing hostname in the configuration. Please run `{} find-ip --save` first", std::env::current_exe().unwrap_or_default().display()))?,
            };

            println!(
                "Warning: please hold the Home button for 2 seconds and check that the ring led is \
                blinking blue."
            );

            let password = loop {
                match Client::get_password(hostname) {
                    Err(err) => {
                        println!("{}", err);
                        std::thread::sleep(std::time::Duration::from_secs(3));
                    }
                    Ok(password) => break password,
                }
            };

            println!("Password: {}", password);

            if !no_save {
                config.hostname = Some(hostname.to_string());
                config.password = Some(password);
                save_config(config);
            }

            Ok(())
        }
        cli::AnyCommand::Authenticated(cli) => block_on(async {
            let mut client = Client::new(cli.hostname, cli.username, cli.password, 0).await?;

            match cli.command {
                Some(command) => {
                    let (command, extra) = command.into_command_with_extra();
                    let message = api::Message::new_command(command, extra);

                    client.send_message(&message).await?;
                }
                None => {
                    while let Some(maybe_msg) = client.events.next().await {
                        if let Some(msg) = maybe_msg {
                            println!("{}", msg);
                        }
                    }
                }
            }

            Ok(())
        }),
    }
}
