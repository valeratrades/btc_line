use color_eyre::eyre::Result;
use std::fs;
use std::io::Write;
use std::path::Path;

use crate::config::AppConfig;

#[derive(Debug)]
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
		let pipe_dir = "/tmp/btc_line";
		if !Path::new(pipe_dir).exists() {
			fs::create_dir_all(pipe_dir)?;
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
					.arg(format!("eww update btc_line_main_str=\"{}\"", main_line))
					.status()
					.expect("eww daemon is not running");
				std::process::Command::new("sh")
					.arg("-c")
					.arg(format!("eww update btc_line_spy_str=\"{}\"", spy_line))
					.status()
					.expect("eww daemon is not running");
				std::process::Command::new("sh")
					.arg("-c")
					.arg(format!("eww update btc_line_additional_str=\"{}\"", additional_line))
					.status()
					.expect("eww daemon is not running");
			}))
		} else {
			None
		};

		// Write to named pipes
		let pipe_handle = {
			let main_line = self.main_line_str.clone();
			let spy_line = self.spy_line_str.clone();
			let additional_line = self.additional_line_str.clone();
			
			std::thread::spawn(move || -> Result<()> {
				Self::write_to_pipe(&format!("{}/main", pipe_dir), &main_line)?;
				Self::write_to_pipe(&format!("{}/spy", pipe_dir), &spy_line)?;
				Self::write_to_pipe(&format!("{}/additional", pipe_dir), &additional_line)?;
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

	fn write_to_pipe(pipe_path: &str, content: &str) -> Result<()> {
		// Create named pipe if it doesn't exist
		if !Path::new(pipe_path).exists() {
			std::process::Command::new("mkfifo")
				.arg(pipe_path)
				.status()?;
		}

		// Write to pipe (non-blocking)
		if let Ok(mut file) = std::fs::OpenOptions::new()
			.write(true)
			.open(pipe_path) 
		{
			let _ = writeln!(file, "{}", content);
		}

		Ok(())
	}

	pub fn new(config: AppConfig) -> Self {
		Self {
			config,
			main_line_str: "".to_string(),
			spy_line_str: "".to_string(),
			additional_line_str: "".to_string(),
		}
	}
}
