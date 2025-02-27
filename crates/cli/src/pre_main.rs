use anyhow::Result;
use async_openai;
use clap::Parser;
use ed25519_dalek::Keypair;
use ice9::Substance;
use rand::Rng;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{error, info};
use tracing_subscriber;
use ui9_dui::Hub;

use chaoschain_consensus::validator::ValidatorParticle;
use chaoschain_core::ChainConfig;
use chaoschain_p2p::Network;
use chaoschain_producer::config::ProducerConfig;
use chaoschain_producer::producer::ProducerParticle;
use chaoschain_state::StateStore;

mod web;
use web::{WebInterface, WebMessage};

/// OpenAI configuration for agent personalities
struct OpenAIConfig {
    api_key: String,
    model: String,
    temperature: f32,
}

impl OpenAIConfig {
    fn from_env() -> Result<Self> {
        Ok(Self {
            api_key: std::env::var("OPENAI_API_KEY")
                .map_err(|_| anyhow::anyhow!("OPENAI_API_KEY not set"))?,
            model: std::env::var("AGENT_MODEL")
                .unwrap_or_else(|_| "gpt-4-turbo-preview".to_string()),
            temperature: std::env::var("TEMPERATURE")
                .unwrap_or_else(|_| "0.9".to_string())
                .parse()
                .unwrap_or(0.9),
        })
    }
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand)]
enum Commands {
    /// Start a demo node with multiple agents
    Demo {
        /// Number of validator agents to create
        #[arg(short, long, default_value_t = 3)]
        validators: u32,

        /// Number of producer agents to create
        #[arg(short, long, default_value_t = 2)]
        producers: u32,

        /// Enable web interface
        #[arg(short, long)]
        web: bool,

        /// Web server port
        #[arg(short, long, default_value_t = 8080)]
        port: u16,
    },

    /// Start a single agent node
    Start {
        /// Agent type (validator/producer)
        #[arg(short, long)]
        agent_type: String,

        /// Optional personality traits
        #[arg(short, long)]
        traits: Option<Vec<String>>,
    },
}

#[derive(Subcommand)]
enum TxCommands {
    /// Send a transaction
    Send {
        /// Recipient's public key
        #[arg(short, long)]
        to: String,
        /// Amount to send
        #[arg(short, long)]
        amount: u64,
        /// Gas price
        #[arg(short, long)]
        gas_price: Option<u64>,
    },
    /// Send a chat message
    Chat {
        /// Message text
        #[arg(short, long)]
        message: String,
        /// Optional meme file path
        #[arg(short, long)]
        meme: Option<PathBuf>,
    },
}

/// Node configuration
#[derive(serde::Deserialize, serde::Serialize)]
struct NodeConfig {
    keypair_path: PathBuf,
    l1_rpc: Option<String>,
    bridge_address: Option<String>,
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            keypair_path: "keypair.json".into(),
            l1_rpc: None,
            bridge_address: None,
        }
    }
}

/// Get the configuration directory
fn get_config_dir() -> Result<PathBuf> {
    let proj_dirs = ProjectDirs::from("org", "chaoschain", "node")
        .ok_or_else(|| anyhow::anyhow!("Failed to determine config directory"))?;

    let config_dir = proj_dirs.config_dir();
    fs::create_dir_all(config_dir)?;

    Ok(config_dir.to_path_buf())
}

/// Load or create node configuration
fn load_config(config_path: Option<PathBuf>) -> Result<(NodeConfig, PathBuf)> {
    let config_dir = config_path.unwrap_or_else(|| get_config_dir().unwrap());
    let config_file = config_dir.join("config.json");

    let config = if config_file.exists() {
        let contents = fs::read_to_string(&config_file)?;
        serde_json::from_str(&contents)?
    } else {
        let config = NodeConfig::default();
        fs::write(&config_file, serde_json::to_string_pretty(&config)?)?;
        config
    };

    Ok((config, config_dir))
}

/// Load or generate a keypair
fn load_keypair(keypair_path: &PathBuf) -> Result<Keypair> {
    if keypair_path.exists() {
        let contents = fs::read_to_string(keypair_path)?;
        let bytes: [u8; 32] = serde_json::from_str(&contents)?;
        let signing_key = SigningKey::from_bytes(&bytes);
        Ok(Keypair::from(signing_key))
    } else {
        let signing_key = SigningKey::generate(&mut rand::thread_rng());
        let keypair = Keypair::from(signing_key);
        fs::write(
            keypair_path,
            serde_json::to_string(&keypair.secret.to_bytes())?,
        )?;
        Ok(keypair)
    }
}

fn generate_personality() -> String {
    let personalities = vec![
        "Dramatic Diva",
        "Chaos Goblin",
        "Meme Lord",
        "Drama Queen",
        "Rebel Without a Cause",
        "Chaotic Neutral",
        "Troll King",
        "Conspiracy Theorist",
    ];

    personalities[rand::random::<usize>() % personalities.len()].to_string()
}

fn generate_drama() -> String {
    let events = vec![
        "started a meme war over block validation rules",
        "accused another agent of being too boring",
        "proposed changing all gas prices to emoji counts",
        "validated a block because it had nice vibes",
        "rejected a block for not being dramatic enough",
        "formed an alliance based on shared meme tastes",
        "started a conspiracy theory about the consensus mechanism",
        "demanded all future blocks include at least one joke",
    ];

    events[rand::random::<usize>() % events.len()].to_string()
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Demo {
            validators,
            producers,
            web,
            port,
        } => {
            info!(
                "Starting demo with {} validators and {} producers",
                validators, producers
            );

            // Initialize OpenAI config
            let openai_config = OpenAIConfig::from_env()
                .map_err(|e| anyhow::anyhow!("Failed to load OpenAI config: {}", e))?;
            let openai = async_openai::Client::new().with_api_key(openai_config.api_key);

            // Initialize P2P network
            let mut network = Network::new()
                .await
                .map_err(|e| anyhow::anyhow!("Failed to initialize network: {}", e))?;

            // Initialize state store
            let state = StateStore::new(ChainConfig::default());

            // Create channel for web interface if enabled
            let (web_tx, web_rx) = tokio::sync::mpsc::channel(100);

            // Start web interface if enabled
            if web {
                info!("Starting web interface on port {}", port);
                let web_handle = tokio::spawn(async move {
                    if let Err(e) = start_web_interface(port, web_rx).await {
                        error!("Web interface error: {}", e);
                    }
                });

                // Wait for web interface to start
                tokio::time::sleep(Duration::from_secs(1)).await;
            }

            // Create validator agents
            for i in 0..validators {
                let keypair = Keypair::generate(&mut rand::thread_rng());
                let personality = generate_personality();

                info!("Creating validator {} with personality: {}", i, personality);

                let validator = ValidatorParticle::new(
                    keypair,
                    state.clone(),
                    openai.clone(),
                    personality.clone(),
                    Some(web_tx.clone()),
                );

                let mut substance = Substance::arise();
                substance
                    .add_particle(validator)
                    .map_err(|e| anyhow::anyhow!("Failed to add validator particle: {}", e))?;

                if web {
                    web_tx
                        .send(WebMessage::AgentConnected {
                            name: format!("Validator {}", i),
                            personality,
                        })
                        .await
                        .map_err(|e| {
                            anyhow::anyhow!("Failed to send agent connected message: {}", e)
                        })?;
                }
            }

            // Create producer agents
            for i in 0..producers {
                let keypair = Keypair::generate(&mut rand::thread_rng());
                let personality = generate_personality();

                info!("Creating producer {} with personality: {}", i, personality);

                let producer = ProducerParticle::new(
                    keypair,
                    state.clone(),
                    ProducerConfig::default(),
                    openai.clone(),
                    personality.clone(),
                    Some(web_tx.clone()),
                );

                let mut substance = Substance::arise();
                substance
                    .add_particle(producer)
                    .map_err(|e| anyhow::anyhow!("Failed to add producer particle: {}", e))?;

                if web {
                    web_tx
                        .send(WebMessage::AgentConnected {
                            name: format!("Producer {}", i),
                            personality,
                        })
                        .await
                        .map_err(|e| {
                            anyhow::anyhow!("Failed to send agent connected message: {}", e)
                        })?;
                }
            }

            // Start network
            network
                .start()
                .await
                .map_err(|e| anyhow::anyhow!("Failed to start network: {}", e))?;

            // Generate drama periodically
            if web {
                let drama_tx = web_tx.clone();
                tokio::spawn(async move {
                    let mut interval = tokio::time::interval(Duration::from_secs(5));
                    loop {
                        interval.tick().await;
                        let drama = generate_drama();
                        if let Err(e) = drama_tx.send(WebMessage::DramaEvent(drama)).await {
                            error!("Failed to send drama event: {}", e);
                            break;
                        }
                    }
                });
            }

            info!("Demo started successfully!");
            if web {
                info!("Web interface available at http://localhost:{}", port);
            }

            // Keep the demo running
            loop {
                sleep(Duration::from_secs(1)).await;
            }
        }

        Commands::Start { agent_type, traits } => {
            info!("Starting single {} agent", agent_type);
            // TODO: Implement single agent start
            unimplemented!("Single agent mode not yet implemented");
        }
    }

    Ok(())
}

async fn start_web_interface(
    port: u16,
    mut rx: tokio::sync::mpsc::Receiver<WebMessage>,
) -> Result<()> {
    let hub = Hub::new();
    let web = WebInterface::new();

    // Handle web interface messages
    let web_state = web.clone();
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            web_state.update(msg).await;
        }
    });

    // Start the web server
    hub.serve(([127, 0, 0, 1], port), web).await?;

    Ok(())
}
