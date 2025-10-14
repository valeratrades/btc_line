use std::{fs, io::Write, os::unix::fs::OpenOptionsExt, path::PathBuf};

use color_eyre::eyre::Result;
use tracing::instrument;
use v_utils::xdg_state;

use crate::config::AppConfig;

#[derive(Debug, Default)]
pub struct Output {
	config: AppConfig,
	pub main_line_str: String,
	pub spy_line_str: String,
	pub additional_line_str: String,
}

//? potentially, could store last modified for every value, subsequently that all of them are recent enough when called from the main loop.
impl Output {
	pub fn out(&self) -> Result<()> {
		// Create /tmp/btc_line directory if it doesn't exist
		let pipe_dir = xdg_state!("");
		if !pipe_dir.exists() {
			fs::create_dir_all(&pipe_dir)?;
		}

		// Write to named pipes in parallel with eww updates
		let main_line = self.main_line_str.clone();
		let spy_line = self.spy_line_str.clone();
		let additional_line = self.additional_line_str.clone();

		// Spawn threads for parallel execution
		let eww_handle = if self.config.output == *"eww" {
			Some(std::thread::spawn(move || {
				std::process::Command::new("sh")
					.arg("-c")
					.arg(format!("eww update btc_line_main_str=\"{main_line}\""))
					.status()
					.expect("eww daemon is not running");
				std::process::Command::new("sh")
					.arg("-c")
					.arg(format!("eww update btc_line_spy_str=\"{spy_line}\""))
					.status()
					.expect("eww daemon is not running");
				std::process::Command::new("sh")
					.arg("-c")
					.arg(format!("eww update btc_line_additional_str=\"{additional_line}\""))
					.status()
					.expect("eww daemon is not running");
			}))
		} else {
			None
		};

		// Write to named pipes
		let pipe_handle = {
			let main_line = self.main_line_str.clone();
			let main_file = pipe_dir.join("main");

			let spy_line = self.spy_line_str.clone();
			let spy_file = pipe_dir.join("spy");

			let additional_line = self.additional_line_str.clone();
			let additional_file = pipe_dir.join("additional");

			std::thread::spawn(move || -> Result<()> {
				Self::write_to_pipe(main_file, &main_line)?;
				Self::write_to_pipe(spy_file, &spy_line)?;
				Self::write_to_pipe(additional_file, &additional_line)?;
				Ok(())
			})
		};

		// Wait for both operations to complete
		if let Some(handle) = eww_handle {
			handle.join().unwrap();
		}
		pipe_handle.join().unwrap()?;

		Ok(())
	}

	#[instrument]
	fn write_to_pipe(pipe_path: PathBuf, content: &str) -> Result<()> {
		// Create named pipe if it doesn't exist
		if !pipe_path.exists() {
			std::process::Command::new("mkfifo").arg(pipe_path.display().to_string()).status()?;
		}

		{
			if let Ok(mut file) = std::fs::OpenOptions::new().write(true).custom_flags(libc::O_NONBLOCK).open(pipe_path) {
				let _ = writeln!(file, "{content}");
			}
		}

		Ok(())
	}

	pub fn new(config: AppConfig) -> Self {
		Self { config, ..Default::default() }
	}
}
