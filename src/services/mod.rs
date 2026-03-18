pub mod aggregator;
pub mod binance;
pub mod coinbase;
pub mod okx;
pub mod rest;
use crate::types::{OrderbookState, Symbol};
use tokio::task::JoinHandle;

pub fn start_all_streams(state: OrderbookState) -> Vec<JoinHandle<()>> {
    let symbols = vec![Symbol::BTC, Symbol::ETH, Symbol::SOL];
    let mut handles = vec![];

    for symbol in symbols {
        // Binance
        let state_clone = state.clone();
        handles.push(tokio::spawn(async move {
            if let Err(e) = binance::start_binance_stream(symbol, state_clone).await {
                eprintln!("[Binance] Fatal error for {}: {}", symbol, e);
            }
        }));

        // Coinbase
        let state_clone = state.clone();
        handles.push(tokio::spawn(async move {
            if let Err(e) = coinbase::start_coinbase_stream(symbol, state_clone).await {
                eprintln!("[Coinbase] Fatal error for {}: {}", symbol, e);
            }
        }));

        // OKX
        let state_clone = state.clone();
        handles.push(tokio::spawn(async move {
            if let Err(e) = okx::start_okx_stream(symbol, state_clone).await {
                eprintln!("[OKX] Fatal error for {}: {}", symbol, e);
            }
        }));
    }

    handles
}
