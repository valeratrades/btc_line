use std::{io::Write, os::unix::fs::OpenOptionsExt, path::PathBuf};

use color_eyre::eyre::Result;
use tokio::fs;
use tracing::instrument;
use v_utils::xdg_state;

use crate::config::AppConfig;

#[derive(Debug, Default, Clone)]
pub struct Output {
	config: AppConfig,
	pub main_line_str: String,
	pub spy_line_str: String,
	pub additional_line_str: String,
}

//? potentially, could store last modified for every value, subsequently that all of them are recent enough when called from the main loop.
impl Output {
	pub async fn out(&self) -> Result<()> {
		// Create /tmp/btc_line directory if it doesn't exist
		let pipe_dir = xdg_state!("");
		if !pipe_dir.exists() {
			fs::create_dir_all(&pipe_dir).await?;
		}

		// Write to named pipes in parallel with eww updates
		let main_line = self.main_line_str.clone();
		let spy_line = self.spy_line_str.clone();
		let additional_line = self.additional_line_str.clone();

		// Spawn async tasks for parallel execution
		let eww_task = if self.config.output == *"eww" {
			Some(tokio::spawn(async move {
				let _ = tokio::process::Command::new("sh")
					.arg("-c")
					.arg(format!("eww update btc_line_main_str=\"{main_line}\""))
					.status()
					.await;
				let _ = tokio::process::Command::new("sh")
					.arg("-c")
					.arg(format!("eww update btc_line_spy_str=\"{spy_line}\""))
					.status()
					.await;
				let _ = tokio::process::Command::new("sh")
					.arg("-c")
					.arg(format!("eww update btc_line_additional_str=\"{additional_line}\""))
					.status()
					.await;
			}))
		} else {
			None
		};

		// Write to named pipes
		let pipe_task = {
			let main_line = self.main_line_str.clone();
			let main_file = pipe_dir.join("main");

			let spy_line = self.spy_line_str.clone();
			let spy_file = pipe_dir.join("spy");

			let additional_line = self.additional_line_str.clone();
			let additional_file = pipe_dir.join("additional");

			tokio::spawn(async move {
				Self::write_to_pipe(main_file, &main_line).await?;
				Self::write_to_pipe(spy_file, &spy_line).await?;
				Self::write_to_pipe(additional_file, &additional_line).await?;
				Ok::<_, color_eyre::eyre::Error>(())
			})
		};

		// Wait for both operations to complete
		if let Some(task) = eww_task {
			let _ = task.await;
		}
		pipe_task.await??;

		Ok(())
	}

	#[instrument]
	async fn write_to_pipe(pipe_path: PathBuf, content: &str) -> Result<()> {
		// Create named pipe if it doesn't exist
		if !pipe_path.exists() {
			tokio::process::Command::new("mkfifo").arg(pipe_path.display().to_string()).status().await?;
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
