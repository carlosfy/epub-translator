use epub_translator::deepl;

use tokio::signal;

// Runs the mock server, useful for testing changes to epubs.
// Use Ctrl+C to stop the server.
#[tokio::main]
async fn main() {
    let shutdown_signal = deepl::run_mock_server()
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
