use crate::types::*;
use anyhow::Result;
use futures_util::StreamExt;
use serde::Deserialize;
use tokio_tungstenite::connect_async;

#[derive(Debug, Deserialize)]
struct BinanceDepthUpdate {
    #[serde(rename = "e")]
    #[allow(dead_code)]
    event_type: String,
    #[serde(rename = "E")]
    event_time: i64,
    #[serde(rename = "s")]
    #[allow(dead_code)]
    symbol: String,
    #[serde(rename = "U")]
    #[allow(dead_code)]
    first_update_id: i64,
    #[serde(rename = "u")]
    #[allow(dead_code)]
    final_update_id: i64,
    #[serde(rename = "b")]
    bids: Vec<[String; 2]>,
    #[serde(rename = "a")]
    asks: Vec<[String; 2]>,
}

pub async fn start_binance_stream(symbol: Symbol, state: OrderbookState) -> Result<()> {
    println!("[Binance] Starting stream for {}", symbol);

    loop {
        match initialize_and_stream(symbol, state.clone()).await {
            Ok(_) => println!("[Binance] Stream ended for {}", symbol),
            Err(e) => {
                eprintln!(
                    "[Binance] Error for {}: {}. Reconnecting in 5s...",
                    symbol, e
                );
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            }
        }
    }
}

async fn initialize_and_stream(symbol: Symbol, state: OrderbookState) -> Result<()> {
    // Step 1: Fetch initial snapshot via REST API
    println!("[Binance] Fetching initial snapshot for {}...", symbol);
    let snapshot = crate::services::rest::fetch_binance_snapshot(symbol).await?;

    {
        let mut state_lock = state.write().await;
        let book = state_lock
            .entry((symbol, Exchange::Binance))
            .or_insert_with(|| ExchangeOrderbook::new(symbol, Exchange::Binance));

        book.initialize_from_snapshot(snapshot);
        println!(
            "[Binance] {} INITIALIZED - Bids: {}, Asks: {}",
            symbol,
            book.bids.len(),
            book.asks.len()
        );
    }

    // Step 2: Connect to WebSocket for delta updates
    let stream_name = format!("{}@depth@100ms", symbol.to_binance_symbol().to_lowercase());
    let url = format!("wss://stream.binance.com:9443/ws/{}", stream_name);

    let (ws_stream, _) = connect_async(&url).await?;
    let (_write, mut read) = ws_stream.split();

    println!("[Binance] Connected to delta stream for {}", symbol);

    // Step 3: Process delta updates
    while let Some(msg) = read.next().await {
        let msg = msg?;

        if let tokio_tungstenite::tungstenite::Message::Text(text) = msg {
            match serde_json::from_str::<BinanceDepthUpdate>(&text) {
                Ok(update) => {
                    let normalized = normalize_binance_delta(update, symbol)?;

                    let mut state_lock = state.write().await;
                    if let Some(book) = state_lock.get_mut(&(symbol, Exchange::Binance)) {
                        book.apply_delta(&normalized);
                    }
                }
                Err(e) => {
                    eprintln!("[Binance] Failed to parse delta: {}", e);
                }
            }
        }
    }

    Ok(())
}

fn normalize_binance_delta(
    update: BinanceDepthUpdate,
    symbol: Symbol,
) -> Result<NormalizedOrderbook> {
    let bids = update
        .bids
        .iter()
        .filter_map(|[price, qty]| {
            Some(OrderbookLevel {
                price: price.parse().ok()?,
                quantity: qty.parse().ok()?,
            })
        })
        .collect();

    let asks = update
        .asks
        .iter()
        .filter_map(|[price, qty]| {
            Some(OrderbookLevel {
                price: price.parse().ok()?,
                quantity: qty.parse().ok()?,
            })
        })
        .collect();

    Ok(NormalizedOrderbook {
        symbol,
        exchange: Exchange::Binance,
        timestamp: update.event_time,
        bids,
        asks,
    })
}
