use dashmap::DashMap;
use rust_decimal::Decimal;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Exchange {
    Hyperliquid,
    Bitbank,
    Kraken,
    Gmo,
}

impl fmt::Display for Exchange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Clone, Default)]
pub struct SymbolData {
    pub bid: Decimal,
    pub ask: Decimal,
    pub last_price: Decimal,
    pub funding_rate: Decimal,
    pub timestamp: u64,
}

#[derive(Clone)]
pub struct MarketStore {
    // キー: (取引所, シンボル名)
    pub data: Arc<DashMap<(Exchange, String), SymbolData>>,
}

impl MarketStore {
    pub fn new() -> Self {
        Self {
            data: Arc::new(DashMap::new()),
        }
    }

    pub fn update_market_data(&self, exchange: Exchange, symbol: &str, bid: Decimal, ask: Decimal, last: Decimal) {
        let timestamp = current_timestamp();
        self.data
            .entry((exchange, symbol.to_string()))
            .and_modify(|d| {
                d.bid = bid;
                d.ask = ask;
                d.last_price = last;
                d.timestamp = timestamp;
            })
            .or_insert_with(|| SymbolData {
                bid, ask, last_price: last, timestamp, ..Default::default()
            });
    }

    pub fn update_funding_rate(&self, exchange: Exchange, symbol: &str, funding: Decimal) {
        let timestamp = current_timestamp();
        self.data
            .entry((exchange, symbol.to_string()))
            .and_modify(|d| {
                d.funding_rate = funding;
                d.timestamp = timestamp;
            })
            .or_insert_with(|| SymbolData {
                funding_rate: funding, timestamp, ..Default::default()
            });
    }

    pub fn get_symbol_data(&self, exchange: Exchange, symbol: &str) -> Option<SymbolData> {
        self.data.get(&(exchange, symbol.to_string())).map(|entry| entry.clone())
    }

    pub fn get_fx_rate(&self, symbol: &str) -> Option<Decimal> {
        self.get_symbol_data(Exchange::Kraken, symbol).map(|d| d.last_price)
    }

    pub fn get_market_data(&self, exchange: Exchange, symbol: &str) -> Option<SymbolData> {
        self.get_symbol_data(exchange, symbol)
    }
}

fn current_timestamp() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
}