// 修正版のHyperliquid コレクター実装テスト
use futures_util::{SinkExt, StreamExt};
use log::{debug, error, info, warn};
use rust_decimal::Decimal;
use serde_json::Value;
use std::str::FromStr;
use std::time::Duration;
use std::sync::Arc;
use tokio::time::sleep;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use dashmap::DashMap;
use std::time::{SystemTime, UNIX_EPOCH};

const WS_URL: &str = "wss://api.hyperliquid.xyz/ws";

#[derive(Debug, Clone, Default)]
struct SymbolData {
    pub bid: Decimal,
    pub ask: Decimal,
    pub last_price: Decimal,
    pub funding_rate: Decimal,
    pub timestamp: u64,
}

#[derive(Clone)]
struct MarketStore {
    pub data: Arc<DashMap<String, SymbolData>>,
}

impl MarketStore {
    fn new() -> Self {
        Self {
            data: Arc::new(DashMap::new()),
        }
    }

    fn update_market_data(&self, symbol: &str, bid: Decimal, ask: Decimal, last: Decimal) {
        let timestamp = current_timestamp();
        
        self.data
            .entry(symbol.to_string())
            .and_modify(|d| {
                d.bid = bid;
                d.ask = ask;
                d.last_price = last;
                d.timestamp = timestamp;
            })
            .or_insert_with(|| SymbolData {
                bid,
                ask,
                last_price: last,
                timestamp,
                ..Default::default()
            });
    }

    fn update_funding_rate(&self, symbol: &str, funding: Decimal) {
        let timestamp = current_timestamp();
        
        self.data
            .entry(symbol.to_string())
            .and_modify(|d| {
                d.funding_rate = funding;
                d.timestamp = timestamp;
            })
            .or_insert_with(|| SymbolData {
                funding_rate: funding,
                timestamp,
                ..Default::default()
            });
    }

    fn get_symbol_data(&self, symbol: &str) -> Option<SymbolData> {
        self.data.get(symbol).map(|entry| entry.clone())
    }
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[tokio::main]
async fn main() {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug)
        .init();

    info!("Starting FIXED Hyperliquid collector test...");

    let store = MarketStore::new();
    let store_clone = store.clone();

    let symbols = vec![
        "BTC".to_string(),
        "ETH".to_string(),
        "SOL".to_string(),
    ];

    // データ収集を別タスクで開始
    let collector_handle = tokio::spawn(async move {
        start_collection(symbols, store_clone).await;
    });

    // 10秒待ってデータを確認
    info!("Waiting 10 seconds for data collection...");
    sleep(Duration::from_secs(10)).await;

    // データ取得の確認
    info!("\n========== Market Data ==========");
    info!("Store has {} symbols", store.data.len());
    
    // すべてのキーを表示
    for entry in store.data.iter() {
        debug!("Found symbol in store: {}", entry.key());
    }
    
    for symbol in &["BTC", "ETH", "SOL"] {
        if let Some(data) = store.get_symbol_data(symbol) {
            info!("\n{} Data:", symbol);
            info!("  Bid:          {}", data.bid);
            info!("  Ask:          {}", data.ask);
            info!("  Last Price:   {}", data.last_price);
            info!("  Funding Rate: {}", data.funding_rate);
            info!("  Timestamp:    {}", data.timestamp);
            
            // バリデーション
            if data.bid.is_zero() || data.ask.is_zero() {
                error!("  ⚠️  WARNING: Bid or Ask is zero!");
            } else if data.bid >= data.ask {
                error!("  ⚠️  WARNING: Bid >= Ask (invalid orderbook)");
            } else {
                info!("  ✅ Price data looks valid");
            }
            
            if data.funding_rate.is_zero() {
                warn!("  ⚠️  WARNING: Funding rate is zero (might not be received yet)");
            } else {
                info!("  ✅ Funding rate received");
            }
        } else {
            error!("{}: No data available", symbol);
        }
    }

    info!("\nTest completed successfully!");
    
    // コレクターを停止
    collector_handle.abort();
}

async fn start_collection(symbols: Vec<String>, store: MarketStore) {
    info!("Connecting to {}...", WS_URL);

    match connect_async(WS_URL).await {
        Ok((ws_stream, _)) => {
            info!("WebSocket connected successfully");
            
            if let Err(e) = run_websocket(ws_stream, &symbols, &store).await {
                error!("WebSocket error: {}", e);
            }
        }
        Err(e) => {
            error!("Connection failed: {}", e);
        }
    }
}

async fn run_websocket(
    ws_stream: tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    symbols: &[String],
    store: &MarketStore,
) -> Result<(), Box<dyn std::error::Error>> {
    let (mut write, mut read) = ws_stream.split();

    // サブスクリプション送信
    for sym in symbols {
        let sub_l2 = serde_json::json!({
            "method": "subscribe",
            "subscription": { "type": "l2Book", "coin": sym }
        });
        
        let sub_ctx = serde_json::json!({
            "method": "subscribe",
            "subscription": { "type": "activeAssetCtx", "coin": sym }
        });

        write.send(Message::Text(sub_l2.to_string())).await
            .map_err(|e| format!("Failed to subscribe l2Book for {}: {}", sym, e))?;
        
        write.send(Message::Text(sub_ctx.to_string())).await
            .map_err(|e| format!("Failed to subscribe activeAssetCtx for {}: {}", sym, e))?;
        
        info!("Subscribed to {} (l2Book + activeAssetCtx)", sym);
    }

    // メッセージ受信ループ
    while let Some(msg_res) = read.next().await {
        match msg_res {
            Ok(Message::Text(text)) => {
                handle_message(&text, store);
            }
            Ok(Message::Ping(_)) => {
                debug!("Received ping");
            }
            Ok(Message::Pong(_)) => {
                debug!("Received pong");
            }
            Ok(Message::Close(frame)) => {
                warn!("Server closed connection: {:?}", frame);
                return Ok(());
            }
            Ok(_) => {}
            Err(e) => {
                return Err(Box::new(e));
            }
        }
    }

    Ok(())
}

fn handle_message(text: &str, store: &MarketStore) {
    let v: Value = match serde_json::from_str(text) {
        Ok(v) => v,
        Err(e) => {
            error!("JSON parse error: {} | Raw: {}", e, text);
            return;
        }
    };

    if let Some(channel) = v.get("channel").and_then(|c| c.as_str()) {
        if let Some(data) = v.get("data") {
            match channel {
                "l2Book" => process_l2_book(data, store),
                "activeAssetCtx" => process_asset_ctx(data, store),
                _ => {
                    debug!("Unhandled channel: {}", channel);
                }
            }
        }
    }
}

// 修正版: 板情報の処理
fn process_l2_book(data: &Value, store: &MarketStore) {
    let coin = match data.get("coin").and_then(|s| s.as_str()) {
        Some(c) => c,
        None => {
            warn!("l2Book: missing 'coin' field");
            return;
        }
    };

    // levelsは [[bid_levels], [ask_levels]] の形式
    // 各レベルは {"n": count, "px": "price", "sz": "size"} のオブジェクト
    let levels = match data.get("levels").and_then(|l| l.as_array()) {
        Some(l) if l.len() >= 2 => l,
        _ => {
            warn!("l2Book: invalid 'levels' format for {}", coin);
            return;
        }
    };

    let bids = levels[0].as_array();
    let asks = levels[1].as_array();

    let best_bid = get_best_price_fixed(bids);
    let best_ask = get_best_price_fixed(asks);

    match (best_bid, best_ask) {
        (Some(bid), Some(ask)) => {
            let mid_price = (bid + ask) / Decimal::from(2);
            
            store.update_market_data(coin, bid, ask, mid_price);
            
            debug!("{} | Bid: {}, Ask: {}, Mid: {}", 
                   coin, bid, ask, mid_price);
        }
        _ => {
            warn!("Could not parse bid/ask for {}", coin);
        }
    }
}

fn process_asset_ctx(data: &Value, store: &MarketStore) {
    let coin = match data.get("coin").and_then(|s| s.as_str()) {
        Some(c) => c,
        None => {
            warn!("activeAssetCtx: missing 'coin' field");
            return;
        }
    };

    if let Some(ctx) = data.get("ctx") {
        if let Some(funding_val) = ctx.get("funding") {
            if let Some(funding_rate) = parse_decimal(funding_val) {
                store.update_funding_rate(coin, funding_rate);
                
                debug!("{} | Funding Rate: {}", coin, funding_rate);
            } else {
                warn!("Could not parse funding rate for {}", coin);
            }
        }
    }
}

// 修正版: オブジェクト形式からベスト価格を取得
fn get_best_price_fixed(level_arr: Option<&Vec<Value>>) -> Option<Decimal> {
    level_arr?
        .first()?
        .as_object()?
        .get("px")
        .and_then(parse_decimal)
}

fn parse_decimal(v: &Value) -> Option<Decimal> {
    match v {
        Value::String(s) => Decimal::from_str(s).ok(),
        Value::Number(n) => Decimal::from_str(&n.to_string()).ok(),
        _ => None,
    }
}
