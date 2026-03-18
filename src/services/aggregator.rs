use crate::types::*;

pub struct OrderbookAggregator;

impl OrderbookAggregator {
    pub fn aggregate(
        state: &std::collections::HashMap<(Symbol, Exchange), ExchangeOrderbook>,
        symbol: Symbol,
    ) -> Option<InternalOrderbook> {
        let mut aggregated = InternalOrderbook::new();

        //collect orderbooks for this symbol accross all exchanges
        for exchange in [Exchange::Binance, Exchange::Okx, Exchange::Coinbase] {
            if let Some(exchange_book) = state.get(&(symbol, exchange)) {
                //sum quantities at each price level
                for (price, qty) in &exchange_book.bids {
                    *aggregated.bids.entry(*price).or_insert(0.0) += qty;
                }
                for (price, qty) in &exchange_book.asks {
                    *aggregated.asks.entry(*price).or_insert(0.0) += qty;
                }
            }
        }
        if aggregated.bids.is_empty() && aggregated.asks.is_empty() {
            None
        } else {
            Some(aggregated)
        }
    }

    pub fn best_prices(
        state: &std::collections::HashMap<(Symbol, Exchange), ExchangeOrderbook>,
        symbol: Symbol,
    ) -> Option<BestPrice> {
        let mut best_bid: Option<(f64, f64, Exchange)> = None;
        let mut best_ask: Option<(f64, f64, Exchange)> = None;

        for exchange in [Exchange::Binance, Exchange::Okx, Exchange::Coinbase] {
            if let Some(book) = state.get(&(symbol, exchange)) {
                //find best bid (highest)
                if let Some((price, qty)) = book.bids.iter().next_back() {
                    match best_bid {
                        None => best_bid = Some((price.0, *qty, exchange)),
                        Some((current_price, _, _)) if price.0 > current_price => {
                            best_bid = Some((price.0, *qty, exchange));
                        }
                        _ => {}
                    }
                }

                //find best ask (lowest)
                if let Some((price, qty)) = book.asks.iter().next() {
                    match best_ask {
                        None => best_ask = Some((price.0, *qty, exchange)),
                        Some((current_price, _, _)) if price.0 < current_price => {
                            best_ask = Some((price.0, *qty, exchange));
                        }
                        _ => {}
                    }
                }
            }
        }
        match (best_bid, best_ask) {
            (Some((bid_price, bid_qty, bid_exch)), Some((ask_price, ask_qty, ask_exch))) => {
                Some(BestPrice {
                    symbol,
                    timestamp: chrono::Utc::now().timestamp_millis(),
                    best_bid: PriceLevel {
                        price: bid_price,
                        quantity: bid_qty,
                        exchange: bid_exch.to_string(),
                    },
                    best_ask: PriceLevel {
                        price: ask_price,
                        quantity: ask_qty,
                        exchange: ask_exch.to_string(),
                    },
                    spread: ask_price - bid_price,
                })
            }
            _ => None,
        }
    }
    /// Get orderbook for single exchange
    pub fn single_exchange(
        state: &std::collections::HashMap<(Symbol, Exchange), ExchangeOrderbook>,
        symbol: Symbol,
        exchange: Exchange,
        limit: usize,
    ) -> Option<NormalizedOrderbook> {
        state.get(&(symbol, exchange)).map(|book| {
            let bids: Vec<OrderbookLevel> = book
                .bids
                .iter()
                .rev()
                .take(limit)
                .map(|(price, qty)| OrderbookLevel {
                    price: price.0,
                    quantity: *qty,
                })
                .collect();

            let asks: Vec<OrderbookLevel> = book
                .asks
                .iter()
                .take(limit)
                .map(|(price, qty)| OrderbookLevel {
                    price: price.0,
                    quantity: *qty,
                })
                .collect();

            NormalizedOrderbook {
                symbol,
                exchange,
                timestamp: book.last_update,
                bids,
                asks,
            }
        })
    }
}
