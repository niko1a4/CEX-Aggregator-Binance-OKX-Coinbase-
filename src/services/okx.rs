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
struct OkxMessage {
    #[allow(dead_code)]
    arg: Option<OkxArg>,
    data: Option<Vec<OkxData>>,
}

#[derive(Debug, Deserialize)]
struct OkxArg {
    #[allow(dead_code)]
    channel: String,
    #[serde(rename = "instId")]
    #[allow(dead_code)]
    inst_id: String,
}

#[derive(Debug, Deserialize)]
struct OkxData {
    asks: Vec<[String; 4]>,
    bids: Vec<[String; 4]>,
    ts: String,
}

pub async fn start_okx_stream(symbol: Symbol, state: OrderbookState) -> Result<()> {
    println!("[OKX] Starting stream for {}", symbol);

    loop {
        match initialize_and_stream(symbol, state.clone()).await {
            Ok(_) => println!("[OKX] Stream ended for {}", symbol),
            Err(e) => {
                eprintln!("[OKX] Error for {}: {}. Reconnecting in 5s...", symbol, e);
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            }
        }
    }
}

async fn initialize_and_stream(symbol: Symbol, state: OrderbookState) -> Result<()> {
    // Step 1: Fetch initial snapshot
    println!("[OKX] Fetching initial snapshot for {}...", symbol);
    let snapshot = crate::services::rest::fetch_okx_snapshot(symbol).await?;

    {
        let mut state_lock = state.write().await;
        let book = state_lock
            .entry((symbol, Exchange::Okx))
            .or_insert_with(|| ExchangeOrderbook::new(symbol, Exchange::Okx));

        book.initialize_from_snapshot(snapshot);
        println!(
            "[OKX] {} INITIALIZED - Bids: {}, Asks: {}",
            symbol,
            book.bids.len(),
            book.asks.len()
        );
    }

    // Step 2: Connect to WebSocket
    let url = "wss://ws.okx.com:8443/ws/v5/public";
    let (ws_stream, _) = connect_async(url).await?;
    let (mut write, mut read) = ws_stream.split();

    let subscribe_msg = json!({
        "op": "subscribe",
        "args": [{
            "channel": "books",
            "instId": symbol.to_okx_symbol()
        }]
    });

    write
        .send(Message::Text(Utf8Bytes::from(subscribe_msg.to_string())))
        .await?;
    println!("[OKX] Connected to delta stream for {}", symbol);

    // Step 3: Process delta updates
    while let Some(msg) = read.next().await {
        let msg = msg?;

        if let Message::Text(text) = msg {
            match serde_json::from_str::<OkxMessage>(&text) {
                Ok(okx_msg) => {
                    if let Some(data) = okx_msg.data {
                        for update in data {
                            let normalized = normalize_okx_delta(update, symbol)?;

                            let mut state_lock = state.write().await;
                            if let Some(book) = state_lock.get_mut(&(symbol, Exchange::Okx)) {
                                book.apply_delta(&normalized);
                            }
                        }
                    }
                }
                Err(_) => {
                    // Ignore subscription confirmations
                }
            }
        }
    }

    Ok(())
}

fn normalize_okx_delta(update: OkxData, symbol: Symbol) -> Result<NormalizedOrderbook> {
    let bids = update
        .bids
        .iter()
        .filter_map(|[price, qty, _, _]| {
            Some(OrderbookLevel {
                price: price.parse().ok()?,
                quantity: qty.parse().ok()?,
            })
        })
        .collect();

    let asks = update
        .asks
        .iter()
        .filter_map(|[price, qty, _, _]| {
            Some(OrderbookLevel {
                price: price.parse().ok()?,
                quantity: qty.parse().ok()?,
            })
        })
        .collect();

    let timestamp = update
        .ts
        .parse::<i64>()
        .unwrap_or_else(|_| chrono::Utc::now().timestamp_millis());

    Ok(NormalizedOrderbook {
        symbol,
        exchange: Exchange::Okx,
        timestamp,
        bids,
        asks,
    })
}
