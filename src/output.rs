use std::{collections::HashMap, io::Write, os::unix::fs::OpenOptionsExt, rc::Rc};

use color_eyre::eyre::{self, Result, bail};
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
			tokio::process::Command::new("sh")
				.arg("-c")
				.arg(format!("eww update btc_line_{name}_str=\"{new_value_clone}\""))
				.status()
				.await
				.map_err(|e| eyre::eyre!(e))?;
			Ok::<_, eyre::Report>(())
		};

		let pipe_update_handler = async {
			let pipe_path = v_utils::xdg_state_file!(name.to_string());
			if !pipe_path.exists() {
				let status = tokio::process::Command::new("mkfifo")
					.arg(pipe_path.display().to_string())
					.status()
					.await
					.map_err(|e| eyre::eyre!(e))?;

				// Ignore "already exists" errors (exit code 1 from mkfifo)
				if !status.success() && status.code() != Some(1) {
					bail!("mkfifo failed with status: {status}");
				}
			}

			tokio::task::spawn_blocking(move || {
				if let Ok(mut file) = std::fs::OpenOptions::new().write(true).custom_flags(libc::O_NONBLOCK).open(pipe_path) {
					let _ = writeln!(file, "{new_value}");
				}
			})
			.await
			.map_err(|e| eyre::eyre!(e))?;

			Ok::<_, eyre::Report>(())
		};

		tokio::try_join!(eww_update_handler, pipe_update_handler)?;
		Ok(())
	}
}
