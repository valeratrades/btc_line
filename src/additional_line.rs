use std::{rc::Rc, sync::Arc, time::Duration};

use color_eyre::eyre::Result;
use tokio::time::Interval;
use v_exchanges::{Exchange, ExchangeResult};
use v_utils::NowThen;

use crate::config::Settings;

//TODO!: implement tiny graphics
#[derive(Debug)]
pub struct AdditionalLine {
	settings: Rc<Settings>,

	open_interest_change: Option<NowThen>,
	btc_volume_change: Option<NowThen>,

	exchange_client: Arc<Box<dyn Exchange>>,
	update_interval: Interval,
}

impl AdditionalLine {
	pub fn new(settings: Rc<Settings>, exchange_client: Arc<Box<dyn Exchange>>, update_freq: Duration) -> Self {
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

	/// # Returns
	/// if any of the data has been updated, returns `true`
	pub async fn collect(&mut self) -> ExchangeResult<bool> {
		self.update_interval.tick().await;

		let mut changed = false;

		let oi_result = self.get_open_interest_change().await;
		match oi_result {
			Ok(open_interest_change) =>
				if self.open_interest_change.is_none_or(|v| v != open_interest_change) {
					self.open_interest_change = Some(open_interest_change);
					changed = true;
				},
			Err(e) => {
				tracing::warn!("Failed to get open interest: {e}");
			}
		};

		let volume_result = self.get_btc_volume_change().await;
		match volume_result {
			Ok(btc_volume_change) =>
				if self.btc_volume_change.is_none_or(|v| v != btc_volume_change) {
					self.btc_volume_change = Some(btc_volume_change);
					changed = true;
				},
			Err(e) => {
				tracing::warn!("Failed to get BTC volume: {e}");
			}
		};

		Ok(changed)
	}

	pub fn display(&self) -> Result<String> {
		let mut oi_str = self.open_interest_change.as_ref().map_or("None".to_string(), |v| v.to_string());
		let mut v_str = self.btc_volume_change.as_ref().map_or("None".to_string(), |v| v.to_string());

		if self.settings.config()?.label {
			oi_str = format!("OI:{oi_str}");
			v_str = format!("V:{v_str}");
		}
		let s = format!("{oi_str} {v_str}");
		Ok(s)
	}

	/// Compares two last periods of `comparison_offset_h` hours. Default is yesterday against the day before.
	async fn get_btc_volume_change(&self) -> Result<NowThen> {
		let base_interval = self.settings.config()?.comparison_offset_h * 12;

		let mut klines = self.exchange_client.klines("BTC-USDT.P".into(), "5m".into(), (base_interval * 2).into()).await?;

		let second_period = klines.split_off(base_interval);
		let first_period = klines;

		let avg_second = second_period.iter().map(|k| k.volume_quote).sum::<f64>() / second_period.len() as f64;
		let avg_first = first_period.iter().map(|k| k.volume_quote).sum::<f64>() / first_period.len() as f64;

		Ok(NowThen::new(avg_second, avg_first))
	}

	/// Compares btc OI today against 24h ago (changes based on `comparison_offset_h`)
	async fn get_open_interest_change(&self) -> Result<NowThen> {
		let n_intervals = self.settings.config()?.comparison_offset_h * 12 + 1;

		let oi = self.exchange_client.open_interest("BTC-USDT.P".into(), "5m".into(), n_intervals.into()).await?;

		let then: f64 = oi[0].val_quote.expect("atm we're doing Binance, for which `quote` value will be present");
		let now: f64 = oi[oi.len() - 1].val_quote.expect("atm we're doing Binance, for which `quote` value will be present");

		Ok(NowThen::new(now, then))
	}
}
