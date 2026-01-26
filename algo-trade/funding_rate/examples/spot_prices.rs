use futures_util::{SinkExt, StreamExt};
use log::info;
use rust_decimal::Decimal;
use serde_json::{json, Value};
use std::str::FromStr;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

const WS_URL: &str = "wss://api.hyperliquid.xyz/ws";
const API_URL: &str = "https://api.hyperliquid.xyz/info";

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    info!("🔍 Spot Asset Explorer");
    info!("=====================\n");

    let client = reqwest::Client::new();
    
    // spotMetaからSpot資産リストを取得
    let spot_names = match fetch_spot_asset_names(&client).await {
        Ok(names) => names,
        Err(e) => {
            eprintln!("❌ Failed to fetch spot asset names: {}", e);
            return;
        }
    };

    info!("📊 Total Spot Assets: {}\n", spot_names.len());
    
    // コマンドラインで範囲指定可能（デフォルト: 最初の100個の一覧表示）
    let args: Vec<String> = std::env::args().collect();
    
    if args.len() >= 3 {
        // 価格取得モード
        let idx1: usize = args[1].parse().unwrap_or(0);
        let idx2: usize = args[2].parse().unwrap_or(1);
        
        if idx1 >= spot_names.len() || idx2 >= spot_names.len() {
            eprintln!("❌ Index out of range. Valid range: 0-{}", spot_names.len() - 1);
            return;
        }
        
        let name1 = &spot_names[idx1].1;
        let name2 = &spot_names[idx2].1;
        
        info!("📡 Fetching prices for:");
        info!("  [{}] {}", idx1, name1);
        info!("  [{}] {}\n", idx2, name2);
        
        if let Err(e) = fetch_two_spot_prices(idx1, idx2).await {
            eprintln!("❌ Error: {}", e);
        }
    } else {
        // リスト表示モード
        let start = if args.len() > 1 {
            args[1].parse::<usize>().unwrap_or(0)
        } else {
            0
        };
        let end = (start + 100).min(spot_names.len());
        
        info!("📋 Spot Asset List [{}..{}]", start, end - 1);
        info!("================================================\n");
        
        for (i, name) in &spot_names[start..end] {
            println!("[{:3}] {}", i, name);
        }
        
        info!("\n💡 Usage to get prices:");
        info!("   cargo run --example spot_prices -- <idx1> <idx2>");
        info!("   Example: cargo run --example spot_prices -- 0 1");
        info!("\n   Show other assets:");
        info!("   cargo run --example spot_prices -- --list 100");
    }
}

/// spotMeta APIからSpot資産名を取得
async fn fetch_spot_asset_names(client: &reqwest::Client) -> Result<Vec<(usize, String)>, Box<dyn std::error::Error>> {
    let body = json!({ "type": "spotMeta" });
    
    let resp = client.post(API_URL)
        .json(&body)
        .send()
        .await?;
    
    let json: Value = resp.json().await?;
    
    let mut names = Vec::new();
    if let Some(universe) = json["universe"].as_array() {
        for (i, asset) in universe.iter().enumerate() {
            if let Some(name) = asset["name"].as_str() {
                names.push((i, name.to_string()));
            }
        }
    }
    
    Ok(names)
}

/// 2つのSpot資産の価格をWebSocketで取得
async fn fetch_two_spot_prices(idx1: usize, idx2: usize) -> Result<(), Box<dyn std::error::Error>> {
    let (ws_stream, _) = connect_async(WS_URL).await?;
    let (mut write, mut read) = ws_stream.split();

    // 2つのSpot資産をサブスクリプション
    let sub1 = json!({
        "method": "subscribe",
        "subscription": { "type": "l2Book", "coin": format!("@{}", idx1) }
    });
    let sub2 = json!({
        "method": "subscribe",
        "subscription": { "type": "l2Book", "coin": format!("@{}", idx2) }
    });
    
    write.send(Message::Text(sub1.to_string())).await?;
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    write.send(Message::Text(sub2.to_string())).await?;
    
    info!("📡 Subscribed. Waiting for prices...\n");

    let mut collected = 0;
    let start_time = std::time::Instant::now();
    
    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                let v: Value = serde_json::from_str(&text).unwrap_or(json!({}));

                if v["channel"] == "l2Book" {
                    let data = &v["data"];
                    let coin_raw = data["coin"].as_str().unwrap_or("");
                    
                    if let Some((bid, ask, mid)) = parse_price(data) {
                        println!("Coin: {} | Bid: {}, Ask: {}, Mid: {}", coin_raw, bid, ask, mid);
                        collected += 1;
                    }
                }
            }
            Err(e) => {
                eprintln!("❌ WebSocket error: {}", e);
                break;
            }
            _ => {}
        }

        // 両方のSpot資産データを複数回取得
        if collected >= 10 || start_time.elapsed() > std::time::Duration::from_secs(10) {
            break;
        }
    }

    info!("\n✅ Price collection completed.");
    
    Ok(())
}

/// 板情報から最良価格を抽出
fn parse_price(data: &Value) -> Option<(Decimal, Decimal, Decimal)> {
    let levels = &data["levels"];
    if let (Some(bids), Some(asks)) = (levels[0].as_array(), levels[1].as_array()) {
        if bids.is_empty() || asks.is_empty() {
            return None;
        }
        
        let bid = bids[0]["px"].as_str().and_then(|s| Decimal::from_str(s).ok())?;
        let ask = asks[0]["px"].as_str().and_then(|s| Decimal::from_str(s).ok())?;
        let mid = (bid + ask) / Decimal::from(2);
        
        return Some((bid, ask, mid));
    }
    None
}
