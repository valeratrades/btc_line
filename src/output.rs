use color_eyre::eyre::Result;

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
		if self.config.output == *"eww" {
			std::process::Command::new("sh")
				.arg("-c")
				.arg(format!("eww update btc_line_main_str=\"{}\"", self.main_line_str))
				.status()
				.expect("eww daemon is not running");
			std::process::Command::new("sh")
				.arg("-c")
				.arg(format!("eww update btc_line_spy_str=\"{}\"", self.spy_line_str))
				.status()
				.expect("eww daemon is not running");
			std::process::Command::new("sh")
				.arg("-c")
				.arg(format!("eww update btc_line_additional_str=\"{}\"", self.additional_line_str))
				.status()
				.expect("eww daemon is not running");
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
