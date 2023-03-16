//! The [`GetPoints`] query and the types it returns.
use crate::{Database, DateTime, DbResult};
use fallible_iterator::FallibleIterator;
use std::error::Error;
use tokio_postgres::{
    types::{FromSql, Type},
    Row,
};

/// A query which retrieves the "points" for each pair of users, aggregated over given lengths of
/// time.
///
/// Each pair of users get a point each time they talk to each other - we consider that a user talks
/// to another if they send a message directly after the other user, in the same channel, or if they
/// explicitly reply to the other user's message.
///
/// Points are counted for every pair of users per time period. The pair of users (A, B) is
/// the same as the pair (B, A) - to enforce this, the first user in the pair is always the one with
/// the lower user ID (and hence older account).
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
pub struct GetPoints {
    /// The length of time over which to aggregate points.
    pub period: TimePeriod,
    /// The guild to filter by, if any.
    pub guild: Option<u64>,
    /// If set, only include messages sent after this time.
    pub after: Option<DateTime>,
    /// If set, only include messages sent before this time.
    pub before: Option<DateTime>,
}

impl GetPoints {
    /// Run the query, returning the results.
    ///
    /// # Errors
    ///
    /// If there is an unexpected error communicating with the Postgres server.
    pub async fn run(&self, db: &Database) -> DbResult<Vec<PeriodData>> {
        db.client
            .query(
                &db.get_points,
                &[
                    &self.period.to_string(),
                    &self.guild.map(|g| g as i64),
                    &self.after.map(|t| t.naive_utc()),
                    &self.before.map(|t| t.naive_utc()),
                ],
            )
            .await
            .map(|rows| rows.into_iter().map(PeriodData::from).collect())
    }
}

/// Points per user pair over a certain time period.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct PeriodData {
    /// The start of the relevant time period.
    pub start: DateTime,
    /// The points for each user pair.
    pub pairs: Vec<PeriodUserPoints>,
}

impl From<Row> for PeriodData {
    fn from(row: Row) -> Self {
        let naive_start: chrono::NaiveDateTime = row.get(0);
        let points: PeriodUserPointsVec = row.get(1);
        Self {
            start: DateTime::from_utc(naive_start, chrono::Utc),
            pairs: points.0,
        }
    }
}

/// A new type wrapper around a [`Vec<PeriodUserPoints>`] which implements [`FromSql`].
/// Not sure why, but implementing [`FromSql`] on [`PeriodUserPoints`] directly didn't work.
struct PeriodUserPointsVec(Vec<PeriodUserPoints>);

impl<'a> FromSql<'a> for PeriodUserPointsVec {
    fn from_sql(_ty: &Type, raw: &'a [u8]) -> Result<Self, Box<dyn Error + Sync + Send>> {
        let array = postgres_protocol::types::array_from_sql(raw)?;
        array
            .values()
            .iterator()
            .map(|value| match value {
                Ok(Some(value)) => Ok(PeriodUserPoints::from(value)),
                Ok(None) => Err("unexpected null value".into()),
                Err(e) => Err(e),
            })
            .collect::<Result<_, _>>()
            .map(Self)
    }

    fn accepts(ty: &Type) -> bool {
        matches!(ty, &Type::RECORD_ARRAY)
    }
}

/// The points for a pair of users over a certain time period.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct PeriodUserPoints {
    /// The ID of the first user in the pair. To ensure that pairs of users are only counted once,
    /// this is always the user with the lower ID.
    pub user1: u64,
    /// The ID of the second user in the pair.
    pub user2: u64,
    /// The number of times this pair of users talked to each other during the relevant period.
    pub points: u64,
}

fn array_slice<const LEN: usize, T: Copy>(slice: &[T], offset: usize) -> [T; LEN] {
    std::array::from_fn(|i| slice[offset + i])
}

fn read_u64(slice: &[u8], offset: usize) -> u64 {
    u64::from_be_bytes(array_slice(slice, offset))
}

impl From<&[u8]> for PeriodUserPoints {
    fn from(value: &[u8]) -> Self {
        // I couldn't find documentation on the binary format used here, so the comments below are
        // just guesses based on observation.
        // 0..4: the number of fields (3)
        // 4..8: the type of the first field (20)
        // 8..12: the length of the first field (8)
        let user1 = read_u64(value, 12);
        // 20..24: the type of the second field (20)
        // 24..28: the length of the second field (8)
        let user2 = read_u64(value, 28);
        // 36..40: the type of the third field (20)
        // 40..44: the length of the third field (8)
        let points = read_u64(value, 44);
        Self {
            user1,
            user2,
            points,
        }
    }
}

/// The length of time over which to aggregate points.
///
/// The full range of possible values is included for completeness, but values less than second or
/// greater than year are unlikely to be useful.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
#[allow(missing_docs)]
pub enum TimePeriod {
    Microsecond,
    Millisecond,
    Second,
    Minute,
    Hour,
    Day,
    Week,
    Month,
    Quarter,
    Year,
    Decade,
    Century,
    Millennium,
}

impl TimePeriod {
    const fn to_string(self) -> &'static str {
        match self {
            Self::Microsecond => "microsecond",
            Self::Millisecond => "millisecond",
            Self::Second => "second",
            Self::Minute => "minute",
            Self::Hour => "hour",
            Self::Day => "day",
            Self::Week => "week",
            Self::Month => "month",
            Self::Quarter => "quarter",
            Self::Year => "year",
            Self::Decade => "decade",
            Self::Century => "century",
            Self::Millennium => "millennium",
        }
    }
}
