use ordered_float::OrderedFloat;
use serde::{Deserialize, Serialize};

use std::{
    collections::{BTreeMap, HashMap},
    fmt,
    sync::Arc,
};
use tokio::sync::RwLock;

//trading symbols for BASE/USDC pairs only
//supported markets:
// BTC/USDC
// ETH/USDC
// SOL/USDC
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Symbol {
    BTC,
    ETH,
    SOL,
}

//to print symbol
impl fmt::Display for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Symbol::BTC => write!(f, "BTC"),
            Symbol::ETH => write!(f, "ETH"),
            Symbol::SOL => write!(f, "SOL"),
        }
    }
}

//for parsing a string into a symbol
impl Symbol {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "BTC" => Some(Symbol::BTC),
            "ETH" => Some(Symbol::ETH),
            "SOL" => Some(Symbol::SOL),
            _ => None,
        }
    }
    pub fn to_binance_symbol(&self) -> String {
        format!("{}USDC", self)
    }
    pub fn to_coinbase_symbol(&self) -> String {
        format!("{}-USD", self)
    }
    pub fn to_okx_symbol(&self) -> String {
        format!("{}-USDC", self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Exchange {
    Binance,
    Okx,
    Coinbase,
}

impl fmt::Display for Exchange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Exchange::Binance => write!(f, "binance"),
            Exchange::Okx => write!(f, "okx"),
            Exchange::Coinbase => write!(f, "coinbase"),
        }
    }
}

impl Exchange {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "binance" => Some(Exchange::Binance),
            "okx" => Some(Exchange::Okx),
            "coinbase" => Some(Exchange::Coinbase),
            _ => None,
        }
    }
}
// raw per-exchange orderbook that mirrors the exchange's native feed and format
#[derive(Debug, Clone)]
pub struct ExchangeOrderbook {
    pub symbol: Symbol,
    pub exchange: Exchange,
    pub bids: BTreeMap<OrderedFloat<f64>, f64>,
    pub asks: BTreeMap<OrderedFloat<f64>, f64>,
    pub last_update: i64,
}

impl ExchangeOrderbook {
    pub fn new(symbol: Symbol, exchange: Exchange) -> Self {
        Self {
            symbol,
            exchange,
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            last_update: chrono::Utc::now().timestamp_millis(),
        }
    }

    pub fn update_from_normalized(&mut self, normalized: &NormalizedOrderbook) {
        self.bids.clear();
        self.asks.clear();
        for level in &normalized.bids {
            self.bids.insert(OrderedFloat(level.price), level.quantity);
        }

        for level in &normalized.asks {
            self.asks.insert(OrderedFloat(level.price), level.quantity);
        }
        self.last_update = normalized.timestamp;
    }

    pub fn apply_delta(&mut self, normalized: &NormalizedOrderbook) {
        for level in &normalized.bids {
            if level.quantity == 0.0 {
                self.bids.remove(&OrderedFloat(level.price));
            } else {
                self.bids.insert(OrderedFloat(level.price), level.quantity);
            }
        }
        for level in &normalized.asks {
            if level.quantity == 0.0 {
                self.asks.remove(&OrderedFloat(level.price));
            } else {
                self.asks.insert(OrderedFloat(level.price), level.quantity);
            }
        }
        self.last_update = normalized.timestamp;
    }

    pub fn initialize_from_snapshot(&mut self, normalized: NormalizedOrderbook) {
        self.bids.clear();
        self.asks.clear();

        for level in normalized.bids {
            self.bids.insert(OrderedFloat(level.price), level.quantity);
        }

        for level in normalized.asks {
            self.asks.insert(OrderedFloat(level.price), level.quantity);
        }

        self.last_update = normalized.timestamp;
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderbookLevel {
    pub price: f64,
    pub quantity: f64,
}

// aggregated orderbook that merges normalized liquidity across all exchanges
#[derive(Debug, Clone)]
pub struct InternalOrderbook {
    pub bids: BTreeMap<OrderedFloat<f64>, f64>, //price -> total_quantity
    pub asks: BTreeMap<OrderedFloat<f64>, f64>,
}

impl InternalOrderbook {
    pub fn new() -> Self {
        Self {
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
        }
    }

    pub fn update_bid(&mut self, price: f64, quantity: f64) {
        let price_key = OrderedFloat(price);
        if quantity == 0.0 {
            self.bids.remove(&price_key);
        } else {
            self.bids.insert(price_key, quantity);
        }
    }

    pub fn update_ask(&mut self, price: f64, quantity: f64) {
        let price_key = OrderedFloat(price);
        if quantity == 0.0 {
            self.asks.remove(&price_key);
        } else {
            self.asks.insert(price_key, quantity);
        }
    }

    pub fn best_bid(&self) -> Option<(f64, f64)> {
        self.bids
            .iter()
            .next_back()
            .map(|(price, qty)| (price.0, *qty))
    }

    pub fn best_ask(&self) -> Option<(f64, f64)> {
        self.asks.iter().next().map(|(price, qty)| (price.0, *qty))
    }

    //convert to API response format
    pub fn to_levels(&self, limit: usize) -> (Vec<OrderbookLevel>, Vec<OrderbookLevel>) {
        let bids = self
            .bids
            .iter()
            .rev()
            .take(limit)
            .map(|(price, qty)| OrderbookLevel {
                price: price.0,
                quantity: *qty,
            })
            .collect();

        let asks = self
            .asks
            .iter()
            .take(limit)
            .map(|(price, qty)| OrderbookLevel {
                price: price.0,
                quantity: *qty,
            })
            .collect();
        (bids, asks)
    }
}

impl Default for InternalOrderbook {
    fn default() -> Self {
        Self::new()
    }
}

//per exchange orderbook normalized (converted into a unified symbol, currencty, precision)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedOrderbook {
    pub symbol: Symbol,
    pub exchange: Exchange,
    pub timestamp: i64,
    pub bids: Vec<OrderbookLevel>,
    pub asks: Vec<OrderbookLevel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BestPrice {
    pub symbol: Symbol,
    pub timestamp: i64,
    pub best_bid: PriceLevel,
    pub best_ask: PriceLevel,
    pub spread: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceLevel {
    pub price: f64,
    pub quantity: f64,
    pub exchange: String,
}

//response for aggregated orderbook endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedOrderbookResponse {
    pub symbol: Symbol,
    pub timestamp: i64,
    pub bids: Vec<OrderbookLevel>,
    pub asks: Vec<OrderbookLevel>,
    pub spread: f64,
    pub mid_price: f64,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub connections: std::collections::HashMap<String, ExchangeStatus>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExchangeStatus {
    pub connected: bool,
}

pub type OrderbookState = Arc<RwLock<HashMap<(Symbol, Exchange), ExchangeOrderbook>>>;
pub fn create_orderbook_state() -> OrderbookState {
    Arc::new(RwLock::new(HashMap::new()))
}
