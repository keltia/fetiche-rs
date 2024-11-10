//! Connect to a Thales Senhive antenna and fetch messages through Lapin as AMQP client.
//!

use eyre::Result;
use futures_util::stream::StreamExt;
use lapin::{options::*, types::FieldTable, Connection, ConnectionProperties};
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    let url = env::var("LAPIN_URL").expect("LAPIN_URL must be set");

    // Connect to the AMQP server
    let conn = Connection::connect(&url, ConnectionProperties::default()).await?;
    println!("Connected to RabbitMQ");

    // Create a channel
    let data_ch = conn.create_channel().await?;
    println!("Created data channel");

    // Create a channel
    let alert_ch = conn.create_channel().await?;
    println!("Created data channel");

    // Create a channel
    let state_ch = conn.create_channel().await?;
    println!("Created data channel");

    // Consume messages from the queue
    let mut data = data_ch
        .basic_consume(
            "fused_data", // Queue name
            "drone_tag",  // Consumer tag
            BasicConsumeOptions::default(),
            FieldTable::default(),
        )
        .await?;

    // Consume messages from the queue
    let mut alert = alert_ch
        .basic_consume(
            "system_alert", // Queue name
            "drone_tag",    // Consumer tag
            BasicConsumeOptions::default(),
            FieldTable::default(),
        )
        .await?;

    // Consume messages from the queue
    let mut state = state_ch
        .basic_consume(
            "system_state", // Queue name
            "drone_tag",    // Consumer tag
            BasicConsumeOptions::default(),
            FieldTable::default(),
        )
        .await?;

    println!("Waiting for messages...");

    // Process each message
    loop {
        tokio::select! {
            Some(data) = data.next() => {
                let delivery = data?;
                println!(
                    "Received data message: {:?}",
                    std::str::from_utf8(&delivery.data).unwrap()
                );
            }
            Some(alert) = alert.next() => {
                let delivery = alert?;
                println!(
                    "Received alert message: {:?}",
                    std::str::from_utf8(&delivery.data).unwrap()
                );

            }
            Some(state) = state.next() => {
                let delivery = state?;
                println!(
                    "Received state message: {:?}",
                    std::str::from_utf8(&delivery.data).unwrap()
                );

            }
        }
    }
    Ok(())
}
