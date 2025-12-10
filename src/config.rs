use std::{
	path::{Path, PathBuf},
	sync::{Arc, RwLock},
};

use config::{ConfigError, File};
use notify::{RecommendedWatcher, RecursiveMode, Watcher, event::ModifyKind};
use v_utils::macros::MyConfigPrimitives;

#[derive(Clone, Debug, Default, MyConfigPrimitives)]
pub struct AppConfig {
	pub spy: Spy,
	pub comparison_offset_h: usize,
	/// whether to label the displayed values, or (false ->) assume user's acquaintance with the layout
	pub label: bool,
	pub outputs: Outputs,
}

#[derive(Clone, Debug, MyConfigPrimitives, smart_default::SmartDefault)]
pub struct Outputs {
	pub eww: bool,
	#[default(true)]
	pub pipes: bool,
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

/// Config hot reload using notify crate for file change detection.
#[derive(Clone, Debug)]
pub struct Settings {
	config_path: PathBuf,
	config: Arc<RwLock<AppConfig>>,
	watcher: Arc<RwLock<Option<RecommendedWatcher>>>,
}
#[derive(Debug, derive_more::Display, thiserror::Error, derive_more::From)]
pub enum SettingsError {
	Config(ConfigError),
	Io(std::io::Error),
	Notify(notify::Error),
}

impl Settings {
	pub fn new(path: PathBuf, _config_update_freq: std::time::Duration) -> Self {
		let initial_config = AppConfig::try_build(&path).unwrap_or_default();
		let config = Arc::new(RwLock::new(initial_config));

		let settings = Self {
			config_path: path,
			config,
			watcher: Arc::new(RwLock::new(None)),
		};

		settings.start_watcher();
		settings
	}

	fn start_watcher(&self) {
		let config_path = self.config_path.clone();
		let config = Arc::clone(&self.config);
		let watcher_slot = Arc::clone(&self.watcher);

		let watch_path = config_path.clone();
		let watcher_result = notify::recommended_watcher(move |res: Result<notify::Event, notify::Error>| {
			if let Ok(event) = res {
				let dominated_by_modify = event.kind.is_modify() && matches!(event.kind, notify::EventKind::Modify(ModifyKind::Data(_) | ModifyKind::Any));
				let is_remove = event.kind.is_remove();
				let is_create = event.kind.is_create();
				if (dominated_by_modify || is_remove || is_create)
					&& let Ok(new_config) = AppConfig::try_build(&config_path)
					&& let Ok(mut conf) = config.write()
				{
					*conf = new_config;
					tracing::debug!("Config reloaded from {config_path:?}");
				}
			}
		});

		match watcher_result {
			Ok(mut watcher) => {
				// Watch the parent directory if the file doesn't exist yet, otherwise watch the file
				let path_to_watch = if watch_path.exists() {
					watch_path.as_path()
				} else {
					watch_path.parent().unwrap_or(Path::new("."))
				};

				if let Err(e) = watcher.watch(path_to_watch, RecursiveMode::NonRecursive) {
					tracing::warn!("Failed to watch config file: {e}");
				} else {
					if let Ok(mut slot) = watcher_slot.write() {
						*slot = Some(watcher);
					}
					tracing::debug!("Started watching config at {path_to_watch:?}");
				}
			}
			Err(e) => {
				tracing::warn!("Failed to create config watcher: {e}");
			}
		}
	}

	pub fn config(&self) -> Result<AppConfig, SettingsError> {
		let conf = self.config.read().map_err(|_| ConfigError::Message("Failed to acquire config read lock".into()))?;
		Ok(conf.clone())
	}
}
