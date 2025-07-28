# Contributing to Good First Bot

First off, thank you for considering contributing to Good First Bot! It's people like you that make this project great.

## Code of Conduct

This project and everyone participating in it is governed by the [Code of Conduct](CODE_OF_CONDUCT.md). By participating, you are expected to uphold this code.

## How to Contribute

There are many ways to contribute, from writing code and documentation to reporting bugs and suggesting enhancements. Every contribution is welcome!

## Getting Started

To get started with development, you'll need to set up the project on your local machine.

1.  **Fork the repository** on GitHub.
2.  **Clone your fork** locally:
    ```bash
    git clone https://github.com/your-username/good-first-bot-rs.git
    cd good-first-bot-rs
    ```
3.  **Configure `upstream` remote**:
    Add the original repository as an `upstream` remote to keep your fork synced with the main project.
    ```bash
    git remote add upstream https://github.com/isserge/good-first-bot-rs.git
    ```
4.  **Install Rust and `sqlx-cli`**:
    Follow the instructions in the [README.md](README.md) to install the necessary tools.
5.  **Set up environment variables**:
    Create a `.env` file from the `.env.example` and fill in the required values.
    ```bash
    cp .env.example .env
    ```
6.  **Set up the database**:
    ```bash
    mkdir data
    sqlx database setup
    ```
7.  **Build the project**:
    ```bash
    cargo build
    ```

## Keeping Your Fork Synced

Before starting a new feature, you should sync your fork with the latest changes from the `upstream` repository.

1.  Fetch the latest changes from `upstream`:
    ```bash
    git fetch upstream
    ```
2.  Check out your `main` branch:
    ```bash
    git checkout main
    ```
3.  Rebase your `main` branch with `upstream/main`:
    ```bash
    git rebase upstream/main
    ```
4.  Push the changes to your fork:
    ```bash
    git push origin main
    ```

## Development Workflow

All contributions should be submitted via a Pull Request (PR) from a feature branch in your fork.

1.  **Create a feature branch** in your fork for your feature or bug fix:
    ```bash
    git checkout -b my-new-feature
    ```
2.  **Make your changes** and commit them with a descriptive message.
3.  **Run the tests** to ensure everything is working correctly:
    ```bash
    cargo test
    ```
4.  **Format your code** to maintain a consistent style:
    ```bash
    cargo fmt
    ```
5.  **Lint your code** to catch common mistakes:
    ```bash
    cargo clippy
    ```
6.  **Push your feature branch** to your fork:
    ```bash
    git push origin my-new-feature
    ```
7.  **Create a Pull Request** from your feature branch to the `main` branch of the original repository.

## Pull Request Guidelines

-   Your PR should have a clear, descriptive title.
-   The PR description should provide a detailed overview of the changes.
-   Reference the issue your PR resolves (e.g., `Closes #123`).
-   Ensure all tests and checks are passing before submitting.

## Commit Message Conventions

This project uses [Conventional Commits](https://www.conventionalcommits.org/) for commit messages. This makes the project history more readable and allows for automated changelog generation.

Examples:
-   `feat: Add user authentication`
-   `fix: Correct pagination bug`
-   `docs: Update CONTRIBUTING.md`
-   `style: Fix indentation in main.rs`
-   `refactor: Improve performance of GitHub API calls`
-   `test: Add tests for the new feature`
-   `chore: Update dependencies`

## Coding Standards

- Use **Rust 2024 edition**.
- Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/).
- Format code with `rustfmt`:

  ```sh
  rustup component add rustfmt
  cargo fmt
  ```

- Lint code with `clippy`:

  ```sh
  cargo clippy --all-targets --all-features
  ```

## Testing

Testing is the responsibility of all contributors as such all contributions must pass existing tests and include new tests when applicable:

1. Write tests for new features or bug fixes.
2. Run the test suite:

   ```sh
   cargo test
   ```

3. Ensure no warnings or errors.

## Reporting Bugs

If you find a bug, please open an issue on GitHub. Please include as much detail as possible, including:
- A clear and descriptive title.
- Steps to reproduce the bug.
- The expected behavior.
- The actual behavior.
- Your environment (OS, Rust version, etc.).

## Suggesting Enhancements

If you have an idea for a new feature or an improvement to an existing one, please open an issue on GitHub. Please include:
- A clear and descriptive title.
- A detailed description of the proposed enhancement.
- Any relevant mockups or examples.
