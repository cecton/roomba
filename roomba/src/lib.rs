#[macro_use]
extern crate log;

pub mod api;
pub mod client;

pub use client::*;

use std::{io, result};
use thiserror::Error;
use paho_mqtt as mqtt;
use serde_json as json;
use openssl;

// roomba errors can come from io, mqtt, serde/json, etc

#[derive(Error, Debug)]
pub enum Error {
    #[error("MQTT error: {0}")]
    Mqtt(#[from] mqtt::Error),
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] json::Error),
    #[error("SSL error: {0}")]
    Ssl(#[from] openssl::error::Error),
    #[error("SSL error stack: {0}")]
    SslStack(#[from] openssl::error::ErrorStack),
}

// unified roomba result type
pub type Result<T> = result::Result<T, Error>;


