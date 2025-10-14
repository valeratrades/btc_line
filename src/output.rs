use std::{collections::HashMap, io::Write, os::unix::fs::OpenOptionsExt};

use color_eyre::eyre::{self, Result, bail};
use tracing::instrument;
use v_utils::{define_str_enum, xdg_state};

use crate::config::AppConfig;

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
	config: AppConfig,
	values: HashMap<LineName, String>,
}
impl Output {
	#[instrument(skip_all, fields(?name, new_value))]
	pub async fn output(&mut self, name: LineName, new_value: String) -> Result<()> {
		if self.values.get(&name).map(|v| v == &new_value).unwrap_or(false) {
			return Ok(());
		}
		self.values.insert(name, new_value);

		let eww_update_handler = tokio::process::Command::new("sh").arg("-c").arg(format!("eww update btc_line_main_str=\"{name}_line\"")).status();

		let pipe_update_handler = {
			let pipe_path = xdg_state!(name.to_string());
			if !pipe_path.exists() {
				tokio::process::Command::new("mkfifo").arg(pipe_path.display().to_string()).status().await?;
			}

			{
				if let Ok(mut file) = std::fs::OpenOptions::new().write(true).custom_flags(libc::O_NONBLOCK).open(pipe_path) {
					let _ = writeln!(file, "{new_value}");
				}
			}
		};

		tokio::try_join!(eww_update_handler, pipe_update_handler)
	}
}
