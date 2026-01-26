use crate::store::{MarketStore, Exchange};
use futures_util::{SinkExt, StreamExt};
use log::{error, info, warn};
use serde_json::{json, Value};
use serde::Deserialize;
use std::fs;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time::sleep;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

const WS_URL: &str = "wss://api.hyperliquid.xyz/ws";
const RECONNECT_DELAY_SECS: u64 = 5;

/// Spot資産のマッピング情報 (name -> WS ID)
type SpotMapping = Arc<Mutex<HashMap<String, String>>>;

/// Hyperliquidのデータ収集を開始するメイン関数
pub async fn start_collection(symbols: Vec<String>, store: MarketStore) {
    // 設定ファイルからの静的ID読み込み
    let spot_mapping = Arc::new(Mutex::new(HashMap::new()));
    load_spot_ids(&spot_mapping);
    
    loop {
        info!("[Hyperliquid] Connecting to {}...", WS_URL);

        match connect_async(WS_URL).await {
            Ok((ws_stream, _)) => {
                info!("[Hyperliquid] WebSocket connected successfully");
                
                if let Err(e) = run_websocket(ws_stream, &symbols, &store, &spot_mapping).await {
                    error!("[Hyperliquid] WebSocket error: {}", e);
                }
            }
            Err(e) => {
                error!("[Hyperliquid] Connection failed: {}", e);
            }
        }

        warn!("[Hyperliquid] Reconnecting in {} seconds...", RECONNECT_DELAY_SECS);
        sleep(Duration::from_secs(RECONNECT_DELAY_SECS)).await;
    }
}

/// WebSocket接続のメインループ
async fn run_websocket(
    ws_stream: tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    symbols: &[String],
    store: &MarketStore,
    spot_mapping: &SpotMapping,
) -> Result<(), Box<dyn std::error::Error>> {
    let (mut write, mut read) = ws_stream.split();

    // サブスクリプション送信
    for sym in symbols {
        // Perp購読 (元のシンボル名で)
        let sub_l2 = json!({
            "method": "subscribe",
            "subscription": { "type": "l2Book", "coin": sym }
        });
        
        let sub_ctx = json!({
            "method": "subscribe",
            "subscription": { "type": "activeAssetCtx", "coin": sym }
        });

        write.send(Message::Text(sub_l2.to_string())).await
            .map_err(|e| format!("Failed to subscribe l2Book for {}: {}", sym, e))?;
        
        write.send(Message::Text(sub_ctx.to_string())).await
            .map_err(|e| format!("Failed to subscribe activeAssetCtx for {}: {}", sym, e))?;
        
        info!("[Hyperliquid] Subscribed to {} PERP (l2Book + activeAssetCtx)", sym);
        
        // Spot購読 (マッピングされたIDで)
        let spot_id_opt = spot_mapping.lock().unwrap().get(sym).cloned();
        if let Some(spot_id) = spot_id_opt {
            let sub_spot = json!({
                "method": "subscribe",
                "subscription": { "type": "l2Book", "coin": spot_id.clone() }
            });
            
            write.send(Message::Text(sub_spot.to_string())).await
                .map_err(|e| format!("Failed to subscribe l2Book for spot {}: {}", spot_id, e))?;
            
            info!("[Hyperliquid] Subscribed to {} SPOT ({})", sym, spot_id);
        } else {
            warn!("[Hyperliquid] No spot ID found for {}", sym);
        }
    }

    // メッセージ受信ループ
    while let Some(msg_res) = read.next().await {
        match msg_res {
            Ok(Message::Text(text)) => {
                handle_message(&text, store, spot_mapping);
            }
            Ok(Message::Ping(_)) => {
                // Tungsteniteが自動でPongを返す
            }
            Ok(Message::Close(frame)) => {
                warn!("[Hyperliquid] Server closed connection: {:?}", frame);
                return Ok(());
            }
            Err(e) => {
                return Err(Box::new(e));
            }
            _ => {}
        }
    }

    Ok(())
}

/// 受信したJSONメッセージを処理
fn handle_message(text: &str, store: &MarketStore, spot_mapping: &SpotMapping) {
    let v: Value = match serde_json::from_str(text) {
        Ok(v) => v,
        Err(_) => return, // パースエラーは無視
    };

    if v["channel"] != "l2Book" {
        // activeAssetCtx も処理
        if v["channel"] == "activeAssetCtx" {
            process_asset_ctx(&v["data"], store);
        }
        return;
    }

    let data = &v["data"];
    let coin_raw = match data["coin"].as_str() {
        Some(c) => c,
        None => return,
    };

    if let Some((bid_decimal, ask_decimal)) = parse_bid_ask(data) {
        let mid_decimal = (bid_decimal + ask_decimal) / rust_decimal::Decimal::from(2);
        
        // coin_rawが@で始まる場合はSpot、そうでない場合はPerp
        if coin_raw.starts_with('@') {
            // Spotの場合、対応するPerpシンボル名を逆引き
            let mapping = spot_mapping.lock().unwrap();
            if let Some((perp_name, _)) = mapping.iter().find(|(_, id)| *id == coin_raw) {
                let spot_key = format!("{}_SPOT", perp_name);
                store.update_market_data(Exchange::Hyperliquid, &spot_key, bid_decimal, ask_decimal, mid_decimal);
            }
        } else {
            // Perpの場合はそのまま
            store.update_market_data(Exchange::Hyperliquid, coin_raw, bid_decimal, ask_decimal, mid_decimal);
        }
    }
}

/// l2Book データからbid/askを直接Decimalで取得
fn parse_bid_ask(data: &Value) -> Option<(rust_decimal::Decimal, rust_decimal::Decimal)> {
    use rust_decimal::Decimal;
    use std::str::FromStr;
    
    let levels = &data["levels"];
    if let (Some(bids), Some(asks)) = (levels[0].as_array(), levels[1].as_array()) {
        if bids.is_empty() || asks.is_empty() {
            return None;
        }
        let bid_str = bids[0]["px"].as_str()?;
        let ask_str = asks[0]["px"].as_str()?;
        let best_bid = Decimal::from_str(bid_str).ok()?;
        let best_ask = Decimal::from_str(ask_str).ok()?;
        return Some((best_bid, best_ask));
    }
    None
}

/// 市場コンテキスト (activeAssetCtx) の処理
fn process_asset_ctx(data: &Value, store: &MarketStore) {
    let coin = match data["coin"].as_str() {
        Some(c) => c,
        None => return,
    };
    
    if let Some(ctx) = data.get("ctx") {
        if let Some(funding_val) = ctx.get("funding") {
            if let Some(funding_str) = funding_val.as_str() {
                if let Ok(decimal_funding) = rust_decimal::Decimal::from_str(funding_str) {
                    store.update_funding_rate(Exchange::Hyperliquid, coin, decimal_funding);
                }
            }
        }
    }
}

// --- 設定ファイル読み込み ---

#[derive(Debug, Deserialize)]
struct SpotIdsConfig {
    spot: HashMap<String, u32>,
}

/// spot_ids.toml から設定を読み込み、マッピングを設定
fn load_spot_ids(mapping: &SpotMapping) {
    let path = "spot_ids.toml";
    let mut spot_ids: HashMap<String, String> = HashMap::new();

    match fs::read_to_string(path) {
        Ok(contents) => {
            match toml::from_str::<SpotIdsConfig>(&contents) {
                Ok(conf) => {
                    for (sym, idx) in conf.spot.into_iter() {
                        spot_ids.insert(sym, format!("@{}", idx));
                    }
                    info!("[Hyperliquid] Loaded spot IDs from {}: {:?}", path, spot_ids);
                }
                Err(e) => {
                    warn!("[Hyperliquid] Failed to parse {}: {}", path, e);
                }
            }
        }
        Err(_) => {
            // 既定値を使用 (ユーザ指定)
            spot_ids.insert("BTC".to_string(), "@142".to_string());
            spot_ids.insert("ETH".to_string(), "@151".to_string());
            spot_ids.insert("SOL".to_string(), "@156".to_string());
            spot_ids.insert("HYPE".to_string(), "@107".to_string());
            info!("[Hyperliquid] Config {} not found, using defaults: {:?}", path, spot_ids);
        }
    }

    let mut map = mapping.lock().unwrap();
    for (k, v) in spot_ids.into_iter() {
        map.insert(k, v);
    }
}