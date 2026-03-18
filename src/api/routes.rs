use crate::services::aggregator::OrderbookAggregator;
use crate::types::*;
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
};

pub fn create_router(state: OrderbookState) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/orderbook/{symbol}", get(get_aggregated_orderbook))
        .route("/orderbook/{symbol}/best", get(get_best_prices))
        .route("/orderbook/{symbol}/{exchange}", get(get_single_exchange))
        .with_state(state)
}

// GET /health
async fn health_check(State(state): State<OrderbookState>) -> impl IntoResponse {
    let state_lock = state.read().await;

    let mut connections = std::collections::HashMap::new();

    // Check each exchange for each symbol
    for exchange in [Exchange::Binance, Exchange::Okx, Exchange::Coinbase] {
        let connected = [Symbol::BTC, Symbol::ETH, Symbol::SOL]
            .iter()
            .any(|symbol| state_lock.contains_key(&(*symbol, exchange)));

        connections.insert(exchange.to_string(), ExchangeStatus { connected });
    }

    let all_connected = connections.values().all(|status| status.connected);
    let status = if all_connected { "healthy" } else { "degraded" };

    Json(HealthResponse {
        status: status.to_string(),
        connections,
    })
}

// GET /orderbook/{symbol}
async fn get_aggregated_orderbook(
    Path(symbol_str): Path<String>,
    State(state): State<OrderbookState>,
) -> Result<Json<AggregatedOrderbookResponse>, StatusCode> {
    let symbol = Symbol::from_str(&symbol_str).ok_or(StatusCode::BAD_REQUEST)?;

    let state_lock = state.read().await;
    let aggregated =
        OrderbookAggregator::aggregate(&state_lock, symbol).ok_or(StatusCode::NOT_FOUND)?;

    let (bids, asks) = aggregated.to_levels(20); // top 20 levels

    let spread = match (aggregated.best_ask(), aggregated.best_bid()) {
        (Some((ask_price, _)), Some((bid_price, _))) => ask_price - bid_price,
        _ => 0.0,
    };

    let mid_price = match (aggregated.best_ask(), aggregated.best_bid()) {
        (Some((ask_price, _)), Some((bid_price, _))) => (ask_price + bid_price) / 2.0,
        _ => 0.0,
    };

    Ok(Json(AggregatedOrderbookResponse {
        symbol,
        timestamp: chrono::Utc::now().timestamp_millis(),
        bids,
        asks,
        spread,
        mid_price,
    }))
}

// GET /orderbook/{symbol}/best
async fn get_best_prices(
    Path(symbol_str): Path<String>,
    State(state): State<OrderbookState>,
) -> Result<Json<BestPrice>, StatusCode> {
    let symbol = Symbol::from_str(&symbol_str).ok_or(StatusCode::BAD_REQUEST)?;

    let state_lock = state.read().await;
    let best =
        OrderbookAggregator::best_prices(&state_lock, symbol).ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(best))
}

// GET /orderbook/{symbol}/{exchange}
async fn get_single_exchange(
    Path((symbol_str, exchange_str)): Path<(String, String)>,
    State(state): State<OrderbookState>,
) -> Result<Json<NormalizedOrderbook>, StatusCode> {
    let symbol = Symbol::from_str(&symbol_str).ok_or(StatusCode::BAD_REQUEST)?;
    let exchange = Exchange::from_str(&exchange_str).ok_or(StatusCode::BAD_REQUEST)?;

    let state_lock = state.read().await;
    let orderbook = OrderbookAggregator::single_exchange(&state_lock, symbol, exchange, 20)
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(orderbook))
}
