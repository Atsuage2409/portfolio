use crate::store::MarketStore;
use futures_util::{SinkExt, StreamExt};
use log::{debug, error, info, warn};
use reqwest::Client;
use rust_decimal::Decimal;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time::sleep;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

const WS_URL: &str = "wss://api.hyperliquid.xyz/ws";
const API_URL: &str = "https://api.hyperliquid.xyz/info";
const RECONNECT_DELAY_SECS: u64 = 5;

/// Spot資産のマッピング情報 (name -> WS ID)
type SpotMapping = Arc<Mutex<HashMap<String, String>>>;

/// Hyperliquidのデータ収集を開始するメイン関数
pub async fn start_collection(symbols: Vec<String>, store: MarketStore) {
    // RESTクライアントの作成
    let client = Client::new();
    
    // Spot資産のマッピングを取得
    let spot_mapping = Arc::new(Mutex::new(HashMap::new()));
    if let Err(e) = fetch_spot_mapping(&client, &spot_mapping).await {
        error!("[Hyperliquid] Failed to fetch spot mapping: {}", e);
    }
    
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
        if let Some(spot_id) = spot_mapping.lock().unwrap().get(sym) {
            let sub_spot = json!({
                "method": "subscribe",
                "subscription": { "type": "l2Book", "coin": spot_id.clone() }
            });
            
            write.send(Message::Text(sub_spot.to_string())).await
                .map_err(|e| format!("Failed to subscribe l2Book for spot {}: {}", spot_id, e))?;
            
            info!("[Hyperliquid] Subscribed to {} SPOT ({})", sym, spot_id);
        }
    }

    // メッセージ受信ループ
    while let Some(msg_res) = read.next().await {
        match msg_res {
            Ok(Message::Text(text)) => {
                handle_message(&text, store, spot_mapping);
            }
            Ok(Message::Ping(_)) => {
                debug!("[Hyperliquid] Received ping");
                // Tungsteniteが自動でPongを返す
            }
            Ok(Message::Pong(_)) => {
                debug!("[Hyperliquid] Received pong");
            }
            Ok(Message::Close(frame)) => {
                warn!("[Hyperliquid] Server closed connection: {:?}", frame);
                return Ok(());
            }
            Ok(Message::Binary(_)) => {
                warn!("[Hyperliquid] Unexpected binary message");
            }
            Ok(Message::Frame(_)) => {
                // 低レベルフレーム、通常無視
            }
            Err(e) => {
                return Err(Box::new(e));
            }
        }
    }

    Ok(())
}

/// 受信したJSONメッセージを振り分ける
fn handle_message(text: &str, store: &MarketStore, spot_mapping: &SpotMapping) {
    let v: Value = match serde_json::from_str(text) {
        Ok(v) => v,
        Err(e) => {
            // パースエラーは頻繁に起きないはずなので、ログレベルはerrorのまま
            error!("[Hyperliquid] JSON parse error: {} | Raw: {}", e, text);
            return;
        }
    };

    // チャンネル判定
    if let Some(channel) = v.get("channel").and_then(|c| c.as_str()) {
        if let Some(data) = v.get("data") {
            match channel {
                "l2Book" => process_l2_book(data, store, spot_mapping),
                "activeAssetCtx" => process_asset_ctx(data, store),
                _ => {
                    debug!("[Hyperliquid] Unhandled channel: {}", channel);
                }
            }
        }
    }
}

/// 板情報 (l2Book) の処理
fn process_l2_book(data: &Value, store: &MarketStore, spot_mapping: &SpotMapping) {
    let coin_raw = match data.get("coin").and_then(|s| s.as_str()) {
        Some(c) => c,
        None => {
            warn!("[Hyperliquid] l2Book: missing 'coin' field");
            return;
        }
    };

    // levelsは [[bid_levels], [ask_levels]] の形式
    let levels = match data.get("levels").and_then(|l| l.as_array()) {
        Some(l) if l.len() >= 2 => l,
        _ => {
            warn!("[Hyperliquid] l2Book: invalid 'levels' format for {}", coin_raw);
            return;
        }
    };

    let bids = levels[0].as_array();
    let asks = levels[1].as_array();

    let best_bid = get_best_price(bids);
    let best_ask = get_best_price(asks);

    match (best_bid, best_ask) {
        (Some(bid), Some(ask)) => {
            let mid_price = (bid + ask) / Decimal::from(2);
            
            // coin_rawが@で始まる場合はSpot、そうでない場合はPerp
            let (coin, is_spot) = if coin_raw.starts_with('@') {
                // Spotの場合、対応するPerpシンボル名を逆引き
                let mapping = spot_mapping.lock().unwrap();
                let perp_name = mapping.iter()
                    .find(|(_, id)| *id == coin_raw)
                    .map(|(name, _)| name.clone())
                    .unwrap_or_else(|| coin_raw.to_string());
                (perp_name, true)
            } else {
                // Perpの場合はそのまま
                (coin_raw.to_string(), false)
            };
            
            if is_spot {
                // Spot価格を別途保存（キー名に"_SPOT"を付加）
                let spot_key = format!("{}_SPOT", coin);
                store.update_market_data(&spot_key, bid, ask, mid_price);
                debug!("[Hyperliquid] {} SPOT | Bid: {}, Ask: {}, Mid: {}", 
                       coin, bid, ask, mid_price);
            } else {
                // Perp価格を通常通り保存
                store.update_market_data(&coin, bid, ask, mid_price);
                debug!("[Hyperliquid] {} PERP | Bid: {}, Ask: {}, Mid: {}", 
                       coin, bid, ask, mid_price);
            }
        }
        _ => {
            warn!("[Hyperliquid] Could not parse bid/ask for {}", coin_raw);
        }
    }
}

/// 市場コンテキスト (activeAssetCtx) の処理
fn process_asset_ctx(data: &Value, store: &MarketStore) {
    let coin = match data.get("coin").and_then(|s| s.as_str()) {
        Some(c) => c,
        None => {
            warn!("[Hyperliquid] activeAssetCtx: missing 'coin' field");
            return;
        }
    };

    if let Some(ctx) = data.get("ctx") {
        if let Some(funding_val) = ctx.get("funding") {
            if let Some(funding_rate) = parse_decimal(funding_val) {
                store.update_funding_rate(coin, funding_rate);
                
                debug!("[Hyperliquid] {} | Funding Rate: {}", coin, funding_rate);
            } else {
                warn!("[Hyperliquid] Could not parse funding rate for {}", coin);
            }
        }
    }
}

// --- ヘルパー関数 ---

/// 板情報の配列から最良価格を取得
/// levels[0] or levels[1] は [{"n": count, "px": "price", "sz": "size"}, ...] 形式
fn get_best_price(level_arr: Option<&Vec<Value>>) -> Option<Decimal> {
    level_arr?
        .first()?
        .as_object()?
        .get("px")
        .and_then(parse_decimal)
}

/// JSONの値をDecimalに変換
fn parse_decimal(v: &Value) -> Option<Decimal> {
    match v {
        Value::String(s) => Decimal::from_str(s).ok(),
        Value::Number(n) => Decimal::from_str(&n.to_string()).ok(),
        _ => None,
    }
}

/// Spot資産のマッピングを取得 (名前 -> WS ID)
async fn fetch_spot_mapping(client: &Client, mapping: &SpotMapping) -> Result<(), Box<dyn std::error::Error>> {
    let body = json!({ "type": "spotMeta" });
    
    let resp = client.post(API_URL)
        .json(&body)
        .send()
        .await?;
    
    let json: Value = resp.json().await?;
    
    if let Some(universe) = json["universe"].as_array() {
        let mut map = mapping.lock().unwrap();
        
        for (i, asset) in universe.iter().enumerate() {
            if let Some(name) = asset["name"].as_str() {
                let ws_id = format!("@{}", i);
                map.insert(name.to_string(), ws_id.clone());
                info!("[Hyperliquid] Mapped SPOT: {} -> {}", name, ws_id);
            }
        }
    }
    
    Ok(())
}