mod console_graphics;
mod constants;
mod message;
mod networking;
mod user_interface;

use console_graphics::GraphicsEngine;
use constants::{CHAT_PORT, DISCOVERY_PORT};
use message::Message;
use networking::{Broadcaster, Receiver};
use std::io::Write;
use std::sync::Arc;
use tokio::signal;
use tokio::task;
use tokio::time;
use user_interface::UserInterface;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // Setup terminal cleanup on exit
    let _cleanup_guard = CleanupGuard {};
    println!("Subnet Vox - P2P Chat (Tailscale Compatible)");
    println!("Press Ctrl+Q or Ctrl+C to exit");
    println!("Special Features: Tailscale Mesh Broadcasting Enabled");

    // Create graphics engine
    let graphics_engine = GraphicsEngine::new(64);

    // Prompt for username
    let mut username = String::new();

    // Print logo first
    GraphicsEngine::print_logo()?;
    println!("\n\n========================================\n");
    print!("your username: ");
    std::io::stdout().flush()?;
    std::io::stdin().read_line(&mut username)?;
    let username = username.trim().to_string();

    println!("\n\nwelcome. joining the subnet...");

    // Create the networking components
    let receiver = Receiver::new(CHAT_PORT, username.clone());
    let broadcaster = Broadcaster::new(CHAT_PORT, username.clone());

    // Create user interface
    let mut user_interface =
        UserInterface::new(receiver.clone(), broadcaster.clone(), graphics_engine);
    user_interface.username = username;

    // Load cyberpunk intro
    show_intro(CHAT_PORT, DISCOVERY_PORT).await;

    // Set up terminal UI
    GraphicsEngine::setup_terminal()?;
    {
        let mut engine = user_interface.graphics_engine.lock().unwrap();
        let _ = engine.print_all_messages(true);
        let _ = engine.print_status_bar();
        let _ = engine.print_input_prompt();
    }

    // Start the format keeper thread for terminal
    let graphics_engine_clone = user_interface.graphics_engine.clone();
    task::spawn_blocking(move || {
        GraphicsEngine::console_format_keeper(graphics_engine_clone);
    });

    // Start the discovery listener
    let receiver_clone = receiver.clone();
    task::spawn(async move {
        if let Err(e) = receiver_clone.listen_for_discovery(DISCOVERY_PORT).await {
            eprintln!("Discovery listener error: {}", e);
        }
    });

    // Start the message listener
    let mut receiver_clone2 = receiver.clone();
    task::spawn(async move {
        if let Err(e) = receiver_clone2.listen_for_messages(CHAT_PORT).await {
            eprintln!("Message listener error: {}", e);
        }
    });

    // Start discovery service (periodically broadcasts presence)
    let broadcaster_clone = broadcaster.clone();
    task::spawn(async move {
        if let Err(e) = Broadcaster::discovery_service(Arc::new(broadcaster_clone)).await {
            eprintln!("Discovery service error: {}", e);
        }
    });

    // Set up peer list sync
    let broadcaster_clone = user_interface.broadcaster.clone();
    let receiver_arc = user_interface.receiver.clone();
    task::spawn(async move {
        let sync_interval = time::Duration::from_secs(5);
        loop {
            let receiver_peers = {
                let receiver = receiver_arc.lock().unwrap();
                receiver.get_peers()
            };

            // Update broadcaster's peer list with receiver's peers
            let broadcaster_peers = broadcaster_clone.get_peers();

            // Merge the peer lists
            let receiver_peers_clone = receiver_peers.lock().unwrap().clone();

            {
                let mut broadcaster_peers_lock = broadcaster_peers.lock().unwrap();
                for peer in receiver_peers_clone {
                    broadcaster_peers_lock.insert(peer);
                }
            } // Release lock before await

            time::sleep(sync_interval).await;
        }
    });

    // Start the continuous receive task
    let user_interface_clone = user_interface.clone();
    task::spawn(async move {
        continuous_receive_task(&user_interface_clone).await;
    });

    // Handle graceful shutdown with Ctrl+C
    let shutdown = Arc::new(tokio::sync::Notify::new());
    let shutdown_clone = shutdown.clone();

    tokio::spawn(async move {
        if let Err(e) = signal::ctrl_c().await {
            eprintln!("Failed to listen for Ctrl+C: {}", e);
            return;
        }
        shutdown_clone.notify_one();
    });

    // Start continuous broadcast (this runs on the main thread)
    let broadcast_task =
        tokio::spawn(async move { continuous_broadcast_task(&user_interface).await });

    // Wait for either the broadcast task to complete or Ctrl+C
    tokio::select! {
        result = broadcast_task => {
            if let Err(e) = result {
                eprintln!("Broadcast task failed: {:?}", e);
            }
        }
        _ = shutdown.notified() => {
            println!("\nShutting down gracefully...");
            // Restore terminal properly
            let _ = GraphicsEngine::restore_terminal();
            // Force exit with a small delay to allow terminal to reset
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            std::process::exit(0);
        }
    }

    // Make sure the terminal is properly restored
    let _ = GraphicsEngine::restore_terminal();

    // Force the process to exit completely
    std::process::exit(0);

    // This is unreachable, but needed for type correctness
    #[allow(unreachable_code)]
    Ok(())
}

// Helper functions
async fn continuous_receive_task(ui: &UserInterface) {
    let receiver = ui.receiver.clone();
    let graphics_engine = ui.graphics_engine.clone();

    loop {
        // Try to get a message from the queue
        let message = {
            let receiver_lock = receiver.lock().unwrap();
            receiver_lock.get_queue_message()
        };

        if let Some(message) = message {
            // Add message to graphics engine
            {
                let mut engine = graphics_engine.lock().unwrap();
                engine.add_message(&message);
                let _ = engine.print_all_messages(false);
            }
        }

        // Small delay to prevent CPU thrashing
        time::sleep(time::Duration::from_millis(10)).await;
    }
}

async fn continuous_broadcast_task(ui: &UserInterface) -> std::io::Result<()> {
    GraphicsEngine::setup_terminal()?;

    let engine = ui.graphics_engine.clone();

    loop {
        let mut input = String::new();

        // Prepare for input
        {
            let mut engine = engine.lock().unwrap();
            engine.print_input_prompt()?;
        }

        // Get input character by character
        loop {
            let (input_complete, should_exit) = {
                let mut engine = engine.lock().unwrap();
                engine.read_input(&mut input)?
            };
            if should_exit {
                // User pressed Ctrl+Q or Ctrl+C or Esc
                // Restore terminal properly
                GraphicsEngine::restore_terminal()?;
                // Force exit the process to ensure all threads are terminated
                std::process::exit(0);
            }
            if input_complete {
                break;
            }
        }

        // Reset the printing line
        {
            let mut engine = engine.lock().unwrap();
            engine.print_input_prompt()?;
        }

        // Broadcast message
        let message = Message::new(
            input.clone(), // Clone so we can use it again
            ui.username.clone(),
            constants::OUTBOUND_MESSAGE_REPORTED_IP.to_string(),
        );

        // Also add this message to our own display
        {
            let mut engine = ui.graphics_engine.lock().unwrap();
            // Create a local message to show in our UI
            let local_message = Message::new(
                input,
                ui.username.clone(), // Use just the username, our display logic handles the YOU part
                "local".to_string(),
            );
            engine.add_message(&local_message);
            let _ = engine.print_all_messages(false);
        }

        if let Err(e) = ui.broadcaster.broadcast_message(message).await {
            eprintln!("Failed to broadcast message: {}", e);
        }
    }
}

async fn show_intro(chat_port: u16, discovery_port: u16) {
    if constants::DO_BULLSHIT_INTRO {
        // Cyberpunk-style intro sequence
        time::sleep(time::Duration::from_millis(1000)).await;
        println!(
            "RECEIVER    >>> ONLINE!                   LISTENING ON:    DISCOVERY:{} | CHAT:{}",
            discovery_port, chat_port
        );
        time::sleep(time::Duration::from_millis(20)).await;
        println!(
            "BROADCASTER >>> ONLINE!                   BROADCASTING ON: DISCOVERY:{} | CHAT:{}",
            discovery_port, chat_port
        );
        time::sleep(time::Duration::from_millis(500)).await;
        println!("setting up auxillery networking systems...");
        time::sleep(time::Duration::from_millis(12)).await;
        println!("launching threads...");
        time::sleep(time::Duration::from_millis(5)).await;
        println!("jacking in...");
        time::sleep(time::Duration::from_millis(2)).await;
        println!("breaking the cyber ice...");
        time::sleep(time::Duration::from_millis(172)).await;
        println!("contacting chatgpt to fix compilation errors...");
        time::sleep(time::Duration::from_millis(7)).await;
        println!("hol up mom said dinner is ready brb...");
        time::sleep(time::Duration::from_millis(165)).await;
        println!("ok im back...");
        time::sleep(time::Duration::from_millis(1)).await;
        println!("chatgpt unable to fix all errors, contacting gemini...");
        time::sleep(time::Duration::from_millis(2)).await;
        println!("connecting to imperial vox channels...");
        time::sleep(time::Duration::from_millis(3)).await;
        println!("requesting ip from adeptus mechanicus router...");
        time::sleep(time::Duration::from_millis(3)).await;
        println!("negotiating connection terms with NetWatch...");
        time::sleep(time::Duration::from_millis(186)).await;
        println!("determining used device type: Cyberdeck...");
        time::sleep(time::Duration::from_millis(2)).await;
        println!("turning on styalized neon japanese advertisement in a filthy back alley...");
        time::sleep(time::Duration::from_millis(100)).await;
        println!("{}", constants::ONLINE_ASCII_ART);
        time::sleep(time::Duration::from_millis(1200)).await;
    }
}

// This struct ensures that terminal is restored on program exit
struct CleanupGuard;

impl Drop for CleanupGuard {
    fn drop(&mut self) {
        if let Err(e) = GraphicsEngine::restore_terminal() {
            eprintln!("Failed to restore terminal: {}", e);
        }
    }
}
