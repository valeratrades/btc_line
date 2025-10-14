use std::path::{Path, PathBuf};

use config::{ConfigError, File};
use serde::Deserialize;
use v_utils::{io::ExpandedPath, macros::MyConfigPrimitives};

#[derive(Deserialize, Clone, Debug, Default)]
pub struct AppConfig {
	pub spy: Spy,
	pub comparison_offset_h: usize,
	pub label: bool,
	pub output: String,
}

#[derive(Clone, Debug, Default, MyConfigPrimitives)]
pub struct Spy {
	pub alpaca_key: String,
	pub alpaca_secret: String,
}

impl AppConfig {
	pub fn new(path: &Path) -> Result<Self, ConfigError> {
		let builder = config::Config::builder()
			.set_default("comparison_offset_h", 24)?
			.add_source(File::with_name(&path.display().to_string()));

		let conf: config::Config = builder.build()?;
		let conf: Self = conf.try_deserialize()?;

		if conf.comparison_offset_h > 24 {
			return Err(ConfigError::Message("comparison limits above a day are not supported".into()));
		}
		Ok(conf)
	}
}

#[derive(Deserialize, Clone, Debug, Default, derive_new::new)]
pub struct Settings {
	pub config: AppConfig,
	config_path: PathBuf,
}
impl Settings {
	pub async fn watch_config(&mut self, config: &mut AppConfig) {
		todo!();
	}
}
