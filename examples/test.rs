use std::{
	rc::Rc,
	sync::{Arc, Mutex},
	time::Duration,
};

use btc_line::config::{AppConfig, Settings};
use color_eyre::eyre::{Result, bail};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::time::Interval;
use tracing::{debug, info};
use v_exchanges::{Exchange, ExchangeName, ExchangeResult};
use v_utils::NowThen;

//TODO!: implement tiny graphics
#[derive(Debug)]
pub struct AdditionalLine {
	settings: Rc<Settings>,

	open_interest_change: Option<NowThen>,
	btc_volume_change: Option<NowThen>,

	exchange_client: Box<dyn Exchange>,
	update_interval: Interval,
}

impl AdditionalLine {
	pub fn new(settings: Rc<Settings>, update_freq: Duration, exchange_client: Box<dyn Exchange>) -> Self {
		let update_interval = tokio::time::interval(update_freq);

		Self {
			settings,
			update_interval,
			exchange_client,
			// defaults {{{
			open_interest_change: None,
			btc_volume_change: None,
			//,}}}
		}
	}

	//pub async fn collect(self_arc: Arc<Mutex<Self>>, config: &AppConfig) {
	//	let comparison_offset_h = config.comparison_offset_h;
	//
	//	let client = reqwest::Client::new();
	//	let open_interest_change_handler = get_open_interest_change(&client, "BTCUSDT", comparison_offset_h);
	//	let btc_volume_change_handler = get_btc_volume_change(&client, comparison_offset_h);
	//
	//	let mut new_state = AdditionalLine::default(); // slight perf hit in favor of debuggability
	//	let (oi_result, volume_result) = tokio::join!(open_interest_change_handler, btc_volume_change_handler);
	//
	//	match oi_result {
	//		Ok(open_interest_change) => new_state.open_interest_change = Some(open_interest_change),
	//		Err(e) => {
	//			debug!("Failed to get Open Interest: {e}");
	//		}
	//	};
	//	match volume_result {
	//		Ok(btc_volume_change) => new_state.btc_volume_change = Some(btc_volume_change),
	//		Err(e) => {
	//			debug!("Failed to get BTC Volume: {e}");
	//		}
	//	};
	//
	//	info!(?new_state);
	//	*self_arc.lock().unwrap() = new_state;
	//}

	pub fn display(&self, config: &AppConfig) -> String {
		let mut oi_str = self.open_interest_change.as_ref().map_or("None".to_string(), |v| v.to_string());
		let mut v_str = self.btc_volume_change.as_ref().map_or("None".to_string(), |v| v.to_string());

		if config.label {
			oi_str = format!("OI:{oi_str}");
			v_str = format!("V:{v_str}");
		}
		format!("{oi_str} {v_str}")
	}

	/// Compares two last periods of `comparison_offset_h` hours. Default is yesterday against the day before.
	async fn get_btc_volume_change(&self, client: &reqwest::Client, comparison_offset_h: usize) -> Result<NowThen> {
		let interval = comparison_offset_h * 12;

		let klines = self.exchange_client.klines("BTC-USDT.P".try_into()?, "5m".into(), (comparison_offset_h * 12).into()).await?;
		let now: f64 = klines.back().unwrap().volume_quote;
		let then: f64 = klines.front().unwrap().volume_quote;

		dbg!(&klines.back().unwrap(), &klines.front().unwrap()); //TODO: figure out which one is older

		Ok(NowThen::new(now, then))
	}
}

/// Compares btc OI today against 24h ago (changes based on `comparison_offset_h`)
async fn get_open_interest_change(client: &reqwest::Client, symbol: &str, comparison_offset_h: usize) -> Result<NowThen> {
	let url = format!(
		"https://fapi.binance.com/futures/data/openInterestHist?symbol={}&period=5m&limit={}",
		symbol,
		comparison_offset_h * 12 + 1
	);

	let response = client.get(&url).send().await?;
	if response.status().is_success() {
		let json_string = response.text().await?;
		let r: Vec<OpenInterestHist> = serde_json::from_str(&json_string)?;

		let now: f64 = r[0].sumOpenInterestValue.parse()?;
		let then: f64 = r[r.len() - 1].sumOpenInterestValue.parse()?;

		Ok(NowThen::new(now, then))
	} else {
		bail!("Failed to get Open Interest Change: {}", response.status())
	}
}

///// Compares two last periods of `comparison_offset_h` hours. Default is yesterday against the day before.
//async fn get_btc_volume_change(client: &reqwest::Client, comparison_offset_h: usize) -> Result<NowThen> {
//	let interval = comparison_offset_h * 12;
//	let url = format!("https://fapi.binance.com/fapi/v1/klines?symbol=BTCUSDT&interval=5m&limit={}", interval + interval);
//
//	let response = client.get(&url).send().await?;
//	if response.status().is_success() {
//		let json_string = response.text().await?;
//		let r: Vec<Kline> = serde_json::from_str(&json_string)?;
//
//		let split = r.split_at(interval);
//		let now: f64 = split.0.iter().map(|v| v.quote_asset_volume.parse::<f64>().unwrap()).sum();
//		let then: f64 = split.1.iter().map(|v| v.quote_asset_volume.parse::<f64>().unwrap()).sum();
//
//		Ok(NowThen::new(now, then))
//	} else {
//		bail!("Failed to get BTC Volume: {}", response.status())
//	}
//}

#[derive(Deserialize, Debug)]
#[allow(non_snake_case, dead_code)]
struct OpenInterestHist {
	symbol: String,
	sumOpenInterest: String,
	sumOpenInterestValue: String,
	timestamp: i64,
}

#[derive(Deserialize, Debug)]
#[allow(non_snake_case, dead_code)]
struct Kline {
	open_time: i64,
	open: String,
	high: String,
	low: String,
	close: String,
	volume: String,
	close_time: i64,
	quote_asset_volume: String,
	number_of_trades: usize,
	taker_buy_base_asset_volume: String,
	taker_buy_quote_asset_volume: String,
	ignore: String,
}

#[tokio::main]
async fn main() {
	let mut exch = ExchangeName::Binance.init_client();
	exch.set_max_tries(3);

	let interest_change = get_open_interest_change(&client, "BTCUSDT", 24).await;
	let volume_change = get_btc(&client, 24).await;

	dbg!(&interest_change, &volume_change);
}
