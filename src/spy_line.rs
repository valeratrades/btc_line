use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, error, info, warn};

use crate::{config::AppConfig, output::Output};

#[derive(Default, Debug)]
pub struct SpyLine {
	pub spy_price: Option<f64>,
	//TODO!: have another loop that updates spy_price to None if last timestamp is more than 60s old.
	last_message_timestamp: DateTime<Utc>,
}
impl SpyLine {
	pub fn display(&self) -> String {
		self.spy_price.map_or_else(|| "".to_string(), |v| format!("{:.2}", v))
	}

	pub async fn websocket(self_arc: Arc<Mutex<Self>>, config: AppConfig, output: Arc<Mutex<Output>>) {
		let alpaca_key = &config.spy.alpaca_key;
		let alpaca_secret = &config.spy.alpaca_secret;
		loop {
			let handle = spy_websocket_listen(self_arc.clone(), output.clone(), alpaca_key, alpaca_secret);

			handle.await;
			{
				let mut lock = self_arc.lock().unwrap();
				lock.spy_price = None;
			}
			debug!("Restarting Spy Websocket in 30 seconds...");
			tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
		}
	}
}

async fn spy_websocket_listen(self_arc: Arc<Mutex<SpyLine>>, output: Arc<Mutex<Output>>, alpaca_key: &str, alpaca_secret: &str) {
	let ws_result = tokio::time::timeout(tokio::time::Duration::from_secs(30), connect_async("wss://stream.data.alpaca.markets/v2/iex")).await;

	let (ws_stream, _) = match ws_result {
		Ok(Ok(connection)) => connection,
		Ok(Err(e)) => {
			error!("Failed to connect to Alpaca WebSocket: {}", e);
			return;
		}
		Err(_) => {
			error!("Connection to Alpaca WebSocket timed out after 30 seconds");
			return;
		}
	};

	let (mut write, mut read) = ws_stream.split();

	let auth_message = json!({
		"action": "auth",
		"key": alpaca_key.to_owned(),
		"secret": alpaca_secret.to_owned()
	})
	.to_string();

	// Wait for connection message with timeout
	let connection_result = tokio::time::timeout(tokio::time::Duration::from_secs(10), read.next()).await;

	if let Ok(Some(message_result)) = connection_result {
		let message = match message_result {
			Ok(msg) => msg,
			Err(e) => {
				error!("Error receiving connection message: {}", e);
				return;
			}
		};
		info!("Connected Message: {:?}", message);

		let expected_msg = Message::Text("[{\"T\":\"success\",\"msg\":\"connected\"}]".to_string().into());
		if message != expected_msg {
			warn!("Unexpected connection message, got: {:?}, expected: {:?}", message, expected_msg);
			// Continue anyway, server might have changed response format
		}

		if let Err(e) = write.send(Message::Text(auth_message.into())).await {
			error!("Failed to send auth message: {}", e);
			return;
		}
	} else {
		error!("No connection message received from server or timed out");
		return;
	}

	let listen_message = json!({
		"action":"subscribe",
		"trades":["SPY"]
	})
	.to_string();

	// Wait for authentication message with timeout
	let auth_result = tokio::time::timeout(tokio::time::Duration::from_secs(10), read.next()).await;

	if let Ok(Some(message_result)) = auth_result {
		let message = match message_result {
			Ok(msg) => msg,
			Err(e) => {
				error!("Error receiving authentication message: {}", e);
				return;
			}
		};
		info!("Authenticated Message: {:?}", message);

		let expected_msg = Message::Text("[{\"T\":\"success\",\"msg\":\"authenticated\"}]".to_string().into());
		if message != expected_msg {
			warn!("Unexpected authentication message, got: {:?}, expected: {:?}", message, expected_msg);
			// Continue anyway, server might have changed response format
		}

		if let Err(e) = write.send(Message::Text(listen_message.into())).await {
			error!("Failed to send subscription message: {}", e);
			return;
		}
	} else {
		error!("No authentication message received from server or timed out");
		return;
	}

	// Wait for subscription message with timeout
	let subscription_result = tokio::time::timeout(tokio::time::Duration::from_secs(10), read.next()).await;

	if let Ok(Some(message_result)) = subscription_result {
		let message = match message_result {
			Ok(msg) => msg,
			Err(e) => {
				error!("Error receiving subscription message: {}", e);
				return;
			}
		};
		info!("Subscription Message: {:?}", message);

		// Check if this looks like a valid subscription message (more flexible matching)
		if let Message::Text(ref text) = message {
			if text.contains("\"T\":\"subscription\"") && text.contains("\"trades\":[\"SPY\"]") {
				debug!("Subscription confirmed successfully");
			} else {
				warn!("Unexpected subscription message format: {}", text);
				// Continue anyway as the subscription might still work
			}
		} else {
			warn!("Subscription message was not text: {:?}", message);
		}
	} else {
		error!("No subscription message received from server or timed out");
		return;
	}

	let refresh_arc = self_arc.clone();
	let refresh_output = output.clone();
	tokio::spawn(async move {
		loop {
			if refresh_arc.lock().unwrap().last_message_timestamp < Utc::now() - chrono::Duration::seconds(10 * 60) && refresh_arc.lock().unwrap().spy_price.is_some() {
				refresh_arc.lock().unwrap().spy_price = None;
				let output_copy = {
					let mut output_lock = refresh_output.lock().unwrap();
					output_lock.spy_line_str = "".to_string();
					output_lock.clone()
				};
				if let Err(e) = output_copy.out().await {
					error!("Failed to update output in refresh task: {}", e);
				}
			}
			tokio::time::sleep(tokio::time::Duration::from_secs(5 * 60)).await;
		}
	});

	while let Some(message_result) = read.next().await {
		let message = match message_result {
			Ok(msg) => msg,
			Err(e) => {
				error!("WebSocket error: {}", e);
				// Handle specific WebSocket errors gracefully
				match e {
					tokio_tungstenite::tungstenite::Error::Protocol(ref protocol_err) => {
						error!("WebSocket protocol error: {}", protocol_err);
						break; // Exit loop to trigger reconnection
					}
					tokio_tungstenite::tungstenite::Error::ConnectionClosed => {
						warn!("WebSocket connection closed by server");
						break; // Exit loop to trigger reconnection
					}
					tokio_tungstenite::tungstenite::Error::Io(ref io_err) => {
						error!("WebSocket I/O error: {}", io_err);
						break; // Exit loop to trigger reconnection
					}
					_ => {
						warn!("Other WebSocket error, continuing: {}", e);
						continue; // Try to continue for other errors
					}
				}
			}
		};
		match message {
			Message::Ping(ref data) if data.is_empty() => {
				//erpintln!("SPY ping");
			}
			Message::Text(ref contents) => match serde_json::from_str::<Vec<AlpacaTrade>>(contents) {
				Ok(alpaca_trades) => {
					let alpaca_trade = &alpaca_trades[0];
					if alpaca_trade.symbol == "SPY" {
						let spy_str = {
							let mut lock = self_arc.lock().unwrap();
							lock.spy_price = Some(alpaca_trade.trade_price);
							lock.last_message_timestamp = Utc::now();
							lock.display()
						};
						let output_copy = {
							let mut output_lock = output.lock().unwrap();
							output_lock.spy_line_str = spy_str;
							output_lock.clone()
						};
						if let Err(e) = output_copy.out().await {
							error!("Failed to update output: {}", e);
						}
					}
				}
				Err(e) => {
					debug!("Text but not a quote: {:?}", e);
				}
			},
			_ => {
				debug!("Message from alpaca, that is not text or ping: {:?}", message);
			}
		}
	}
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AlpacaTrade {
	#[serde(rename = "T")]
	pub message_type: String, // Always "t" for trade endpoint
	#[serde(rename = "S")]
	pub symbol: String,
	#[serde(rename = "i")]
	pub trade_id: i64,
	#[serde(rename = "x")]
	pub exchange_code: String,
	#[serde(rename = "p")]
	pub trade_price: f64,
	#[serde(rename = "s")]
	pub trade_size: f64,
	#[serde(rename = "c")]
	pub trade_condition: Vec<String>, // Assuming "array" is a vector of strings
	#[serde(rename = "t")]
	pub timestamp: String, // iso format, could parse to chrono immediately, but don't see a point
}
