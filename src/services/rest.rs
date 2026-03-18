use crate::types::*;
use anyhow::Result;

pub async fn fetch_binance_snapshot(symbol: Symbol) -> Result<NormalizedOrderbook> {
    let symbol_str = symbol.to_binance_symbol();
    let url = format!(
        "https://api.binance.com/api/v3/depth?symbol={}&limit=1000",
        symbol_str
    );

    println!("[Binance REST] Fetching from: {}", url);

    let response = reqwest::get(&url).await?;
    let status = response.status();

    if !status.is_success() {
        let text = response.text().await?;
        println!("[Binance REST] Error status: {}, Body: {}", status, text);
        return Err(anyhow::anyhow!(
            "Binance API returned status {}: {}",
            status,
            text
        ));
    }

    let text = response.text().await?;
    println!(
        "[Binance REST] Success! Response length: {} bytes",
        text.len()
    );

    let data: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| anyhow::anyhow!("Failed to parse JSON: {}", e))?;

    // Check for API error in response
    if let Some(code) = data.get("code") {
        let msg = data
            .get("msg")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown error");
        return Err(anyhow::anyhow!("Binance API error code {}: {}", code, msg));
    }

    let bids_array = data["bids"].as_array().ok_or_else(|| {
        println!("[Binance REST] Response data: {:?}", data);
        anyhow::anyhow!("No bids array in response")
    })?;

    let asks_array = data["asks"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("No asks array in response"))?;

    println!(
        "[Binance REST] Parsing {} bids and {} asks",
        bids_array.len(),
        asks_array.len()
    );

    let bids = bids_array
        .iter()
        .filter_map(|item| {
            let arr = item.as_array()?;
            Some(OrderbookLevel {
                price: arr[0].as_str()?.parse().ok()?,
                quantity: arr[1].as_str()?.parse().ok()?,
            })
        })
        .collect();

    let asks = asks_array
        .iter()
        .filter_map(|item| {
            let arr = item.as_array()?;
            Some(OrderbookLevel {
                price: arr[0].as_str()?.parse().ok()?,
                quantity: arr[1].as_str()?.parse().ok()?,
            })
        })
        .collect();

    Ok(NormalizedOrderbook {
        symbol,
        exchange: Exchange::Binance,
        timestamp: chrono::Utc::now().timestamp_millis(),
        bids,
        asks,
    })
}

pub async fn fetch_coinbase_snapshot(symbol: Symbol) -> Result<NormalizedOrderbook> {
    let symbol_str = symbol.to_coinbase_symbol();
    let url = format!(
        "https://api.exchange.coinbase.com/products/{}/book?level=2",
        symbol_str
    );

    println!("[Coinbase REST] Fetching from: {}", url);

    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .header("User-Agent", "rust-cex-aggregator")
        .send()
        .await?;

    let status = response.status();

    if !status.is_success() {
        let text = response.text().await?;
        println!("[Coinbase REST] Error status: {}, Body: {}", status, text);
        return Err(anyhow::anyhow!(
            "Coinbase API returned status {}: {}",
            status,
            text
        ));
    }

    let text = response.text().await?;
    println!(
        "[Coinbase REST] Success! Response length: {} bytes",
        text.len()
    );

    let data: serde_json::Value = serde_json::from_str(&text)?;

    if let Some(message) = data.get("message") {
        return Err(anyhow::anyhow!("Coinbase API error: {}", message));
    }

    let bids = data["bids"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("No bids in response"))?
        .iter()
        .filter_map(|item| {
            let arr = item.as_array()?;
            Some(OrderbookLevel {
                price: arr[0].as_str()?.parse().ok()?,
                quantity: arr[1].as_str()?.parse().ok()?,
            })
        })
        .collect();

    let asks = data["asks"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("No asks in response"))?
        .iter()
        .filter_map(|item| {
            let arr = item.as_array()?;
            Some(OrderbookLevel {
                price: arr[0].as_str()?.parse().ok()?,
                quantity: arr[1].as_str()?.parse().ok()?,
            })
        })
        .collect();

    Ok(NormalizedOrderbook {
        symbol,
        exchange: Exchange::Coinbase,
        timestamp: chrono::Utc::now().timestamp_millis(),
        bids,
        asks,
    })
}

pub async fn fetch_okx_snapshot(symbol: Symbol) -> Result<NormalizedOrderbook> {
    let symbol_str = symbol.to_okx_symbol();
    let url = format!(
        "https://www.okx.com/api/v5/market/books?instId={}&sz=400",
        symbol_str
    );

    println!("[OKX REST] Fetching from: {}", url);

    let response = reqwest::get(&url).await?;
    let status = response.status();

    if !status.is_success() {
        let text = response.text().await?;
        println!("[OKX REST] Error status: {}, Body: {}", status, text);
        return Err(anyhow::anyhow!(
            "OKX API returned status {}: {}",
            status,
            text
        ));
    }

    let text = response.text().await?;
    println!("[OKX REST] Success! Response length: {} bytes", text.len());

    let data: serde_json::Value = serde_json::from_str(&text)?;

    let book = &data["data"][0];

    let bids = book["bids"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("No bids in response"))?
        .iter()
        .filter_map(|item| {
            let arr = item.as_array()?;
            Some(OrderbookLevel {
                price: arr[0].as_str()?.parse().ok()?,
                quantity: arr[1].as_str()?.parse().ok()?,
            })
        })
        .collect();

    let asks = book["asks"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("No asks in response"))?
        .iter()
        .filter_map(|item| {
            let arr = item.as_array()?;
            Some(OrderbookLevel {
                price: arr[0].as_str()?.parse().ok()?,
                quantity: arr[1].as_str()?.parse().ok()?,
            })
        })
        .collect();

    Ok(NormalizedOrderbook {
        symbol,
        exchange: Exchange::Okx,
        timestamp: chrono::Utc::now().timestamp_millis(),
        bids,
        asks,
    })
}
