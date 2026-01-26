use crate::store::{MarketStore, Exchange};
use futures_util::{SinkExt, StreamExt};
use log::{error, info, warn};
use rust_decimal::Decimal;
use serde_json::json;
use std::str::FromStr;
use std::time::Duration;
use tokio::time::sleep;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

const GMO_WS_URL: &str = "wss://api.coin.z.com/ws/public/v1";

pub async fn start_collection(symbols: Vec<String>, store: MarketStore) {
    loop {
        info!("[GMO] Connecting to WebSocket...");

        match connect_async(GMO_WS_URL).await {
            Ok((ws_stream, _)) => {
                info!("[GMO] WebSocket connected");
                if let Err(e) = handle_socket(ws_stream, &symbols, &store).await {
                    error!("[GMO] Connection error: {}", e);
                }
            }
            Err(e) => {
                error!("[GMO] Connection failed: {}", e);
            }
        }
        
        warn!("[GMO] Reconnecting in 5 seconds...");
        sleep(Duration::from_secs(5)).await;
    }
}

async fn handle_socket(
    stream: tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    symbols: &[String],
    store: &MarketStore
) -> Result<(), Box<dyn std::error::Error>> {
    let (mut write, mut read) = stream.split();

    // 1. 為替の購読
    write.send(Message::Text(json!({
        "command": "subscribe", "channel": "ticker", "symbol": "USD_JPY"
    }).to_string())).await?;

    // 2. 仮想通貨（現物・レバレッジ両方）の購読
    for sym in symbols {
        // 現物 (例: BTC)
        write.send(Message::Text(json!({
            "command": "subscribe", "channel": "ticker", "symbol": sym
        }).to_string())).await?;
        
        // レバレッジ (例: BTC_JPY)
        write.send(Message::Text(json!({
            "command": "subscribe", "channel": "ticker", "symbol": format!("{}_JPY", sym)
        }).to_string())).await?;
    }

    while let Some(msg) = read.next().await {
        let msg = msg?;
        if let Message::Text(text) = msg {
            let v: serde_json::Value = serde_json::from_str(&text)?;
            
            if v["channel"] == "ticker" {
                let symbol_raw = v["symbol"].as_str().unwrap_or("");
                let bid_str = v["bid"].as_str().unwrap_or("0");
                let ask_str = v["ask"].as_str().unwrap_or("0");
                
                if let (Ok(bid), Ok(ask)) = (Decimal::from_str(bid_str), Decimal::from_str(ask_str)) {
                    let mid = (bid + ask) / Decimal::from(2);
                    
                    // --- 命名規則のマッピングロジック ---
                    let (store_key, is_spot) = if symbol_raw == "USD_JPY" {
                        ("USD_JPY".to_string(), false)
                    } else if symbol_raw.contains("_JPY") {
                        // レバレッジ (BTC_JPY -> BTC)
                        (symbol_raw.replace("_JPY", ""), false)
                    } else {
                        // 現物 (BTC -> BTC_SPOT)
                        (format!("{}_SPOT", symbol_raw), true)
                    };

                    store.update_market_data(
                        Exchange::Gmo,
                        &store_key,
                        bid,
                        ask,
                        mid
                    );
                }
            }
        }
    }
    Ok(())
}