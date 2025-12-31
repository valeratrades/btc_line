use clap::Parser;
use v_utils::{
	macros::{LiveSettings, MyConfigPrimitives, Settings, SettingsNested},
	trades::Timeframe,
};

#[derive(Clone, Debug, LiveSettings, MyConfigPrimitives, Settings)]
pub struct AppConfig {
	#[settings(flatten)]
	pub spy: Spy,
	#[settings(default = 24)]
	pub comparison_offset_h: usize,
	/// whether to label the displayed values, or (false ->) assume user's acquaintance with the layout
	pub label: bool,
	#[settings(flatten)]
	pub outputs: Outputs,
}

#[derive(Clone, Debug, serde::Deserialize, SettingsNested, smart_default::SmartDefault)]
pub struct Outputs {
	#[default = false]
	pub eww: bool,
	/// Rate limit for eww updates. If set, eww updates will be batched and sent at most once per this duration.
	pub eww_rate_limit: Option<Timeframe>,
	#[default = true]
	pub pipes: bool,
}

#[derive(Clone, Debug, Default, MyConfigPrimitives, SettingsNested)]
pub struct Spy {
	pub alpaca_key: String,
	pub alpaca_secret: String,
}

/// CLI struct with SettingsFlags for clap integration
#[derive(Debug, Parser)]
pub struct Cli {
	#[clap(flatten)]
	pub settings_flags: SettingsFlags,
}
