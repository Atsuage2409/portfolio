use crate::store::{MarketStore, Exchange};
use futures_util::{SinkExt, StreamExt};
use log::{error, info, warn};
use rust_decimal::Decimal;
use serde_json::{json, Value};
use std::str::FromStr;
use std::time::Duration;
use tokio::time::sleep;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

const KRAKEN_WS_URL: &str = "wss://ws.kraken.com";

pub async fn start_collection(store: MarketStore) {
    loop {
        info!("[Kraken] Connecting to WebSocket...");

        match connect_async(KRAKEN_WS_URL).await {
            Ok((ws_stream, _)) => {
                info!("[Kraken] WebSocket connected");
                if let Err(e) = handle_socket(ws_stream, &store).await {
                    error!("[Kraken] Connection error: {}", e);
                }
            }
            Err(e) => {
                error!("[Kraken] Connection failed: {}", e);
            }
        }
        
        warn!("[Kraken] Reconnecting in 5 seconds...");
        sleep(Duration::from_secs(5)).await;
    }
}

async fn handle_socket(
    stream: tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    store: &MarketStore
) -> Result<(), Box<dyn std::error::Error>> {
    let (mut write, mut read) = stream.split();

    // 購読メッセージ送信 (USD/JPYのTicker)
    let subscribe_msg = json!({
        "event": "subscribe",
        "pair": ["USD/JPY"],
        "subscription": {
            "name": "ticker"
        }
    });

    write.send(Message::Text(subscribe_msg.to_string())).await?;
    info!("[Kraken] Subscribed to USD/JPY ticker");

    while let Some(msg) = read.next().await {
        let msg = msg?;
        match msg {
            Message::Text(text) => {
                // Krakenのメッセージ処理
                // 配列形式: [ChannelID, {Data}, ChannelName, Pair]
                // イベント形式: {"event": ...}
                
                let v: Value = serde_json::from_str(&text)?;
                
                if v.is_array() {
                    // データメッセージ
                    if let Some(data) = v.get(1) {
                        process_ticker_data(data, store);
                    }
                } else if v.is_object() {
                    // イベントメッセージ (heartbeat, systemStatus等)
                    if let Some(event) = v.get("event").and_then(|e| e.as_str()) {
                        if event == "heartbeat" {
                            // 何もしなくてOK
                        } else if event == "systemStatus" {
                            // info!("[Kraken] System Status: {:?}", v);
                        }
                    }
                }
            }
            Message::Close(_) => return Ok(()),
            _ => {}
        }
    }
    Ok(())
}

fn process_ticker_data(data: &Value, store: &MarketStore) {
    // Tickerフォーマット:
    // {
    //   "a": ["Ask Price", "Whole Lot Vol", "Lot Vol"],
    //   "b": ["Bid Price", "Whole Lot Vol", "Lot Vol"],
    //   "c": ["Last Price", "Lot Vol"],
    //   ...
    // }

    // Last Price (c[0]) を取得
    if let Some(last_arr) = data.get("c").and_then(|c| c.as_array()) {
        if let Some(price_str) = last_arr.get(0).and_then(|p| p.as_str()) {
            if let Ok(price) = Decimal::from_str(price_str) {
                // Bid/Askも取れるが、為替レートとしてはLastかMidで十分
                // ここでは便宜上すべてLast Priceを入れておく
                store.update_market_data(
                    Exchange::Kraken,
                    "USD_JPY",
                    price, // bid
                    price, // ask
                    price  // last
                );
                // debug!("[Kraken] Updated USD/JPY: {}", price);
            }
        }
    }
}