use futures_util::stream::StreamExt;
use lapin::{options::*, types::FieldTable, Connection, ConnectionProperties};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let url = env::var("LAPIN_URL").expect("LAPIN_URL must be set");

    // Connect to the AMQP server
    let conn = Connection::connect(&url, ConnectionProperties::default()).await?;
    println!("Connected to RabbitMQ");

    // Create a channel
    let channel = conn.create_channel().await?;
    println!("Created a channel");

    // Declare a queue (if it doesn't already exist)
    let queue = channel
        .queue_declare(
            "drone_feed", // Queue name
            QueueDeclareOptions {
                durable: true, // Persist the queue across broker restarts
                ..Default::default()
            },
            FieldTable::default(), // Extra parameters
        )
        .await?;
    println!("Declared queue: {:?}", queue.name());

    // Consume messages from the queue
    let mut consumer = channel
        .basic_consume(
            "drone_feed", // Queue name
            "drone_tag",  // Consumer tag
            BasicConsumeOptions::default(),
            FieldTable::default(),
        )
        .await?;
    println!("Waiting for messages...");

    // Process each message
    while let Some(delivery) = consumer.next().await {
        let delivery = delivery?;
        println!(
            "Received message: {:?}",
            std::str::from_utf8(&delivery.data).unwrap()
        );

        // Acknowledge the message
        delivery.ack(BasicAckOptions::default()).await?;
    }

    Ok(())
}
