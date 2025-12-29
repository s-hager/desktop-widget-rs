use yahoo_finance_api as yahoo;
use chrono::{DateTime, Utc};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct StockData {
    pub symbol: String,
    pub price: f64,
    pub change_percent: f64,
    pub history: Vec<(DateTime<Utc>, f64, f64, f64, f64)>, // timestamp, open, high, low, close
}

pub struct StockClient;

impl StockClient {
    pub async fn fetch_quote(symbol: &str) -> Option<StockData> {
        let provider = yahoo::YahooConnector::new().ok()?;
        
        // Fetch quote for current price
        let quote_resp = provider.get_latest_quotes(symbol, "1d").await.ok()?;
        let quote = quote_resp.last_quote().ok()?;
        
        // Fetch history (1 month)
        // Range: 1mo, Interval: 1d
        let history_resp = provider.get_quote_range(symbol, "1d", "1mo").await.ok()?;
        let quotes = history_resp.quotes().ok()?;

        let history = quotes.iter().map(|q| {
            let time = DateTime::<Utc>::from_target_s(q.timestamp as i64); // timestamp is u64
            (time, q.open, q.high, q.low, q.close)
        }).collect();

        Some(StockData {
            symbol: symbol.to_string(),
            price: quote.close,
            change_percent: (quote.close - quote.open) / quote.open * 100.0,
            history,
        })
    }
}

trait DateTimeFromTimestamp {
    fn from_target_s(timestamp: i64) -> DateTime<Utc>;
}

impl DateTimeFromTimestamp for DateTime<Utc> {
    fn from_target_s(timestamp: i64) -> DateTime<Utc> {
        DateTime::from_timestamp(timestamp, 0).unwrap_or_default()
    }
}
