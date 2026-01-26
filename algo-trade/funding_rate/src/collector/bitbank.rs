use crate::store::{MarketStore, Exchange};
use futures_util::{SinkExt, StreamExt};
use log::{error, info, debug};
use rust_decimal::Decimal;
use serde::Deserialize;
use serde_json::Value;
use std::str::FromStr;
use std::time::Duration;
use tokio::time::sleep;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

// Bitbank Socket.IO Endpoint
// EIO=4 (Engine.IO v4), transport=websocket
const WS_URL: &str = "wss://stream.bitbank.cc/socket.io/?EIO=4&transport=websocket";

#[derive(Deserialize, Debug)]
struct BitbankTickerData {
    sell: String,
    buy: String,
    last: String,
    // timestamp: u64,
}

pub async fn start_collection(store: MarketStore) {
    loop {
        info!("[Bitbank] Connecting to WebSocket (Socket.IO)...");

        match connect_async(WS_URL).await {
            Ok((ws_stream, _)) => {
                info!("[Bitbank] WebSocket connected");
                if let Err(e) = handle_socket_io(ws_stream, &store).await {
                    error!("[Bitbank] Connection lost: {}", e);
                }
            }
            Err(e) => {
                error!("[Bitbank] Connect failed: {}", e);
            }
        }

        sleep(Duration::from_secs(5)).await;
    }
}

async fn handle_socket_io(
    stream: tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    store: &MarketStore
) -> Result<(), Box<dyn std::error::Error>> {
    let (mut write, mut read) = stream.split();

    // Socket.IO Handshake Flow:
    // 1. Receive Open packet (0{...})
    // 2. Send Connect packet (40)
    // 3. Receive Connect packet (40{...})
    // 4. Send Subscribe (42["join-room", ...])
    // 5. Loop (Handle 2=Ping -> Send 3=Pong, Handle 42=Message)

    // 購読するルーム
    let rooms = vec![
        "ticker_btc_jpy",
        // "ticker_eth_jpy", // 必要なら追加
    ];

    while let Some(msg) = read.next().await {
        let msg = msg?;
        match msg {
            Message::Text(text) => {
                // Engine.IO packet types:
                // '0': Open
                // '2': Ping
                // '3': Pong
                // '4': Message (Socket.IO types inside)
                
                if text.starts_with('0') {
                    // Open packet: {"sid":"...", "pingInterval":25000, ...}
                    debug!("[Bitbank] Received Open packet: {}", text);
                    
                    // Namespaceへの接続 (40)
                    write.send(Message::Text("40".to_string())).await?;
                    info!("[Bitbank] Sent Namespace Connect (40)");

                } else if text.starts_with("40") {
                    // Connected to Namespace
                    debug!("[Bitbank] Namespace connected: {}", text);

                    // ルームへのJoin (42["join-room", "room_name"])
                    for room in &rooms {
                        let join_payload = format!("42[\"join-room\",\"{}\"]", room);
                        write.send(Message::Text(join_payload)).await?;
                        info!("[Bitbank] Joined room: {}", room);
                    }

                } else if text.starts_with('2') {
                    // Ping受信 -> Pong (3) を返す
                    // debug!("[Bitbank] Ping received");
                    write.send(Message::Text("3".to_string())).await?;
                    // debug!("[Bitbank] Pong sent");

                } else if text.starts_with("42") {
                    // Event Message: 42["message", {...}]
                    // 最初の2文字 "42" をスキップしてJSON配列としてパース
                    let json_str = &text[2..];
                    if let Ok(val) = serde_json::from_str::<Value>(json_str) {
                        // 配列形式: ["message", { "room_name": "...", "message": { "data": ... } }]
                        if let Some(event_name) = val.get(0).and_then(|v| v.as_str()) {
                            if event_name == "message" {
                                if let Some(payload) = val.get(1) {
                                    process_data(payload, store);
                                }
                            }
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

fn process_data(payload: &Value, store: &MarketStore) {
    // payload structure:
    // {
    //   "room_name": "ticker_btc_jpy",
    //   "message": {
    //     "data": { "sell": "...", "buy": "...", "last": "...", ... }
    //   }
    // }

    let room_name = payload["room_name"].as_str().unwrap_or("");
    
    // 通貨ペアの判定
    let symbol = if room_name.contains("btc_jpy") {
        "BTC"
    } else {
        return; // 未知のペア
    };

    if let Some(data) = payload.get("message").and_then(|m| m.get("data")) {
        // Tickerデータのパース
        // Bitbankは文字列で数値を返してくる
        if let (Some(sell_str), Some(buy_str), Some(last_str)) = (
            data["sell"].as_str(),
            data["buy"].as_str(),
            data["last"].as_str(),
        ) {
            if let (Ok(ask), Ok(bid), Ok(last)) = (
                Decimal::from_str(sell_str),
                Decimal::from_str(buy_str),
                Decimal::from_str(last_str),
            ) {
                store.update_market_data(
                    Exchange::Bitbank,
                    symbol,
                    bid,
                    ask,
                    last
                );
                // info!("[Bitbank WS] Updated {}: {}", symbol, last);
            }
        }
    }
}