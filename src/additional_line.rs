use std::{sync::Arc, time::Duration};

use color_eyre::eyre::Result;
use tokio::time::Interval;
use tracing::instrument;
use v_exchanges::{Exchange, ExchangeResult};
use v_utils::NowThen;

use crate::config::LiveSettings;

//TODO!: implement tiny graphics (now actuaully doable, using snapshot_fonts lib)
#[derive(Debug)]
pub struct AdditionalLine {
	settings: Arc<LiveSettings>,

	open_interest_change: Option<NowThen>,
	btc_volume_change: Option<NowThen>,

	exchange_client: Arc<dyn Exchange>,
	update_interval: Interval,
}

impl AdditionalLine {
	pub fn new(settings: Arc<LiveSettings>, exchange_client: Arc<dyn Exchange>, update_freq: Duration) -> Self {
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
	#[instrument(skip_all)]
	pub async fn collect(&mut self) -> ExchangeResult<bool> {
		//DEPRECATE: nuke the commented-out `dbg`s here, - were useful when figuring out correct form of main loop, that wouldn't drop arms prematurely (this arm is usually the slower one)
		self.update_interval.tick().await;
		//dbg!("after waiting for self.update_interval");
		let (oi_result, vol_result) = tokio::join!(self.get_open_interest_change(), self.get_btc_volume_change());
		//dbg!("got oi and vol: {oi_result:?}, {vol_result:?}");

		let mut changed = false;
		let new_oi = match oi_result {
			Ok(v) => Some(v),
			Err(e) => {
				tracing::warn!("Failed to get open interest: {e}");
				None
			}
		};
		if self.open_interest_change != new_oi {
			self.open_interest_change = new_oi;
			changed = true;
		}

		let new_vol = match vol_result {
			Ok(v) => Some(v),
			Err(e) => {
				tracing::warn!("Failed to get BTC volume: {e}");
				None
			}
		};
		if self.btc_volume_change != new_vol {
			self.btc_volume_change = new_vol;
			changed = true;
		}

		Ok(changed)
	}

	pub fn display(&self) -> String {
		let mut oi_str = self.open_interest_change.as_ref().map_or("None".to_string(), |v| v.to_string());
		let mut v_str = self.btc_volume_change.as_ref().map_or("None".to_string(), |v| v.to_string());

		if self.settings.config().unwrap().label {
			oi_str = format!("OI:{oi_str}");
			v_str = format!("V:{v_str}");
		}
		let s = format!("{oi_str} {v_str}");
		s
	}

	/// Compares two last periods of `comparison_offset_h` hours. Default is yesterday against the day before.
	async fn get_btc_volume_change(&self) -> Result<NowThen> {
		let base_interval = self.settings.config().unwrap().comparison_offset_h * 12;

		let mut klines = self.exchange_client.klines("BTC-USDT.P".into(), "5m".into(), (base_interval * 2).into()).await?;

		let second_period = klines.split_off(base_interval);
		let first_period = klines;

		let avg_second = second_period.iter().map(|k| k.volume_quote).sum::<f64>() / second_period.len() as f64;
		let avg_first = first_period.iter().map(|k| k.volume_quote).sum::<f64>() / first_period.len() as f64;

		Ok(NowThen::new(avg_second, avg_first))
	}

	/// Compares btc OI today against 24h ago (changes based on `comparison_offset_h`)
	async fn get_open_interest_change(&self) -> Result<NowThen> {
		let n_intervals = self.settings.config().unwrap().comparison_offset_h * 12 + 1;

		let oi = self.exchange_client.open_interest("BTC-USDT.P".into(), "5m".into(), n_intervals.into()).await?;

		let then: f64 = oi[0].val_quote.expect("atm we're doing Binance, for which `quote` value will be present");
		let now: f64 = oi[oi.len() - 1].val_quote.expect("atm we're doing Binance, for which `quote` value will be present");

		Ok(NowThen::new(now, then))
	}
}
