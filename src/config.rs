use config::{ConfigError, File};
use serde::Deserialize;
use v_utils::{io::ExpandedPath, macros::MyConfigPrimitives};

#[derive(Deserialize, Clone, Debug)]
pub struct AppConfig {
	pub spy: Spy,
	pub comparison_offset_h: usize,
	pub label: bool,
	pub output: String,
}

#[derive(Clone, Debug, MyConfigPrimitives)]
pub struct Spy {
	pub alpaca_key: String,
	pub alpaca_secret: String,
}


impl AppConfig {
	pub fn new(path: ExpandedPath) -> Result<Self, ConfigError> {
		let builder = config::Config::builder().set_default("comparison_offset_h", 24)?.add_source(File::with_name(&path.to_string()));

		let settings: config::Config = builder.build()?;
		let settings: Self = settings.try_deserialize()?;

		if settings.comparison_offset_h > 24 {
			return Err(ConfigError::Message("comparison limits above a day are not supported".into()));
		}
		Ok(settings)
	}
}
