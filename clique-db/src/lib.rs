//! A library for sending queries to the Clique database.
//!
//! # Example
//!
//! ```no_run
//! use clique_db::{GetPoints, TimePeriod, PeriodData, PeriodUserPoints, Database, Message};
//! # use clique_db::DbResult;
//! use chrono::Utc;
//!
//! # #[tokio::main]
//! # async fn main() -> DbResult<()> {
//! // Connect to the database and prepare statements.
//! let db = Database::new("postgres://localhost/clique").await?;
//!
//! // Create some users.
//! db.insert_user(123, "My Username").await?;
//! db.insert_user(456, "Your Username").await?;
//!
//! // Check the username is correct.
//! assert_eq!(db.get_user(123).await?, Some("My Username".to_string()));
//!
//! // A message can be inserted by calling `Message::insert`...
//! Message {
//!     id: 888,
//!     guild: 222,
//!     author: 123,
//!     channel: 777,
//!     reply_to: Some(456),
//!     timestamp: Utc::now(),
//! }.insert(&db).await?;
//!
//! // ...or by calling the `Database::insert_message` method directly.
//! db.insert_message(&Message {
//!     id: 999,
//!     guild: 222,
//!     author: 456,
//!     channel: 777,
//!     reply_to: None,
//!     timestamp: Utc::now(),
//! }).await?;
//!
//! // See the documentation on `GetPoints` for more information on the data it provides.
//! let data = GetPoints {
//!     period: TimePeriod::Week,
//!     guild: Some(222),
//!     after: None,
//!     before: Some(Utc::now()),
//! }.run(&db).await?;
//!
//! // We should see that our two users spoke to each other twice.
//! for PeriodData { start, pairs } in data {
//!     println!("During the week beginning {start}:");
//!     for PeriodUserPoints { user1, user2, points } in pairs {
//!         println!("  User ID {user1} and user ID {user2} spoke {points} times");
//!     }
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # Features
//!
//! The `serde` feature enables deserialization of query types and serialization of response types.
//!
//! There is also a feature for each query, allowing you to only compile in the queries you need.
//! These are all enabled by default, so you must use `default-features = false` to disable them,
//! and then enable the queries you need. The query features are:
//! - `q_get_points`, which enables the [`GetPoints`] query.
//! - `q_get_user`, which enables the [`Database::get_user`] method.
//! - `q_insert_message`, which enables the [`Message`] query and [`Database::insert_message`] method.
//! - `q_insert_user`, which enables the [`Database::insert_user`] method.
#![warn(clippy::all, clippy::pedantic, clippy::nursery, missing_docs)]
// We encode Discord snowflakes as i64s in the database, because that's what PostgreSQL's `BIGINT`
// type is. This does mean that we might end up with negative numbers, but that's fine because we
// cast them back to u64s when we retrieve them.
#![allow(clippy::cast_possible_wrap)]

pub use tokio_postgres::Error;
use tokio_postgres::{connect, Client, NoTls, Statement};

/// A type alias for the result of a database query.
pub type DbResult<T> = Result<T, Error>;

/// A type alias for a UTC timestamp.
pub type DateTime = chrono::DateTime<chrono::Utc>;

#[cfg(feature = "q_get_points")]
mod get_points;

#[cfg(feature = "q_get_points")]
pub use get_points::{GetPoints, PeriodData, PeriodUserPoints, TimePeriod};

/// The database client, including prepared statements.
///
/// This struct should ideally be created once and long-lived.
pub struct Database {
    pub(crate) client: Client,
    #[cfg(feature = "q_get_points")]
    pub(crate) get_points: Statement,
    #[cfg(feature = "q_get_user")]
    pub(crate) get_user: Statement,
    #[cfg(feature = "q_insert_message")]
    pub(crate) insert_message: Statement,
    #[cfg(feature = "q_insert_user")]
    pub(crate) insert_user: Statement,
}

async fn init_db(db_url: &str) -> DbResult<Client> {
    let (client, connection) = connect(db_url, NoTls).await?;
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {e}");
        }
    });
    client
        .execute(include_str!("queries/user_table.sql"), &[])
        .await?;
    client
        .execute(include_str!("queries/message_table.sql"), &[])
        .await?;
    Ok(client)
}

impl Database {
    /// Connect to the database and create tables if they don't exist.
    ///
    /// `db_url` should be a connection string in the format
    /// `postgres://user:password@host:port/database`.
    ///
    /// # Errors
    ///
    /// If the connection URL is invalid, the database cannot be connected to, or the tables cannot
    /// be created.
    pub async fn new(db_url: &str) -> DbResult<Self> {
        let client = init_db(db_url).await?;
        Ok(Self {
            #[cfg(feature = "q_get_points")]
            get_points: client
                .prepare(include_str!("queries/get_points.sql"))
                .await?,
            #[cfg(feature = "q_get_user")]
            get_user: client.prepare(include_str!("queries/get_user.sql")).await?,
            #[cfg(feature = "q_insert_message")]
            insert_message: client
                .prepare(include_str!("queries/insert_message.sql"))
                .await?,
            #[cfg(feature = "q_insert_user")]
            insert_user: client
                .prepare(include_str!("queries/insert_user.sql"))
                .await?,
            // This field goes last because it moves `client`, which is used in the other field
            // initializers.
            client,
        })
    }

    #[cfg(feature = "q_get_user")]
    /// Get a user's name from the database.
    ///
    /// # Errors
    ///
    /// If the query fails.
    pub async fn get_user(&self, user_id: u64) -> DbResult<Option<String>> {
        let row = self
            .client
            .query_opt(&self.get_user, &[&(user_id as i64)])
            .await?;
        Ok(row.map(|row| row.get(0)))
    }

    #[cfg(feature = "q_insert_user")]
    /// Insert or update a user's name into the database.
    ///
    /// # Errors
    ///
    /// If the query fails.
    pub async fn insert_user(&self, user_id: u64, name: &str) -> DbResult<()> {
        self.client
            .execute(&self.insert_user, &[&(user_id as i64), &name])
            .await?;
        Ok(())
    }

    #[cfg(feature = "q_insert_message")]
    /// Insert a message into the database.
    ///
    /// # Errors
    ///
    /// If the query fails.
    pub async fn insert_message(&self, message: &Message) -> DbResult<()> {
        self.client
            .execute(
                &self.insert_message,
                &[
                    &(message.id as i64),
                    &(message.guild as i64),
                    &(message.author as i64),
                    &(message.channel as i64),
                    &message.reply_to.map(|id| id as i64),
                    &message.timestamp.naive_utc(),
                ],
            )
            .await?;
        Ok(())
    }
}

/// A Discord message to be inserted into the database.
#[cfg(feature = "q_insert_message")]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
pub struct Message {
    /// The message's ID.
    pub id: u64,
    /// The ID of the guild the message was sent in.
    pub guild: u64,
    /// The ID of the user who sent the message.
    pub author: u64,
    /// The ID of the channel the message was sent in.
    pub channel: u64,
    /// The ID of the author of the message that this message is a reply to, if any.
    pub reply_to: Option<u64>,
    /// The time the message was sent.
    pub timestamp: DateTime,
}

#[cfg(feature = "q_insert_message")]
impl Message {
    /// Insert the message into the database.
    ///
    /// # Errors
    ///
    /// If the query fails.
    pub async fn insert(&self, db: &Database) -> DbResult<()> {
        db.insert_message(self).await
    }
}
