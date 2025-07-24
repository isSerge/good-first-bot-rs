# Good First Bot

A Telegram bot for tracking GitHub issues. The bot allows users to easily add,
remove, and list repositories they want to track, select specific labels like
“good first issue”, "enhancement", "bug", etc. It periodically polls GitHub and
notifies users of new issues.

![Good First Bot](good-first-bot.png)

## Features

- **Track GitHub Repositories:**  
  Add or remove repositories to receive notifications

- **Select specific issue labels:**  
  Users can select specific labels for each added repository

- **GitHub Integration:**  
  Uses the GitHub GraphQL API to verify repository existence and fetch issues
  with specific labels.

- **Telegram Bot Commands:**  
  Supports commands like `/start`, `/help`, `/add`, and `/list` to interact with
  the bot.

- **Polling Mechanism:**  
  Periodically polls tracked repositories to find new issues and sends
  notifications via Telegram.

- **User-based Limits:** Configurable limits on the number of repositories a
  user can track and the number of labels per repository to ensure fair usage

- **SQLite Storage:**  
  Persists repository tracking and polling states using SQLite, with automatic
  migrations on startup.

- **Asynchronous and Modular:**  
  Built using Tokio and Teloxide for async execution and organized into modules
  (configuration, bot handler, GitHub client, repository storage, and
  messaging).

- **Dockerized:** Provides `Dockerfile` and `docker-compose.yml` for easy
  containerized deployment and development.

## Project Structure

```plaintext
Cargo.toml
Dockerfile                # For building the Docker image
docker-compose.yml        # For running with Docker Compose
migrations
src
  ├── bot_handler         # Telegram bot commands and handlers
  │   └── commands        # Individual command implementations (add, list, help, etc.)
  |   └── callbacks       # Handlers for callback queries (view repo, toggle label, etc.)
  ├── config.rs           # Environment-based configuration
  ├── dispatcher.rs       # Dispatcher setup for handling Telegram updates
  ├── github              # GitHub API integration using GraphQL
  │   ├── github.graphql
  │   └── schema.graphql
  ├── main.rs             # Application entry point
  ├── messaging           # Messaging service for Telegram
  ├── poller              # Periodic polling of GitHub issues
  ├── repository          # Repository service
  └── storage             # SQLite-based storage
```

## Installation

1. **Clone the repository:**

   ```bash
   git clone https://github.com/your-username/good-first-bot-rs.git
   cd good-first-bot-rs
   ```

2. **Prerequisites:**

   - **Rust:** Ensure you have [Rust](https://www.rust-lang.org/tools/install) installed.
   - **sqlx-cli:** Install the SQLx command-line tool for database migrations.
     ```bash
     cargo install sqlx-cli
     ```

3. **Set up environment variables:**

Create a .env file in the project root with the following keys (values are
examples):

```bash
# Required
GITHUB_TOKEN=your_github_token_here
TELOXIDE_TOKEN=your_telegram_bot_token_here

# Optional
GITHUB_GRAPHQL_URL=https://api.github.com/graphql
POLL_INTERVAL=10
DATABASE_URL=sqlite:data/data.db
MAX_REPOS_PER_USER=10
MAX_LABELS_PER_REPO=5
```

- GITHUB_TOKEN: Your GitHub personal access token.
- TELOXIDE_TOKEN: Your Telegram bot token obtained from
  [BotFather](https://t.me/botfather).
- GITHUB_GRAPHQL_URL: (Optional) Defaults to https://api.github.com/graphql.
- POLL_INTERVAL: (Optional) Poll interval in seconds. Default is 10.
- DATABASE_URL: (Optional) Database URL for SQLite. Default is
  `sqlite:data/data.db`.
- MAX_REPOS_PER_USER: (Optional) Maximum number of repositories a user can
  track. Default is 20.
- MAX_LABELS_PER_REPO: (Optional) Maximum number of labels per repository a user
  can track. Default is 10

4. **Database Setup:**

Run the following command to set up the database and apply migrations:

```bash
mkdir data
sqlx database setup
```

5. **Install Dependencies:**

In the project directory run:

```bash
cargo build
```

## Running the bot

To run the bot, simply execute:

```bash
cargo run --release
```

## Testing

To run tests:

```bash
cargo test
```

## Generate Test Coverage Report

_Interactive HTML Report_

```sh
RUST_TEST_THREADS=1 cargo +stable llvm-cov --html --open
```

_CLI Report_

```sh
RUST_TEST_THREADS=1 cargo +stable llvm-cov
```

## Running with Docker

This project includes a `Dockerfile` and a `docker-compose.yml` for easy
management. In order to build and run with Docker Compose:

1. Create a data directory on your host machine: The `docker-compose.yml` is
   configured to mount a host directory for SQLite database persistence.

```bash
mkdir data
```

2. Build and start the container: In the project root (where
   `docker-compose.yml` is):

```bash
docker compose up --build
# or docker compose up --build -d (in detached background mode)
```

3. View logs: If running in detached mode or from another terminal:

```bash
docker compose logs -f bot
```

(Assuming your service in `docker-compose.yml` is using default name `bot`).

4. Stop the container:

```bash
docker compose down
```

## License

Distributed under the MIT License. See [LICENSE](LICENSE) for more information.
