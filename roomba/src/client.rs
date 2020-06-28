use crate::api::{Info, Message};
use futures_core::stream::Stream;
use openssl::ssl::{SslConnector, SslMethod, SslVerifyMode};
use std::collections::HashSet;
use std::io::Read;
use std::io::Write;
use std::net::{TcpStream, UdpSocket};
use std::str;

const DISCOVERY_PACKET: &[u8] = b"irobotmcs";

pub struct Client {
    pub mqtt: paho_mqtt::AsyncClient,
    pub events: Box<dyn Stream<Item = Option<paho_mqtt::message::Message>> + Unpin>,
}

impl Client {
    pub async fn new<S: AsRef<str>, B: Into<String>, P: Into<String>>(
        hostname: S,
        blid: B,
        password: P,
        buffer: usize,
    ) -> paho_mqtt::Result<Self> {
        let uri = format!("ssl://{}:8883", hostname.as_ref());
        let opts = paho_mqtt::CreateOptionsBuilder::new()
            .server_uri(uri)
            .finalize();

        let mut client = paho_mqtt::AsyncClient::new(opts)?;

        let ssl_opts = paho_mqtt::SslOptionsBuilder::new()
            .enable_server_cert_auth(false)
            .finalize();

        let conn_opts = paho_mqtt::ConnectOptionsBuilder::new()
            .ssl_options(ssl_opts)
            .user_name(blid)
            .password(password)
            .finalize();

        let rx = client.get_stream(buffer);
        client.connect(conn_opts).await?;

        Ok(Self {
            mqtt: client,
            events: Box::new(rx),
        })
    }

    pub async fn send_message(&self, message: &Message) -> paho_mqtt::Result<()> {
        self.mqtt
            .publish(
                paho_mqtt::MessageBuilder::new()
                    .topic(message.topic())
                    .payload(message.payload())
                    .qos(0)
                    .finalize(),
            )
            .await
    }

    pub fn find_ip_address() -> std::io::Result<Discovery> {
        Discovery::new()
    }

    pub fn get_password<H: AsRef<str>>(hostname: H) -> std::io::Result<String> {
        let packet: &[u8] = &[0xf0, 0x05, 0xef, 0xcc, 0x3b, 0x29, 0x00];

        let mut builder = SslConnector::builder(SslMethod::tls()).unwrap();
        builder.set_verify(SslVerifyMode::NONE);
        let connector = builder.build();

        let uri = format!("{}:8883", hostname.as_ref());
        debug!("Attempting SSL conection to '{}'", uri);
        let socket = TcpStream::connect(uri)?;
        socket.set_read_timeout(Some(std::time::Duration::from_secs(3)))?;
        let mut stream = connector.connect("ignore", socket).unwrap();

        let mut attempts = 0;
        loop {
            let mut data = Vec::new();

            stream.write_all(&packet)?;
            match stream.read_to_end(&mut data) {
                Err(_) if attempts < 3 => {
                    attempts += 1;
                }
                Err(err) => {
                    warn!("Couldn't retrieve password: {}", err);
                    return Err(err);
                }
                Ok(_) => {
                    if let Some(password) = data
                        .rsplit(|&x| x == 0)
                        .filter(|x| !x.is_empty())
                        .find_map(|x| String::from_utf8(x.to_vec()).ok())
                    {
                        info!("Retrieved password: '{}'", password);
                        break Ok(password);
                    }
                }
            }
        }
    }
}

pub struct Discovery {
    socket: UdpSocket,
    found: HashSet<String>,
}

impl Discovery {
    pub fn new() -> std::io::Result<Discovery> {
        let socket = UdpSocket::bind("0.0.0.0:5678")?;
        socket.set_broadcast(true)?;
        socket.set_read_timeout(Some(std::time::Duration::from_secs(3)))?;

        Ok(Discovery {
            socket,
            found: HashSet::new(),
        })
    }
}

impl Iterator for Discovery {
    type Item = std::io::Result<Info>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut data = [0; 800];

        loop {
            match self
                .socket
                .send_to(DISCOVERY_PACKET, "255.255.255.255:5678")
            {
                Err(err) => {
                    return Some(Err(err));
                }
                Ok(_) => loop {
                    match self.socket.recv(&mut data) {
                        Err(err) => {
                            warn!("Error receiving discovery packet: {}", err);
                            return Some(Err(err));
                        }
                        Ok(length) => {
                            if &data[..length] != DISCOVERY_PACKET {
                                match str::from_utf8(&data[..length]) {
                                    Ok(s) => trace!("Discovery received: {}", s),
                                    Err(_) => warn!("Discovery should return a JSON string."),
                                }
                                match serde_json::from_slice::<Info>(&data[..length]) {
                                    Ok(info) => {
                                        if !self.found.contains(&info.ip) {
                                            self.found.insert(info.ip.clone());
                                            return Some(Ok(info));
                                        }
                                    }
                                    Err(err) => warn!("Error parsing discovery JSON: {}", err),
                                }
                            }
                        }
                    }
                },
            }
        }
    }
}
