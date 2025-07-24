use good_first_bot::run;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();
    // Initialize the tracing subscriber
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new(format!("{}={}", module_path!(), "info")))
                .add_directive(format!("dlq_log={}", "error").parse()?),
        )
        .init();

    if let Err(err) = run().await {
        tracing::error!("Error: {err}");
        std::process::exit(1);
    }

    Ok(())
}
