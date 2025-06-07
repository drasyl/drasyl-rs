use clap::{Parser, Subcommand};
use drasyl::identity::Identity;
use drasyl::util;
use drasyl_sdn::node::SdnNode;
use drasyl_sdn::rest_api::{RestApiClient, RestApiServer};
use http_body_util::BodyExt;
use std::sync::Arc;
use tokio::signal;
use tracing::info;

#[derive(Parser, Debug)]
#[command(name = "drasyl-sdn")]
#[command(about = "An SDN client for the drasyl network")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Runs the SDN node
    Run {
        #[arg(num_args = 1..)]
        urls: Vec<String>,
    },
    /// Shows the status of the running SDN node
    Status,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Run { urls } => run_sdn_node(urls).await,
        Commands::Status => show_status().await,
    }
}

async fn run_sdn_node(
    urls: Vec<String>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    // options
    let identity_file = util::get_env("IDENTITY_FILE", "drasyl.identity".to_string());
    let min_pow_difficulty = util::get_env("MIN_POW_DIFFICULTY", 24);

    // identity
    let id = Identity::load_or_generate(&identity_file, min_pow_difficulty)
        .expect("Failed to load identity");
    info!("I am {}", id.pk);

    let node = Arc::new(SdnNode::start(id, urls).await);
    let rest_api = RestApiServer::new(node.clone());

    let node_clone = node.clone();

    tokio::select! {
        biased;
        _ = async {
            #[cfg(unix)]
            {
                let mut sigterm = signal::unix::signal(signal::unix::SignalKind::terminate())?;
                let mut sigint = signal::unix::signal(signal::unix::SignalKind::interrupt())?;
                tokio::select! {
                    _ = sigterm.recv() => {
                        info!("Shutdown initiated via SIGTERM.");
                    }
                    _ = sigint.recv() => {
                        info!("Shutdown initiated via SIGINT.");
                    }
                }
            }
            #[cfg(not(unix))]
            {
                signal::ctrl_c().await?;
                info!("Shutdown initiated via Ctrl+C.");
            }
            Ok::<_, std::io::Error>(())
        } => {
            node_clone.shutdown().await;
        }
        _ = rest_api.bind() => {}
        _ = node.cancelled() => {},
    }

    Ok(())
}

async fn show_status() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let client = RestApiClient::new();

    match client.status().await {
        Ok(status) => {
            println!("{}", status);
        }
        Err(e) => {
            eprintln!("Failed to retrieve status: {e}");
            std::process::exit(1);
        }
    }

    Ok(())
}
