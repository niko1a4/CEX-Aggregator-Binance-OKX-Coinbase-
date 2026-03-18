use crate::types::*;
use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::json;
use tokio_tungstenite::{
    connect_async,
    tungstenite::{Message, Utf8Bytes},
};

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum CoinbaseMessage {
    #[serde(rename = "l2update")]
    L2Update {
        #[allow(dead_code)]
        product_id: String,
        changes: Vec<[String; 3]>,
        #[allow(dead_code)]
        time: String,
    },
    #[serde(rename = "subscriptions")]
    Subscriptions {
        #[allow(dead_code)]
        channels: Vec<serde_json::Value>,
    },
}

pub async fn start_coinbase_stream(symbol: Symbol, state: OrderbookState) -> Result<()> {
    println!("[Coinbase] Starting stream for {}", symbol);

    loop {
        match initialize_and_stream(symbol, state.clone()).await {
            Ok(_) => println!("[Coinbase] Stream ended for {}", symbol),
            Err(e) => {
                eprintln!(
                    "[Coinbase] Error for {}: {}. Reconnecting in 5s...",
                    symbol, e
                );
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            }
        }
    }
}

async fn initialize_and_stream(symbol: Symbol, state: OrderbookState) -> Result<()> {
    // Step 1: Fetch initial snapshot
    println!("[Coinbase] Fetching initial snapshot for {}...", symbol);
    let snapshot = crate::services::rest::fetch_coinbase_snapshot(symbol).await?;

    {
        let mut state_lock = state.write().await;
        let book = state_lock
            .entry((symbol, Exchange::Coinbase))
            .or_insert_with(|| ExchangeOrderbook::new(symbol, Exchange::Coinbase));

        book.initialize_from_snapshot(snapshot);
        println!(
            "[Coinbase] {} INITIALIZED - Bids: {}, Asks: {}",
            symbol,
            book.bids.len(),
            book.asks.len()
        );
    }

    // Step 2: Connect to WebSocket
    let url = "wss://ws-feed.exchange.coinbase.com";
    let (ws_stream, _) = connect_async(url).await?;
    let (mut write, mut read) = ws_stream.split();

    let subscribe_msg = json!({
        "type": "subscribe",
        "product_ids": [symbol.to_coinbase_symbol()],
        "channels": ["level2"]
    });

    write
        .send(Message::Text(Utf8Bytes::from(subscribe_msg.to_string())))
        .await?;
    println!("[Coinbase] Connected to delta stream for {}", symbol);

    // Step 3: Process delta updates
    while let Some(msg) = read.next().await {
        let msg = msg?;

        if let Message::Text(text) = msg {
            match serde_json::from_str::<CoinbaseMessage>(&text) {
                Ok(CoinbaseMessage::L2Update { changes, .. }) => {
                    let normalized = normalize_coinbase_delta(changes, symbol)?;

                    let mut state_lock = state.write().await;
                    if let Some(book) = state_lock.get_mut(&(symbol, Exchange::Coinbase)) {
                        book.apply_delta(&normalized);
                    }
                }
                Ok(CoinbaseMessage::Subscriptions { .. }) => {
                    // Ignore
                }
                Err(_) => {
                    // Ignore parse errors for other message types
                }
            }
        }
    }

    Ok(())
}

fn normalize_coinbase_delta(
    changes: Vec<[String; 3]>,
    symbol: Symbol,
) -> Result<NormalizedOrderbook> {
    let mut bids = Vec::new();
    let mut asks = Vec::new();

    for change in changes {
        let side = &change[0];
        let price: f64 = change[1].parse()?;
        let quantity: f64 = change[2].parse()?;

        let level = OrderbookLevel { price, quantity };

        match side.as_str() {
            "buy" => bids.push(level),
            "sell" => asks.push(level),
            _ => {}
        }
    }

    Ok(NormalizedOrderbook {
        symbol,
        exchange: Exchange::Coinbase,
        timestamp: chrono::Utc::now().timestamp_millis(),
        bids,
        asks,
    })
}
