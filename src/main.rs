mod api;
mod services;
mod types;

use types::create_orderbook_state;

#[tokio::main]
async fn main() {
    println!(" Starting CEX Aggregator...");

    // Create shared state
    let state = create_orderbook_state();

    // Start all WebSocket streams
    let _stream_handles = services::start_all_streams(state.clone());

    // Give streams time to connect and receive initial data
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // Start HTTP server
    let app = api::routes::create_router(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("Failed to bind to port 3000");

    println!(" Server running on http://0.0.0.0:3000");
    println!("\nAvailable endpoints:");
    println!("  GET /health");
    println!("  GET /orderbook/:symbol  (btc, eth, sol)");
    println!("  GET /orderbook/:symbol/best");
    println!("  GET /orderbook/:symbol/:exchange (binance, okx, coinbase)");

    axum::serve(listener, app).await.expect("Server error");
}
