use std::{
	collections::HashMap,
	pin::Pin,
	sync::{Arc, Mutex},
	time::Instant,
};

use color_eyre::eyre::{self, Result};
use tracing::instrument;
use v_utils::define_str_enum;

use crate::config::LiveSettings;

/// Deferred flush of a rate-limited eww update. Driven by the caller (no `tokio::spawn`).
pub type FlushFut = Pin<Box<dyn std::future::Future<Output = ()> + Send + 'static>>;

define_str_enum! {
	#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
	pub enum LineName {
		Additional => "additional",
		Main => "main",
		Spy => "spy",
	}
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

	/// Returns an optional flush future. The caller must drive it to completion.
	/// `None` means no deferred work is pending.
	#[instrument(skip_all, fields(?name, new_value))]
	pub async fn output(&mut self, name: LineName, new_value: String) -> Result<Option<FlushFut>> {
		if self.old_vals.get(&name).map(|v| v == &new_value).unwrap_or(false) {
			return Ok(None);
		}
		self.old_vals.insert(name, new_value.clone());

		let eww_update_handler = self.handle_eww_update(name, new_value.clone());

		let new_value_for_file = new_value.clone();
		let settings = self.settings.clone();
		let file_update_handler = async move {
			let file_path = v_utils::xdg_state_file!(name.to_string());

			if settings.config().unwrap().outputs.pipes {
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

		let (flush, ()) = tokio::try_join!(eww_update_handler, file_update_handler)?;
		Ok(flush)
	}

	async fn handle_eww_update(&self, name: LineName, new_value: String) -> Result<Option<FlushFut>> {
		let config = self.settings.config().unwrap();
		if !config.outputs.eww {
			return Ok(None);
		}

		let Some(rate_limit) = config.outputs.eww_rate_limit else {
			Self::send_eww_update(name, &new_value).await?;
			return Ok(None);
		};

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
			return Ok(None);
		}

		if should_schedule_flush {
			let states = self.eww_rate_limit_states.clone();
			return Ok(Some(Box::pin(Self::flush_pending_eww_update(name, states, duration))));
		}

		Ok(None)
	}

	/// Drains pending eww updates for `name`, respecting the rate limit. Loops until `pending_value`
	/// is empty after a send, only then clears `flush_scheduled`. Holding the flag for the full
	/// drain (rather than clearing it before send) means concurrent `output()` calls during a send
	/// just stash their value in `pending_value` and we pick it up on the next loop iteration —
	/// no chance of orphaned pending values.
	async fn flush_pending_eww_update(name: LineName, states: Arc<Mutex<HashMap<LineName, EwwRateLimitState>>>, rate_limit_duration: std::time::Duration) {
		loop {
			let wait = {
				let states = states.lock().unwrap();
				let state = states.get(&name).expect("flush_scheduled implies entry exists");
				let now = Instant::now();
				state
					.last_sent
					.and_then(|last| {
						let elapsed = now.duration_since(last);
						(elapsed < rate_limit_duration).then(|| rate_limit_duration - elapsed)
					})
					.unwrap_or(std::time::Duration::ZERO)
			};
			if !wait.is_zero() {
				tokio::time::sleep(wait).await;
			}

			let value_to_send = {
				let mut states = states.lock().unwrap();
				let state = states.entry(name).or_default();
				match state.pending_value.take() {
					Some(v) => {
						state.last_sent = Some(Instant::now());
						Some(v)
					}
					None => {
						state.flush_scheduled = false;
						None
					}
				}
			};

			match value_to_send {
				Some(v) =>
					if let Err(e) = Self::send_eww_update(name, &v).await {
						tracing::error!("Failed to send eww update: {e}");
					},
				None => break,
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

#[derive(Debug, Default)]
struct EwwRateLimitState {
	last_sent: Option<Instant>,
	pending_value: Option<String>,
	flush_scheduled: bool,
}
