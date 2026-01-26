mod collector;
mod store;
mod strategy;

use crate::store::{Exchange, MarketStore};
use crate::strategy::{find_best_arbitrage, Asset, Currency, InstrumentType, MarketData};
use log::{info, warn};
use rust_decimal::Decimal;
use rust_decimal::prelude::*;
use std::time::Duration;
use tokio::time::sleep;

// 取引対象の定義
const TARGET_ASSETS: &[Asset] = &[Asset::BTC, Asset::ETH, Asset::SOL, Asset::HYPE];

fn symbol_for(exchange: Exchange, asset: &Asset, instrument: InstrumentType) -> String {
    match (exchange, instrument) {
        (Exchange::Hyperliquid, InstrumentType::Spot) => format!("{}_SPOT", asset.as_symbol()),
        _ => asset.as_symbol().to_string(),
    }
}

#[tokio::main]
async fn main() {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    info!("Initializing Arbitrage Bot System...");

    let store = MarketStore::new();

    // 各Collectorの起動
    let s_hl = store.clone();
    tokio::spawn(async move {
        let symbols = vec!["BTC", "ETH", "SOL", "HYPE"].into_iter().map(String::from).collect();
        collector::hyperliquid::start_collection(symbols, s_hl).await;
    });

    let s_bb = store.clone();
    tokio::spawn(async move {
        collector::bitbank::start_collection(s_bb).await;
    });

    let s_gmo = store.clone();
    tokio::spawn(async move {
        let symbols = vec!["BTC", "ETH", "SOL", "HYPE"].into_iter().map(String::from).collect();
        collector::gmo::start_collection(symbols, s_gmo).await;
    });
    
    let s_kraken = store.clone();
    tokio::spawn(async move {
        collector::kraken::start_collection(s_kraken).await;
    });

    info!("Waiting for market data warmup (5s)...");
    sleep(Duration::from_secs(5)).await;

    loop {
        // 為替レートの取得 (USD_JPY)
        let usd_jpy = match store.get_fx_rate("USD_JPY") {
            Some(rate) => rate,
            None => {
                warn!("FX rate (USD_JPY) not available yet. Skipping cycle.");
                sleep(Duration::from_millis(1000)).await;
                continue;
            }
        };

        for asset in TARGET_ASSETS {
            let mut market_data_list = Vec::new();

            // Hyperliquid (Perp)
            if let Some(data) = store.get_market_data(Exchange::Hyperliquid, &symbol_for(Exchange::Hyperliquid, asset, InstrumentType::Perp)) {
                market_data_list.push(MarketData {
                    exchange: Exchange::Hyperliquid,
                    asset: *asset,
                    instrument: InstrumentType::Perp,
                    currency: Currency::USD,
                    ask: data.ask,
                    bid: data.bid,
                    funding_rate: data.funding_rate,
                });
            }
            
            // Hyperliquid (Spot)
            if let Some(data) = store.get_market_data(Exchange::Hyperliquid, &symbol_for(Exchange::Hyperliquid, asset, InstrumentType::Spot)) {
                market_data_list.push(MarketData {
                    exchange: Exchange::Hyperliquid,
                    asset: *asset,
                    instrument: InstrumentType::Spot,
                    currency: Currency::USD,
                    ask: data.ask,
                    bid: data.bid,
                    funding_rate: Decimal::ZERO,
                });
            }

            // Bitbank (JPY, Spot)
            if let Some(data) = store.get_market_data(Exchange::Bitbank, &symbol_for(Exchange::Bitbank, asset, InstrumentType::Spot)) {
                market_data_list.push(MarketData {
                    exchange: Exchange::Bitbank,
                    asset: *asset,
                    instrument: InstrumentType::Spot,
                    currency: Currency::JPY,
                    ask: data.ask,
                    bid: data.bid,
                    funding_rate: Decimal::ZERO,
                });
            }

            // GMO (JPY, Spot)
            if let Some(data) = store.get_market_data(Exchange::Gmo, &symbol_for(Exchange::Gmo, asset, InstrumentType::Spot)) {
                market_data_list.push(MarketData {
                    exchange: Exchange::Gmo,
                    asset: *asset,
                    instrument: InstrumentType::Spot,
                    currency: Currency::JPY,
                    ask: data.ask,
                    bid: data.bid,
                    funding_rate: Decimal::ZERO,
                });
            }

            // 戦略実行
            if market_data_list.len() >= 2 {
                if let Some(opp) = find_best_arbitrage(&market_data_list, asset.clone(), usd_jpy) {
                    if opp.estimated_profit_pct > Decimal::from_f64(0.05).unwrap() { // 0.05%
                        info!("================================================================================");
                        info!("🚀 [裁定取引機会] {:?}", asset);
                        info!("================================================================================");
                        info!("📊 取引詳細:");
                        info!("  買い: {:?} {:?} @ {} {:?}", opp.long_exchange, opp.long_instrument, opp.long_price_raw, opp.long_currency);
                        info!("  売り: {:?} {:?} @ {} {:?}", opp.short_exchange, opp.short_instrument, opp.short_price_raw, opp.short_currency);
                        info!("  為替レート: {} JPY/USD", opp.usd_jpy_rate);
                        info!("");
                        info!("💰 損益計算 (1単位あたり):");
                        info!("  買値(JPY換算): ¥{:.2}", opp.long_price_jpy);
                        info!("  売値(JPY換算): ¥{:.2}", opp.short_price_jpy);
                        info!("  粗利益: ¥{:.2}", opp.base_profit_jpy);
                        info!("");
                        info!("📉 コスト:");
                        info!("  買い手数料: ¥{:.2}", opp.long_fee_jpy);
                        info!("  売り手数料: ¥{:.2}", opp.short_fee_jpy);
                        info!("  スリッページ: ¥{:.2}", opp.slippage_cost_jpy);
                        info!("  FR影響: ¥{:.2}", opp.fr_impact_jpy);
                        info!("  合計コスト: ¥{:.2}", opp.long_fee_jpy + opp.short_fee_jpy + opp.slippage_cost_jpy);
                        info!("");
                        info!("✅ 純利益: ¥{:.2} ({:.4}%)", opp.estimated_profit_jpy, opp.estimated_profit_pct);
                        info!("================================================================================");
                        // TODO: ここで executor::execute(&opportunity).await;
                    }
                }
            }
        }
        sleep(Duration::from_millis(500)).await;
    }
}