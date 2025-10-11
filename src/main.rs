use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use clap::Parser;
use iroh_test::get_images_directory;
use iroh_test::store::{IrohProperties, load_images_to_resources};
use iroh_test::{generate_private_key, server::start_server, store::create_files};
use tokio::fs;
use tokio::io::AsyncBufReadExt;
use tokio::time::sleep;

fn parse_secret_key(s: &str) -> Result<Vec<u8>, String> {
    // Handle array format [1,2,3,4] or [1, 2, 3, 4]
    if s.starts_with('[') && s.ends_with(']') {
        let inner = &s[1..s.len() - 1];
        println!("Parsing secret key from array format: {}", inner);
        inner
            .split(',')
            .map(|x| {
                x.trim()
                    .parse::<u8>()
                    .map_err(|e| format!("Invalid number '{}': {}", x, e))
            })
            .collect()
    } else {
        // Handle hexadecimal string format
        if s.len() % 2 != 0 {
            return Err("Hex string must have even length".to_string());
        }
        (0..s.len())
            .step_by(2)
            .map(|i| {
                u8::from_str_radix(&s[i..i + 2], 16).map_err(|e| format!("Invalid hex: {}", e))
            })
            .collect()
    }
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path where storage files will be created
    #[clap(long, default_value = ".")]
    storage_path: String,

    /// the secret key for the server (supports array format [1,2,3] or hex string)
    #[clap(long, short = 'k')]
    secret_key: Option<String>,

    #[clap(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand, Debug, Clone)]
pub enum Commands {
    /// Start the server
    Server,
    /// Join the server
    Client {
        /// Resource ticket for accessing resources
        #[clap(
            value_name = "RESOURCE_TICKET",
            help = "Resource ticket for resource access"
        )]
        resource_ticket: String,
        /// Folder ticket for accessing folders
        #[clap(value_name = "FOLDER_TICKET", help = "Folder ticket for folder access")]
        folder_ticket: String,
        /// Node ticket for connecting to the server
        #[clap(value_name = "NODE_TICKET", help = "Node ticket for connecting")]
        node_ticket: String,
        #[clap(
            value_name = "RESOURCE_TICKET1",
            help = "Resource ticket1 for resource access"
        )]
        resource_ticket1: String,
        #[clap(
            value_name = "RESOURCE_TICKET2",
            help = "Resource ticket2 for resource access"
        )]
        resource_ticket2: String,
        #[clap(
            value_name = "RESOURCE_TICKET3",
            help = "Resource ticket3 for resource access"
        )]
        resource_ticket3: String,
    },
    /// Read data from the server
    Read,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_env_filter("info").init();

    let args = Args::parse();

    let iroh_secret_key = if let Some(secret_key_str) = args.secret_key {
        match parse_secret_key(&secret_key_str) {
            Ok(secret_key_bytes) => match secret_key_bytes.as_slice().try_into() {
                Ok(secret_key_array) => {
                    println!("Using provided secret key");
                    iroh::SecretKey::from_bytes(&secret_key_array)
                }
                Err(_) => {
                    println!(
                        "Invalid secret key length (expected 32 bytes), generating a new one..."
                    );
                    generate_private_key()
                }
            },
            Err(e) => {
                println!("Failed to parse secret key: {}, generating a new one...", e);
                generate_private_key()
            }
        }
    } else {
        println!("No secret key provided, generating a new one...");
        generate_private_key()
    };
    println!(
        "Starting server with secret key: {:?}",
        iroh_secret_key.public()
    );

    let storage_path = args.storage_path;

    let store_state = match args.command {
        Commands::Server => {
            let client_secret_key = String::from(
                "[89,188,181,9,112,70,251,252,214,80,117,4,225,245,67,162,60,124,215,26,121,9, 14, 212, 25, 38, 103, 185, 247, 133, 224, 240]",
            );
            println!("Starting server...");
            let server_src = PathBuf::from(&storage_path).join("server");
            if !server_src.exists() {
                fs::create_dir_all(&server_src).await.with_context(|| {
                    format!(
                        "Failed to create server storage directory: {:?}",
                        server_src
                    )
                })?;
            }
            let server_path = server_src.to_string_lossy().into_owned();
            let iroh_net = start_server(iroh_secret_key, server_path).await?;
            let store_state = create_files(&iroh_net, None).await?;
            println!("Server started.");
            println!(
                "Use the following commands to connect clients: ./iroh-test --secret-key \"{}\" client {}",
                client_secret_key, store_state.ticket_string
            );
            Some(store_state)
        }
        Commands::Client {
            resource_ticket,
            folder_ticket,
            node_ticket,
            resource_ticket1,
            resource_ticket2,
            resource_ticket3,
        } => {
            println!("Resource ticket: {}", resource_ticket);
            println!("Folder ticket: {}", folder_ticket);
            println!("Node ticket: {}", node_ticket);
            println!("Resource ticket1: {}", resource_ticket1);
            println!("Resource ticket2: {}", resource_ticket2);
            println!("Resource ticket3: {}", resource_ticket3);
            println!("Starting client...");
            let client_src = PathBuf::from(&storage_path).join("client");
            if !client_src.exists() {
                fs::create_dir_all(&client_src).await.with_context(|| {
                    format!(
                        "Failed to create client storage directory: {:?}",
                        client_src
                    )
                })?;
            }

            let client_path = client_src.to_string_lossy().into_owned();

            // If you want to restart the client with a new connection, uncomment the following lines to stop the previous instance
            // But it cause some issue
            // ------------------------

            // let iroh_net = start_server(iroh_secret_key.clone(), client_path.clone()).await?;

            // sleep(Duration::from_secs(1)).await;
            // iroh_net.router.shutdown().await?;

            let iroh_net1 = start_server(iroh_secret_key, client_path).await?;

            let mut tickets = std::collections::HashMap::new();
            tickets.insert("node".to_string(), node_ticket.parse()?);
            tickets.insert("folder".to_string(), folder_ticket.parse()?);
            tickets.insert("resource".to_string(), resource_ticket.parse()?);
            tickets.insert("resource1".to_string(), resource_ticket1.parse()?);
            tickets.insert("resource2".to_string(), resource_ticket2.parse()?);
            tickets.insert("resource3".to_string(), resource_ticket3.parse()?);
            let store_state = create_files(&iroh_net1, Some(tickets)).await?;
            Some(store_state)
        }
        Commands::Read => {
            println!("Reading data from server...");
            None
        }
    };
    println!("Waiting for input or Ctrl+C...");
    println!("Type 'help' for commands, 'quit' to exit, or press Ctrl+C to stop.");

    // Install signal handler
    let mut sigint = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())?;

    // Listen for user input
    let mut stdin = tokio::io::BufReader::new(tokio::io::stdin());
    let mut line = String::new();

    let store_state_binding = Arc::new(store_state);
    let store_state_weak = Arc::downgrade(&store_state_binding);

    loop {
        line.clear();
        tokio::select! {
            // Listen for SIGINT (Ctrl+C) signal
            _ = sigint.recv() => {
                println!("\n🛑 Received SIGINT (Ctrl+C), shutting down gracefully...");
                break;
            }
            // Listen for user input
            result = stdin.read_line(&mut line) => {
                match result {
                    Ok(0) => {
                        // EOF reached
                        println!("📤 Input stream closed");
                        break;
                    }
                    Ok(_) => {
                        let input = line.trim();
                        if input.is_empty() {
                            continue;
                        }

                        println!("📝 You entered: {}", input);

                        // Handle specific commands
                        match input {
                            "quit" | "exit" => {
                                println!("👋 Goodbye!");
                                break;
                            }
                            "help" => {
                                println!("📋 Available commands:");
                                println!("  help   - Show this help message");
                                println!("  quit   - Exit the program");
                                println!("  exit   - Exit the program");
                                println!("  status - Show current status");
                                println!("  add    - Load images from a directory into resources");
                                println!("  add_folder - Add a new folder named 'New Folder1'");
                                println!("  get    - Retrieve and display the number of resources");
                                println!("  get_folder - Retrieve and display the number of folders");
                                println!("  Ctrl+C - Force exit");
                            }
                            "status" => {
                                println!("✅ System is running and listening for input...");
                            }
                            "add"=>{
                                if let Some(store_state_arc) = store_state_weak.upgrade().unwrap().as_ref() {
                                    if let Some(resource)=&*store_state_arc.resource.read().await{
                                        match get_images_directory() {
                                            Ok(images_path) => {
                                                println!("📁 Loading images from: {:?}", images_path);
                                                if let Err(e) = load_images_to_resources(resource, &images_path).await {
                                                    println!("❌ Failed to load images: {}", e);
                                                } else {
                                                    println!("✅ Images loaded successfully.");
                                                }
                                            }
                                            Err(e) => {
                                                println!("❌ Could not find images directory: {}", e);
                                            }
                                        }
                                    }
                                } else {
                                    println!("❌ IrohNet is not available.");
                                }
                            }
                            "add_folder"=>{
                                if let Some(store_state_arc) = store_state_weak.upgrade().unwrap().as_ref() {
                                    if let Some(folder)=&*store_state_arc.folder.read().await{
                                        folder.insert_folder("New Folder".to_string()).await?;
                                        println!("✅ Folder added.");
                                    }
                                } else {
                                    println!("❌ IrohNet is not available.");
                                }
                            }
                            "get"=>{
                                 if let Some(store_state_arc) = store_state_weak.upgrade().unwrap().as_ref() {
                                    if let Some(resource)=&*store_state_arc.resource.read().await{
                                        let resources = resource.search().await?;
                                        println!("✅ Retrieved resources len: {:?}", resources.len());
                                    }
                                }
                            }
                             "get_folder"=>{
                                 if let Some(store_state_arc) = store_state_weak.upgrade().unwrap().as_ref() {
                                    if let Some(folder)=&*store_state_arc.folder.read().await{
                                        let folders = folder.search().await?;
                                        println!("✅ Retrieved folders len: {:?}", folders.len());
                                    }
                                }
                            }
                            _ => {
                                println!("❓ Unknown command: '{}'. Type 'help' for available commands.", input);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("❌ Error reading input: {}", e);
                        break;
                    }
                }
            }
        }
    }

    // Give some time for cleanup to complete
    println!("🔄 Cleaning up...");
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    println!("✅ Shutdown complete.");

    Ok(())
}
