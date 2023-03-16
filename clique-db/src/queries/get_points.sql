-- Get the number of times every pair of users spoke to each other within given periods.
--
-- The number of times a pair of people spoke to each other in a given period is
-- referred to as their "points" for that period.
--
-- This query results in two columns:
--   - `period`, the start date of a period
--   - `data`, an array of `(user_1, user_2, points)` records
--
-- Rows are given in ascending chronological order. Pairs of users who did not talk to
-- each other during a given period are not included in results for that period.
--
-- There are four parameters for this query:
--   1. The period over which to aggregate points, eg. `"day"` or `"week"`. A full list
--      of valid values can be found [here][periods].
--   2. `NULL` or a guild ID - if present, only use data from that guild.
--   3. `NULL` or a timestamp - if present, only use data from after that time.
--   4. `NULL` or a timestamp - if present, only use data from before that time.
--
-- [periods]: https://www.postgresql.org/docs/current/functions-datetime.html#FUNCTIONS-DATETIME-TRUNC

SELECT
    period,
    ARRAY_AGG((user_1, user_2, points)) AS data
FROM (
         SELECT
             DATE_TRUNC($1, timestamp) AS period,
             GREATEST(author, reply_to) AS user_1,
             LEAST(author, reply_to) AS user_2,
             COUNT(*) AS points
         FROM (
                  SELECT
                      timestamp,
                      author,
                      LAG(author, 1) OVER (PARTITION BY channel ORDER BY timestamp) AS reply_to,
                      guild
                  FROM messages

                  UNION ALL

                  SELECT timestamp, author, reply_to, guild
                  FROM messages
                  WHERE reply_to IS NOT NULL
              ) AS message_pairs
         WHERE
                 author != reply_to
           AND ($2::BIGINT IS NULL OR guild = $2)
           AND ($3::TIMESTAMP IS NULL OR timestamp > $3)
           AND ($4::TIMESTAMP IS NULL OR timestamp < $4)
         GROUP BY user_1, user_2, period
     ) AS pair_points_per_period
GROUP BY period
ORDER BY period;
