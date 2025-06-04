//! Feed module provides functionality for creating and managing AMQP message consumers
//! through RabbitMQ connections.

use lapin::options::BasicConsumeOptions;
use lapin::types::FieldTable;
use lapin::{Connection, Consumer};

/// Represents a message feed from a RabbitMQ queue.
///
/// # Fields
///
/// * `name` - The name of the queue to consume from
/// * `inp` - The AMQP consumer instance
#[derive(Debug)]
pub(crate) struct Feed {
    pub name: String,
    pub inp: Consumer,
}

impl Feed {
    /// Creates a new Feed instance.
    ///
    /// # Arguments
    ///
    /// * `conn` - The RabbitMQ connection
    /// * `name` - The name of the queue to consume from
    /// * `tag` - The consumer tag to identify this consumer
    ///
    /// # Returns
    ///
    /// Returns a Result containing the new Feed instance or an error
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// * Channel creation fails
    /// * Consumer creation fails
    #[tracing::instrument(skip(conn))]
    pub async fn new(conn: &Connection, name: &str, tag: &str) -> eyre::Result<Self> {
        // Create a channel
        let data_ch = conn.create_channel().await?;
        eprintln!("Created {name} channel");

        let data = data_ch
            .basic_consume(
                name,
                tag,
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await?;

        Ok(Feed {
            name: name.into(),
            inp: data,
        })
    }
}
