use std::{
	cell::RefCell,
	path::{Path, PathBuf},
	time::{Duration, SystemTime},
};

use config::{ConfigError, File};
use serde::Deserialize;
use v_utils::macros::MyConfigPrimitives;

#[derive(Clone, Debug, Default, MyConfigPrimitives)]
pub struct AppConfig {
	pub spy: Spy,
	pub comparison_offset_h: usize,
	/// whether to label the displayed values, or (false ->) assume user's acquaintance with the layout
	pub label: bool,
	pub outputs: Outputs,
}

#[derive(Clone, Debug, MyConfigPrimitives)]
pub struct Outputs {
	pub eww: bool,
	pub pipes: bool,
}

impl Default for Outputs {
	fn default() -> Self {
		Self { eww: false, pipes: true }
	}
}

#[derive(Clone, Debug, Default, MyConfigPrimitives)]
pub struct Spy {
	pub alpaca_key: String,
	pub alpaca_secret: String,
}

impl AppConfig {
	pub fn try_build(path: &Path) -> Result<Self, SettingsError> {
		let builder = config::Config::builder()
			.set_default("comparison_offset_h", 24)?
			.add_source(File::with_name(&path.display().to_string()));

		let conf: config::Config = builder.build()?;
		let conf: Self = conf.try_deserialize()?;

		if conf.comparison_offset_h > 24 {
			return Err(ConfigError::Message("comparison limits above a day are not supported".into()).into());
		}
		Ok(conf)
	}
}

//TODO: define a `flags_conf` struct, that is `AppConfig`, constructed from flags only, and frozen in place, so we can run further config updates against it (as flags always win)

#[derive(Clone, Debug, Deserialize)]
struct TimeCapsule<T> {
	pub value: T,
	pub init_t: SystemTime,
	upd_freq: Duration,
}
impl<T: Default> Default for TimeCapsule<T> {
	fn default() -> Self {
		Self {
			value: T::default(),
			init_t: std::time::UNIX_EPOCH,
			upd_freq: Duration::default(),
		}
	}
}

/// # General
/// If config is updated, new values from there should overwrite settings cache
#[derive(Clone, Debug, Default, Deserialize)]
pub struct Settings {
	config_path: PathBuf,
	config: RefCell<TimeCapsule<AppConfig>>,
}
#[derive(Debug, derive_more::Display, thiserror::Error, derive_more::From)]
pub enum SettingsError {
	Config(ConfigError),
	Io(std::io::Error),
}

impl Settings {
	pub fn new(path: PathBuf, config_update_freq: Duration) -> Self {
		Self {
			config_path: path,
			config: RefCell::new(TimeCapsule {
				upd_freq: config_update_freq,
				..Default::default()
			}),
		}
	}

	pub fn config(&self) -> Result<AppConfig, SettingsError> {
		let system_now = SystemTime::now();
		let since_source_change: Duration = {
			let last_modified: SystemTime = std::fs::metadata(&self.config_path)?.modified()?;
			system_now.duration_since(last_modified).unwrap()
		};

		let mut conf_capsule = match self.config.try_borrow_mut() {
			Ok(v) => v,
			//SAFETY: basically, take the previously known conf values, if the conf is currently being updated from some other call // with correct Control Flow, shouldn't happen anyways though
			Err(_) => unsafe {
				let ptr = self.config.as_ptr();
				return Ok((*ptr).value.clone());
			},
		};
		let capsule_age: Duration = system_now.duration_since(conf_capsule.init_t).unwrap();
		if capsule_age < conf_capsule.upd_freq {
			return Ok(conf_capsule.value.clone());
		}

		if since_source_change < capsule_age {
			conf_capsule.value = AppConfig::try_build(&self.config_path)?;
		}
		conf_capsule.init_t = system_now;

		Ok(conf_capsule.value.clone()) //HACK: pretty sure we could do here with just a ref, if I figure out how to handle Result type around it
	}
}
