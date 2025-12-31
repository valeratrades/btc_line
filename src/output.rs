use std::{
	collections::HashMap,
	sync::{Arc, Mutex},
	time::Instant,
};

use color_eyre::eyre::{self, Result};
use tracing::instrument;
use v_utils::define_str_enum;

use crate::config::LiveSettings;

define_str_enum! {
	#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
	pub enum LineName {
		Additional => "additional",
		Main => "main",
		Spy => "spy",
	}
}

#[derive(Debug, Default)]
struct EwwRateLimitState {
	last_sent: Option<Instant>,
	pending_value: Option<String>,
	flush_scheduled: bool,
}

#[derive(Clone, Debug)]
pub struct Output {
	settings: Arc<LiveSettings>,
	old_vals: HashMap<LineName, String>,
	eww_rate_limit_states: Arc<Mutex<HashMap<LineName, EwwRateLimitState>>>,
}
impl Output {
	pub fn new(settings: Arc<LiveSettings>) -> Self {
		Self {
			settings,
			old_vals: HashMap::new(),
			eww_rate_limit_states: Arc::new(Mutex::new(HashMap::new())),
		}
	}

	#[instrument(skip_all, fields(?name, new_value))]
	pub async fn output(&mut self, name: LineName, new_value: String) -> Result<()> {
		if self.old_vals.get(&name).map(|v| v == &new_value).unwrap_or(false) {
			return Ok(());
		}
		self.old_vals.insert(name, new_value.clone());

		let eww_update_handler = self.handle_eww_update(name, new_value.clone());

		let new_value_for_file = new_value.clone();
		let settings = self.settings.clone();
		let file_update_handler = async move {
			let file_path = v_utils::xdg_state_file!(name.to_string());

			if settings.config().outputs.pipes {
				tokio::fs::write(&file_path, format!("{new_value_for_file}\n")).await.map_err(|e| eyre::eyre!(e))?;

				// Update timestamp file
				let timestamp_path = v_utils::xdg_state_file!(".timestamps");
				let timestamp = jiff::Timestamp::now();
				let timestamp_iso = timestamp.to_string();
				let name_str = name.to_string();
				let line_to_write = format!("{name_str}: {timestamp_iso}");

				if timestamp_path.exists() {
					let content = tokio::fs::read_to_string(&timestamp_path).await.map_err(|e| eyre::eyre!(e))?;
					let mut lines: Vec<String> = content.lines().map(String::from).collect();

					let mut found = false;
					for line in &mut lines {
						if let Some((line_name, _)) = line.split_once(": ")
							&& line_name == name_str
						{
							*line = line_to_write.clone();
							found = true;
							break;
						}
					}

					if !found {
						lines.push(line_to_write);
					}

					tokio::fs::write(&timestamp_path, lines.join("\n") + "\n").await.map_err(|e| eyre::eyre!(e))?;
				} else {
					tokio::fs::write(&timestamp_path, format!("{line_to_write}\n")).await.map_err(|e| eyre::eyre!(e))?;
				}
			}

			Ok::<_, eyre::Report>(())
		};

		tokio::try_join!(eww_update_handler, file_update_handler)?;
		Ok(())
	}

	async fn handle_eww_update(&self, name: LineName, new_value: String) -> Result<()> {
		let config = self.settings.config();
		if !config.outputs.eww {
			return Ok(());
		}

		match config.outputs.eww_rate_limit {
			None => {
				// No rate limiting, send immediately
				Self::send_eww_update(name, &new_value).await?;
			}
			Some(rate_limit) => {
				let duration = rate_limit.duration();
				let now = Instant::now();

				let should_send_now;
				let should_schedule_flush;
				{
					let mut states = self.eww_rate_limit_states.lock().unwrap();
					let state = states.entry(name).or_default();

					let can_send = state.last_sent.map(|last| now.duration_since(last) >= duration).unwrap_or(true);

					if can_send {
						state.last_sent = Some(now);
						state.pending_value = None;
						should_send_now = true;
						should_schedule_flush = false;
					} else {
						state.pending_value = Some(new_value.clone());
						should_schedule_flush = !state.flush_scheduled;
						if should_schedule_flush {
							state.flush_scheduled = true;
						}
						should_send_now = false;
					}
				}

				if should_send_now {
					Self::send_eww_update(name, &new_value).await?;
				} else if should_schedule_flush {
					// Schedule a flush task
					let states = self.eww_rate_limit_states.clone();
					let duration = duration;
					tokio::spawn(async move {
						tokio::time::sleep(duration).await;
						Self::flush_pending_eww_update(name, states, duration).await;
					});
				}
			}
		}

		Ok(())
	}

	async fn flush_pending_eww_update(name: LineName, states: Arc<Mutex<HashMap<LineName, EwwRateLimitState>>>, rate_limit_duration: std::time::Duration) {
		loop {
			let now = Instant::now();
			let (value_to_send, should_wait) = {
				let mut states = states.lock().unwrap();
				let state = states.entry(name).or_default();

				let can_send = state.last_sent.map(|last| now.duration_since(last) >= rate_limit_duration).unwrap_or(true);

				if can_send {
					if let Some(value) = state.pending_value.take() {
						state.last_sent = Some(now);
						state.flush_scheduled = false;
						(Some(value), false)
					} else {
						state.flush_scheduled = false;
						(None, false)
					}
				} else {
					// Need to wait more
					(None, true)
				}
			};

			if let Some(value) = value_to_send {
				if let Err(e) = Self::send_eww_update(name, &value).await {
					tracing::error!("Failed to send eww update: {e}");
				}
				break;
			} else if should_wait {
				tokio::time::sleep(rate_limit_duration).await;
			} else {
				break;
			}
		}
	}

	async fn send_eww_update(name: LineName, value: &str) -> Result<()> {
		tokio::process::Command::new("sh")
			.arg("-c")
			.arg(format!("eww update btc_line_{name}_str=\"{value}\""))
			.status()
			.await
			.map_err(|e| eyre::eyre!(e))?;
		Ok(())
	}
}
