use crate::store::Exchange;
use rust_decimal::Decimal;
use rust_decimal::prelude::*;
use std::str::FromStr;

// 定数定義
const SLIPPAGE: &str = "0.0001"; // スリッページ 0.01%

fn slippage() -> Decimal {
    Decimal::from_str(SLIPPAGE).unwrap()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Asset {
    BTC,
    ETH,
    SOL,
    HYPE,
}

impl Asset {
    pub fn as_symbol(&self) -> &'static str {
        match self {
            Asset::BTC => "BTC",
            Asset::ETH => "ETH",
            Asset::SOL => "SOL",
            Asset::HYPE => "HYPE",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstrumentType {
    Spot,
    Perp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Currency {
    JPY,
    USD,
}

#[derive(Debug, Clone)]
pub struct MarketData {
    pub exchange: Exchange,
    pub asset: Asset,
    pub instrument: InstrumentType,
    pub currency: Currency, // 通貨単位 (JPY or USD)
    pub ask: Decimal,       // 買値
    pub bid: Decimal,       // 売値
    pub funding_rate: Decimal, 
}

impl MarketData {
    /// 取引所ごとのTaker手数料を取得 (単位: 小数。例: 0.05% -> 0.0005)
    /// 提示された数値: HL(0.015, 0.045), GMO(-0.01, 0.05), Bitbank(-0.02, 0.12)
    /// ※アビトラは即時約定が必要なためTaker手数料を採用
    pub fn taker_fee(&self) -> Decimal {
        match self.exchange {
            // 0.0450% -> 0.00045
            Exchange::Hyperliquid => Decimal::from_f64(0.00045).unwrap(),
            // 0.05% -> 0.0005
            Exchange::Gmo => Decimal::from_f64(0.0005).unwrap(),
            // 0.12% -> 0.0012
            Exchange::Bitbank => Decimal::from_f64(0.0012).unwrap(),
            // Kraken is used for FX rate only, not for trading
            Exchange::Kraken => Decimal::ZERO,
        }
    }
}

#[derive(Debug)]
pub struct ArbitrageOpportunity {
    pub asset: Asset,
    pub long_exchange: Exchange,
    pub long_instrument: InstrumentType,
    pub long_price_jpy: Decimal,
    pub long_price_raw: Decimal,
    pub long_currency: Currency,
    pub long_fee_jpy: Decimal,
    pub short_exchange: Exchange,
    pub short_instrument: InstrumentType,
    pub short_price_jpy: Decimal,
    pub short_price_raw: Decimal,
    pub short_currency: Currency,
    pub short_fee_jpy: Decimal,
    pub base_profit_jpy: Decimal,
    pub fr_impact_jpy: Decimal,
    pub slippage_cost_jpy: Decimal,
    pub estimated_profit_jpy: Decimal,
    pub estimated_profit_pct: Decimal,
    pub usd_jpy_rate: Decimal,
    pub details: String,
}

/// usd_jpy_rate: Krakenから取得したUSD/JPYレート
pub fn find_best_arbitrage(
    market_data_list: &[MarketData],
    target_asset: Asset,
    usd_jpy_rate: Decimal,
) -> Option<ArbitrageOpportunity> {
    
    // 対象通貨のデータのみ抽出
    let relevant_data: Vec<&MarketData> = market_data_list
        .iter()
        .filter(|d| d.asset == target_asset)
        .collect();

    let mut best_opportunity: Option<ArbitrageOpportunity> = None;
    let mut max_profit_pct = Decimal::ZERO;

    for buy_side in &relevant_data {
        for sell_side in &relevant_data {
            
            // 同一取引所の同一商品はスキップ
            if buy_side.exchange == sell_side.exchange && buy_side.instrument == sell_side.instrument {
                continue;
            }

            // --- 1. Buy Side (Long) コスト計算 (JPY換算) ---
            // Ask価格 * (1 + 手数料 + スリッページ)
            let buy_price_raw = buy_side.ask;
            let buy_fee_multiplier = Decimal::ONE + buy_side.taker_fee() + slippage();
            
            // 通貨変換
            let buy_cost_jpy = match buy_side.currency {
                Currency::JPY => buy_price_raw * buy_fee_multiplier,
                Currency::USD => buy_price_raw * usd_jpy_rate * buy_fee_multiplier,
            };

            // --- 2. Sell Side (Short) 売上計算 (JPY換算) ---
            // Bid価格 * (1 - 手数料 - スリッページ)
            let sell_price_raw = sell_side.bid;
            let sell_fee_multiplier = Decimal::ONE - sell_side.taker_fee() - slippage();

            // 通貨変換
            let sell_revenue_jpy = match sell_side.currency {
                Currency::JPY => sell_price_raw * sell_fee_multiplier,
                Currency::USD => sell_price_raw * usd_jpy_rate * sell_fee_multiplier,
            };

            // --- 3. FRインパクト (JPY換算) ---
            // FRはスポット価格に対する比率として単純加算
            // Buy(Long)なら -FR, Sell(Short)なら +FR (FR>0の場合)
            let mut fr_impact_pct = Decimal::ZERO;
            if let InstrumentType::Perp = buy_side.instrument {
                fr_impact_pct -= buy_side.funding_rate;
            }
            if let InstrumentType::Perp = sell_side.instrument {
                fr_impact_pct += sell_side.funding_rate;
            }

            // 収益計算 (1単位あたり)
            // (売上 - コスト) + (コスト * FR%) 
            // ※FRは保有額(コスト相当)に対してかかると概算
            let base_profit_jpy = sell_revenue_jpy - buy_cost_jpy;
            let fr_profit_jpy = buy_cost_jpy * fr_impact_pct;
            
            let total_profit_jpy = base_profit_jpy + fr_profit_jpy;
            let total_profit_pct = total_profit_jpy / buy_cost_jpy;

            // --- 4. 判定 (プラスかつ最大利益) ---
            if total_profit_pct > Decimal::ZERO && total_profit_pct > max_profit_pct {
                max_profit_pct = total_profit_pct;
                
                // 手数料とスリッページのコスト計算
                let buy_fee_cost = buy_price_raw * buy_side.taker_fee();
                let buy_slippage_cost = buy_price_raw * slippage();
                let sell_fee_cost = sell_price_raw * sell_side.taker_fee();
                let sell_slippage_cost = sell_price_raw * slippage();
                
                let total_buy_fee_jpy = match buy_side.currency {
                    Currency::JPY => buy_fee_cost,
                    Currency::USD => buy_fee_cost * usd_jpy_rate,
                };
                
                let total_sell_fee_jpy = match sell_side.currency {
                    Currency::JPY => sell_fee_cost,
                    Currency::USD => sell_fee_cost * usd_jpy_rate,
                };
                
                let total_slippage_jpy = match buy_side.currency {
                    Currency::JPY => buy_slippage_cost,
                    Currency::USD => buy_slippage_cost * usd_jpy_rate,
                } + match sell_side.currency {
                    Currency::JPY => sell_slippage_cost,
                    Currency::USD => sell_slippage_cost * usd_jpy_rate,
                };

                let details = format!(
                    "Buy {:?} {:?}@{} {} | Sell {:?} {:?}@{} {} | Rate: {} JPY/USD",
                    buy_side.exchange, buy_side.instrument, buy_side.ask, buy_side.currency as u8,
                    sell_side.exchange, sell_side.instrument, sell_side.bid, sell_side.currency as u8,
                    usd_jpy_rate
                );

                best_opportunity = Some(ArbitrageOpportunity {
                    asset: target_asset,
                    long_exchange: buy_side.exchange,
                    long_instrument: buy_side.instrument,
                    long_price_jpy: buy_cost_jpy,
                    long_price_raw: buy_price_raw,
                    long_currency: buy_side.currency,
                    long_fee_jpy: total_buy_fee_jpy,
                    short_exchange: sell_side.exchange,
                    short_instrument: sell_side.instrument,
                    short_price_jpy: sell_revenue_jpy,
                    short_price_raw: sell_price_raw,
                    short_currency: sell_side.currency,
                    short_fee_jpy: total_sell_fee_jpy,
                    base_profit_jpy: base_profit_jpy,
                    fr_impact_jpy: fr_profit_jpy,
                    slippage_cost_jpy: total_slippage_jpy,
                    estimated_profit_jpy: total_profit_jpy,
                    estimated_profit_pct: total_profit_pct * Decimal::from(100),
                    usd_jpy_rate,
                    details,
                });
            }
        }
    }
    best_opportunity
}