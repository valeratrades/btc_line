use anyhow::{anyhow, Result};
use futures_util::StreamExt;
use reqwest;
use serde_json::Value;
use std::sync::{Arc, Mutex};
use tokio_tungstenite::connect_async;

#[derive(Default, Debug)]
pub struct SpyLine {
	pub spy: Option<f32>,
}
imp SpyLine {
	pub fn display(&self) -> String {
		let spy_display = self.spy.map_or("".to_string(), |v| format!("{:.2}", v));
		format!("{}", spy_display)
	}
}
