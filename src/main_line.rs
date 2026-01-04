use std::{pin::pin, sync::Arc, time::Duration};

use futures::future::{Either, select};
use tokio::time::Interval;
use v_exchanges::{Binance, Exchange as _, ExchangeResult, ExchangeStream, Instrument, Trade, adapters::generics::ws::WsError};
use v_utils::{Percent, trades::Pair};

use crate::config::LiveSettings;

#[derive(Debug)]
pub struct MainLine {
	settings: Arc<LiveSettings>,

	btcusdt_price: Option<f64>,
	percent_longs: Option<Percent>,

	lsr_interval: Interval,

	ws_connection: Option<Box<dyn ExchangeStream<Item = Trade>>>,
	binance_agent: Arc<Binance>,
	reconnect_attempt: u32,
}
impl MainLine {
	pub fn try_new(settings: Arc<LiveSettings>, bn: Arc<Binance>, lsr_update_freq: Duration) -> ExchangeResult<Self> {
		let ws_connection = Self::create_ws_connection(&bn)?;
		let lsr_interval = tokio::time::interval(lsr_update_freq);

		Ok(Self {
			settings,
			ws_connection: Some(ws_connection),
			binance_agent: bn,
			lsr_interval,
			reconnect_attempt: 0,
			// defaults {{{
			btcusdt_price: None,
			percent_longs: None,
			//,}}}
		})
	}

	fn create_ws_connection(bn: &Binance) -> ExchangeResult<Box<dyn ExchangeStream<Item = Trade>>> {
		let pairs: Vec<Pair> = vec![("BTC", "USDT").into()];
		let instrument = Instrument::Perp;
		bn.ws_trades(pairs, instrument)
	}

	fn reconnect_delay(attempt: u32) -> Duration {
		let delay_secs = std::f64::consts::E.powi(attempt as i32).min(60.0);
		Duration::from_secs_f64(delay_secs)
	}

	async fn ensure_ws_connection(&mut self) {
		if self.ws_connection.is_some() {
			return;
		}

		loop {
			let delay = Self::reconnect_delay(self.reconnect_attempt);
			v_utils::log!("WebSocket reconnect attempt {} in {:.1}s", self.reconnect_attempt + 1, delay.as_secs_f64());
			tokio::time::sleep(delay).await;

			match Self::create_ws_connection(&self.binance_agent) {
				Ok(ws) => {
					v_utils::log!("WebSocket reconnected successfully");
					self.ws_connection = Some(ws);
					self.reconnect_attempt = 0;
					return;
				}
				Err(e) => {
					v_utils::log!("WebSocket reconnect failed: {e}");
					self.reconnect_attempt += 1;
				}
			}
		}
	}

	/// # Returns
	/// if any of the data has been updated, returns `true`
	pub async fn collect(&mut self) -> ExchangeResult<bool> {
		self.ensure_ws_connection().await;

		enum Event {
			Tick,
			Trade(Option<Result<Trade, WsError>>),
		}

		let event = {
			let tick_fut = pin!(self.lsr_interval.tick());
			let trade_fut = pin!(self.ws_connection.as_mut().unwrap().next());

			match select(tick_fut, trade_fut).await {
				Either::Left((_tick, _trade_fut)) => Event::Tick,
				Either::Right((trade_result, _tick_fut)) => Event::Trade(Some(trade_result)),
			}
		}; // futures dropped here, borrows released

		let changed = match event {
			Event::Tick => self.handle_lsr().await,
			Event::Trade(trade_result) => self.handle_trade(trade_result),
		};

		Ok(changed)
	}

	async fn handle_lsr(&mut self) -> bool {
		let lsr_result = self
			.binance_agent
			.lsr(("BTC", "USDT").into(), "5m".into(), 1.into(), v_exchanges::binance::data::LsrWho::Global)
			.await;

		match lsr_result {
			Ok(percent_longs) => {
				let new_value = *percent_longs[0];
				if self.percent_longs != Some(new_value) {
					self.percent_longs = Some(new_value);
					true
				} else {
					false
				}
			}
			Err(e) => {
				tracing::warn!("Failed to get LSR: {e}");
				false
			}
		}
	}

	fn handle_trade(&mut self, trade_result: Option<Result<Trade, WsError>>) -> bool {
		match trade_result {
			Some(Ok(trade)) =>
				if self.btcusdt_price != Some(trade.price) {
					self.btcusdt_price = Some(trade.price);
					true
				} else {
					false
				},
			Some(Err(e)) => {
				tracing::warn!("WebSocket error: {e}, will reconnect");
				self.ws_connection = None;
				false
			}
			None => {
				tracing::warn!("WebSocket stream ended, will reconnect");
				self.ws_connection = None;
				false
			}
		}
	}

	pub fn display(&self) -> String {
		let price = self.btcusdt_price.map_or("None".to_string(), |v| format!("{v:.0}"));
		let mut lsr = self.percent_longs.map_or("".to_string(), |v| format!("{:.2}", *v));

		if self.settings.config().unwrap().label {
			lsr = format!("L/S:{lsr}");
		}

		let s = format!("{price}|{lsr}");
		s
	}
}
