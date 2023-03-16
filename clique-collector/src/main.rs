//! A service which connects to Discord and stores message metadata in a database.
#![warn(clippy::all, clippy::pedantic, clippy::nursery, missing_docs)]
use serenity::{
    async_trait,
    client::{Client, Context, EventHandler},
    model::{
        channel::Message as SerenityMessage,
        gateway::{GatewayIntents, Ready},
    },
};

async fn store_message(db: &clique_db::Database, msg: SerenityMessage) -> clique_db::DbResult<()> {
    db.insert_user(msg.author.id.0, &msg.author.name).await?;
    let reply_to = match msg.referenced_message {
        Some(referenced) => {
            db.insert_user(referenced.author.id.0, &referenced.author.name)
                .await?;
            Some(referenced.author.id.0)
        }
        None => None,
    };
    clique_db::Message {
        id: msg.id.0,
        guild: msg.guild_id.unwrap().0,
        author: msg.author.id.0,
        channel: msg.channel_id.0,
        reply_to,
        timestamp: *msg.timestamp,
    }
    .insert(db)
    .await
}

struct Collector(clique_db::Database);

#[async_trait]
impl EventHandler for Collector {
    /// Handle an incoming message and store it in the database.
    async fn message(&self, _ctx: Context, msg: SerenityMessage) {
        store_message(&self.0, msg)
            .await
            .expect("error while storing message");
    }

    /// Log to the console once the service is running.
    async fn ready(&self, _ctx: Context, _ready: Ready) {
        eprintln!("Successfully connected to Discord and Postgres.");
    }
}

#[derive(serde::Deserialize)]
struct Config {
    postgres_url: String,
    discord_token: String,
}

/// Parse the config file and start the database and discord connections.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config: Config = toml::from_str(&std::fs::read_to_string("config.toml")?)?;
    let db = clique_db::Database::new(&config.postgres_url).await?;
    let mut discord = Client::builder(config.discord_token, GatewayIntents::GUILD_MESSAGES)
        .event_handler(Collector(db))
        .await
        .expect("Error creating client");
    if let Err(e) = discord.start().await {
        eprintln!("Discord client error: {e:?}");
    }
    Ok(())
}
