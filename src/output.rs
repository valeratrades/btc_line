use std::{collections::HashMap, rc::Rc};

use color_eyre::eyre::{self, Result};
use tracing::instrument;
use v_utils::define_str_enum;

use crate::config::Settings;

define_str_enum! {
	#[derive(Debug, Hash, Eq, PartialEq, Clone, Copy)]
	pub enum LineName {
		Additional => "additional",
		Main => "main",
		Spy => "spy",
	}
}

#[derive(Debug, Default, Clone)]
pub struct Output {
	settings: Rc<Settings>,
	old_vals: HashMap<LineName, String>,
}
impl Output {
	pub fn new(settings: Rc<Settings>) -> Self {
		Self { settings, ..Default::default() }
	}

	#[instrument(skip_all, fields(?name, new_value))]
	pub async fn output(&mut self, name: LineName, new_value: String) -> Result<()> {
		if self.old_vals.get(&name).map(|v| v == &new_value).unwrap_or(false) {
			return Ok(());
		}
		self.old_vals.insert(name, new_value.clone());

		let new_value_clone = new_value.clone();
		let eww_update_handler = async {
			if self.settings.config()?.outputs.eww {
				tokio::process::Command::new("sh")
					.arg("-c")
					.arg(format!("eww update btc_line_{name}_str=\"{new_value_clone}\""))
					.status()
					.await
					.map_err(|e| eyre::eyre!(e))?;
			}
			Ok::<_, eyre::Report>(())
		};

		let file_update_handler = async {
			let file_path = v_utils::xdg_state_file!(name.to_string());

			if self.settings.config()?.outputs.pipes {
				tokio::fs::write(&file_path, format!("{new_value}\n")).await.map_err(|e| eyre::eyre!(e))?;

				// Update timestamp file
				let timestamp_path = v_utils::xdg_state_file!(".timestamps");
				let timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(|e| eyre::eyre!(e))?.as_secs();

				// Format as ISO8601 manually
				let secs_since_epoch = timestamp;
				let days = secs_since_epoch / 86400;
				let rem = secs_since_epoch % 86400;
				let hours = rem / 3600;
				let mins = (rem % 3600) / 60;
				let secs = rem % 60;

				// Days since Unix epoch (1970-01-01) to approximate date
				let year = 1970 + (days / 365);
				let day_of_year = days % 365;
				let month = 1 + (day_of_year / 30); // Rough approximation
				let day = 1 + (day_of_year % 30);

				let timestamp_iso = format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", year, month, day, hours, mins, secs);
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
}
