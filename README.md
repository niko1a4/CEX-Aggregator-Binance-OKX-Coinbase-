# CEX Aggregator (Rust)

A high-performance Centralized Exchange (CEX) orderbook aggregator built with **Rust**, **Axum**, and **Tokio**. This service streams real-time market data from multiple exchanges via WebSockets and exposes a unified REST API to query the aggregated state.

##  Features

* **Real-time Streaming:** Utilizes Tokio-based WebSocket workers to pull live orderbook data.
* **Unified State:** Aggregates data from Binance, OKX, and Coinbase into a single thread-safe state.
* **Async Performance:** Built on the Axum framework for low-latency HTTP responses.
* **Simple API:** Standardized endpoints to get the best prices across different venues.

##  Tech Stack

* **Language:** Rust
* **Web Framework:** [Axum](https://github.com/tokio-rs/axum)
* **Async Runtime:** [Tokio](https://tokio.rs/)
* **Exchanges Supported:** Binance, OKX, Coinbase

##  API Reference

The server runs on `http://0.0.0.0:3000` by default.

### Health Check
Check if the service and streams are active.
* **GET** `/health`

### Orderbook Data
Get the full aggregated orderbook for a specific asset.
* **GET** `/orderbook/:symbol`
* **Example:** `/orderbook/btc` or `/orderbook/sol`

### Best Price (Smart Routing)
Get the best bid and ask across all connected exchanges.
* **GET** `/orderbook/:symbol/best`

### Exchange-Specific Data
Filter the orderbook by a specific exchange.
* **GET** `/orderbook/:symbol/:exchange`
* **Exchanges:** `binance`, `okx`, `coinbase`

##  Installation & Running

1. **Clone the repository**
   ```bash
   git clone <your-repo-url>
   cd cex-aggregator