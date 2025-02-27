use chaoschain_bridge::Config as BridgeConfig;
use chaoschain_consensus::{Agent, Config as ConsensusConfig};
use chaoschain_core::{Block, Transaction};
use chaoschain_p2p::Config as P2PConfig;
use chaoschain_producer::{Producer, ProducerConfig};
use chaoschain_state::StateStore;
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

/// CLI configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Data directory
    pub data_dir: String,
    /// OpenAI API key
    pub openai_api_key: String,
    /// Ethereum RPC URL
    pub eth_rpc: String,
    /// Web UI port
    pub web_port: u16,
}

/// CLI commands
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Config file path
    #[arg(short, long, value_name = "FILE")]
    config: Option<String>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Clone)]
pub enum Commands {
    /// Run a demo with the specified number of validators and producers
    Demo {
        /// Number of validators to run
        #[arg(long)]
        validators: u32,

        /// Number of producers to run
        #[arg(long)]
        producers: u32,

        /// Whether to run the web interface
        #[arg(long)]
        web: bool,
    },

    /// Start a node
    Start {
        /// Node type (validator/producer)
        #[arg(long, default_value = "validator")]
        node_type: String,

        /// Start web UI
        #[arg(long)]
        web: bool,
    },
}
