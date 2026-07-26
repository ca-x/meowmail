use anyhow::{Context, Result};
use meowmail::{AppState, build_router, config::Config};
use tokio::net::TcpListener;
use tracing::info;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    if std::env::args_os()
        .nth(1)
        .is_some_and(|argument| argument == "--version" || argument == "-V")
    {
        println!("meowmail {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "meowmail=info,tower_http=info".into()),
        )
        .init();

    let config = Config::from_env().context("invalid Meowmail configuration")?;
    let address = config.bind;
    let state = AppState::initialize(config)
        .await
        .context("failed to initialize Meowmail")?;
    let listener = TcpListener::bind(address)
        .await
        .with_context(|| format!("failed to bind Meowmail to {address}"))?;

    info!(%address, version = env!("CARGO_PKG_VERSION"), "Meowmail listening");
    axum::serve(listener, build_router(state))
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("Meowmail server failed")
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };
    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
}
