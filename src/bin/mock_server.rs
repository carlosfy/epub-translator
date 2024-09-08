use epub_translator::deepl;

use tokio::signal;

// Runs the mock server, useful for testing changes to epubs.
// Use Ctrl+C to stop the server.
#[tokio::main]
async fn main() {
    println!("Starting mock server...");
    let shutdown_signal = deepl::start_deepl_server()
        .await
        .expect("Failed to create mock server");

    // Wait for a Ctrl+C signal to initiate shutdown
    signal::ctrl_c()
        .await
        .expect("Failed to listen for shutdown signal");

    // Send shutdown signal to the mock server
    shutdown_signal
        .send(())
        .expect("Failed to send shutdown signal");

    println!("Shutting down mock server...");
}
