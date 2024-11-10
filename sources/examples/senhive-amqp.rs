//! Connect to a Thales Senhive antenna and fetch messages through Lapin as AMQP client.
//!

use eyre::Result;
use futures_util::stream::StreamExt;
use lapin::{options::*, types::FieldTable, Connection, ConnectionProperties, Consumer};
use std::env;

#[cfg(unix)]
use tokio::signal::unix::{signal, SignalKind};
#[cfg(windows)]
use tokio::signal::windows::ctrl_c;

async fn subscribe(conn: &Connection, name: &str) -> Result<Consumer> {
    // Create a channel
    let data_ch = conn.create_channel().await?;
    println!("Created {name} channel");

    let data = data_ch
        .basic_consume(
            name,
            "drone_tag",
            BasicConsumeOptions::default(),
            FieldTable::default(),
        )
        .await?;
    Ok(data)
}

#[tokio::main]
async fn main() -> Result<()> {
    let url = env::var("LAPIN_URL").expect("LAPIN_URL must be set");

    // Connect to the AMQP server
    let conn = Connection::connect(&url, ConnectionProperties::default()).await?;
    println!("Connected to RabbitMQ");

    // Subscribe to topics
    //
    let mut data = subscribe(&conn, "fused_data").await?;
    let mut alert = subscribe(&conn, "system_alert").await?;
    let mut state = subscribe(&conn, "system_state").await?;

    println!("Waiting for messages...");

    // setup ctrl-c handled
    //
    #[cfg(windows)]
    let mut sig = ctrl_c().unwrap();

    #[cfg(unix)]
    let mut stream = signal(SignalKind::interrupt()).unwrap();

    // Process each message
    //
    loop {
        #[cfg(unix)]
        tokio::select! {
            Some(data) = data.next() => {
                let delivery = data?;
                println!(
                    "Received data message: {:?}",
                    std::str::from_utf8(&delivery.data).unwrap()
                );
            },
            Some(alert) = alert.next() => {
                let delivery = alert?;
                println!(
                    "Received alert message: {:?}",
                    std::str::from_utf8(&delivery.data).unwrap()
                );
            },
            Some(state) = state.next() => {
                let delivery = state?;
                println!(
                    "Received state message: {:?}",
                    std::str::from_utf8(&delivery.data).unwrap()
                );
            },
            Some(_) = stream.recv() => {
                eprintln!("Got SIGINT");
                break;
            },
        }

        #[cfg(windows)]
        tokio::select! {
            Some(data) = data.next() => {
                let delivery = data?;
                println!(
                    "Received data message: {:?}",
                    std::str::from_utf8(&delivery.data).unwrap()
                );
            },
            Some(alert) = alert.next() => {
                let delivery = alert?;
                println!(
                    "Received alert message: {:?}",
                    std::str::from_utf8(&delivery.data).unwrap()
                );
            },
            Some(state) = state.next() => {
                let delivery = state?;
                println!(
                    "Received state message: {:?}",
                    std::str::from_utf8(&delivery.data).unwrap()
                );
            },
            _ = sig.recv() => {
                eprintln!("^C pressed.");
                break;
            },
        }
    }
    Ok(())
}
