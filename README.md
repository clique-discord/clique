# Clique

Clique is a project aiming to generate interesting statistics and visualizations by looking at the way users interact
with each other on [Discord](https://discord.com/). It is inspired by [PieSpy](https://github.com/mchelen/piespy), a
similar project for IRC.

## Architecture

Clique is written in Rust. Currently, it consists of two parts:
- The Clique collector, a Discord bot which receives message events from Discord and stores them in a PostgreSQL
  database.
- The Clique API, a web server which provides an API for accessing aggregated statistics from the database.

In the future, there will also be a frontend, which will be a web application that uses the API to display
visualizations in the browser.

Both the collector (found in `clique-collector`) and the API (`clique-api`) make use of `clique-db`, a Rust library for
interacting with the database.

## Installation

First, you should have a PostgreSQL database set up, and a Discord application with a bot account created.

Binaries for the collector and the API are available on the releases page for Clique on GitHub. You can download these,
or use Cargo to build them yourself.

Both the collector and the API read configuration from a file called `config.toml` in the current working directory. It
is possible to use the same file for both, in which case it should look like this:

```toml
# A postgres connection string (for the collector and API).
postgres_url = "postgres://user:password@localhost/clique"

# The Discord bot token (for the API).
discord_token = "your-discord-bot-token"

# The address to bind to (for the API).
bind_address = "127.0.0.1"

# The port to listen on (for the API).
port = 8080
```
