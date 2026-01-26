use futures_util::{SinkExt, StreamExt};
use log::{info, error, debug};
use serde_json::Value;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

const WS_URL: &str = "wss://api.hyperliquid.xyz/ws";

#[tokio::main]
async fn main() {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug)
        .init();

    info!("Connecting to Hyperliquid WebSocket for debugging...");

    match connect_async(WS_URL).await {
        Ok((ws_stream, _)) => {
            info!("Connected successfully!");
            
            let (mut write, mut read) = ws_stream.split();

            // BTCのみをサブスクライブ
            let sub_l2 = serde_json::json!({
                "method": "subscribe",
                "subscription": { "type": "l2Book", "coin": "BTC" }
            });
            
            let sub_ctx = serde_json::json!({
                "method": "subscribe",
                "subscription": { "type": "activeAssetCtx", "coin": "BTC" }
            });

            write.send(Message::Text(sub_l2.to_string())).await.unwrap();
            write.send(Message::Text(sub_ctx.to_string())).await.unwrap();
            
            info!("Subscribed to BTC");

            let mut msg_count = 0;
            let max_messages = 20; // 最初の20メッセージだけ確認

            while let Some(msg_res) = read.next().await {
                if msg_count >= max_messages {
                    info!("Received {} messages, exiting...", msg_count);
                    break;
                }

                match msg_res {
                    Ok(Message::Text(text)) => {
                        msg_count += 1;
                        
                        let v: Value = match serde_json::from_str(&text) {
                            Ok(v) => v,
                            Err(e) => {
                                error!("JSON parse error: {}", e);
                                continue;
                            }
                        };

                        if let Some(channel) = v.get("channel").and_then(|c| c.as_str()) {
                            info!("\n========== Message #{} ==========", msg_count);
                            info!("Channel: {}", channel);
                            
                            match channel {
                                "l2Book" => {
                                    info!("L2Book Data Structure:");
                                    info!("{}", serde_json::to_string_pretty(&v).unwrap());
                                }
                                "activeAssetCtx" => {
                                    info!("ActiveAssetCtx Data Structure:");
                                    info!("{}", serde_json::to_string_pretty(&v).unwrap());
                                }
                                "subscriptionResponse" => {
                                    debug!("Subscription response: {}", text);
                                }
                                _ => {
                                    info!("Other channel data: {}", text);
                                }
                            }
                        } else {
                            debug!("Message without channel: {}", text);
                        }
                    }
                    Ok(Message::Close(_)) => {
                        info!("Connection closed by server");
                        break;
                    }
                    Err(e) => {
                        error!("WebSocket error: {}", e);
                        break;
                    }
                    _ => {}
                }
            }

            info!("Debug session completed!");
        }
        Err(e) => {
            error!("Connection failed: {}", e);
        }
    }
}
