mod cli;

use async_std::task::block_on;
use futures_util::stream::StreamExt;
use roomba::{api, Client};
use std::io::Write;
use structopt::StructOpt;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Default to "error" log level unless overridden by environment
    env_logger::init_from_env(env_logger::Env::default().filter_or("RUST_LOG", "error"));

    let cli = cli::Cli::from_args();

    match cli.command {
        cli::AnyCommand::Unauthenticated(cli::UnauthenticatedCommand::FindIp) => {
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
                } else {
                    let _ = fh.write(b".");
                }
                let _ = fh.flush();
            }

            Ok(())
        }
        cli::AnyCommand::Unauthenticated(cli::UnauthenticatedCommand::GetPassword { hostname }) => {
            println!(
                "Warning: please hold the Home button for 2 seconds and check that the ring led is \
                blinking blue."
            );

            let password = loop {
                match Client::get_password(&hostname) {
                    Err(err) => {
                        println!("{}", err);
                        std::thread::sleep(std::time::Duration::from_secs(3));
                    }
                    Ok(password) => break password,
                }
            };

            println!("Password: {}", password);

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
