use config::{ConfigError, File};
use serde::de::{self, Deserializer, Visitor};
use serde::Deserialize;
use std::env;
use std::fmt;
use v_utils::io::ExpandedPath;

#[derive(Deserialize, Clone, Debug)]
pub struct AppConfig {
	pub spy: Spy,
	pub additional_line: AdditionalLine,
	pub comparison_offset_h: usize,
	pub label: bool,
	pub output: String,
}

#[derive(Clone, Debug)]
pub struct Spy {
	pub alpaca_key: String,
	pub alpaca_secret: String,
}
impl<'de> Deserialize<'de> for Spy {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		#[derive(Deserialize)]
		struct Helper {
			alpaca_key: EnvOrDirect,
			alpaca_secret: EnvOrDirect,
		}

		let helper = Helper::deserialize(deserializer)?;
		Ok(Spy {
			alpaca_key: helper.alpaca_key.into_string(),
			alpaca_secret: helper.alpaca_secret.into_string(),
		})
	}
}

#[derive(Deserialize, Clone, Debug)]
pub struct AdditionalLine {
	pub show_by_default: bool,
}

#[derive(Debug, Clone)]
pub enum EnvOrDirect {
	Direct(String),
	Env(String),
}

impl<'de> Deserialize<'de> for EnvOrDirect {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		struct EnvOrDirectVisitor;

		impl<'de> Visitor<'de> for EnvOrDirectVisitor {
			type Value = EnvOrDirect;

			fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
				formatter.write_str("a string or a map with 'env'")
			}

			fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
			where
				E: de::Error,
			{
				Ok(EnvOrDirect::Direct(value.to_owned()))
			}

			fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
			where
				M: de::MapAccess<'de>,
			{
				let key: Option<String> = map.next_key()?;
				match key.as_deref() {
					Some("env") => {
						let env_var: String = map.next_value()?;
						if let Ok(val) = env::var(&env_var) {
							Ok(EnvOrDirect::Direct(val))
						} else {
							Err(de::Error::custom(format!("Environment variable '{}' not set", env_var)))
						}
					}
					_ => Err(de::Error::unknown_field(key.as_deref().unwrap_or(""), &["env"])),
				}
			}
		}

		deserializer.deserialize_any(EnvOrDirectVisitor)
	}
}
impl EnvOrDirect {
	pub fn into_string(self) -> String {
		match self {
			EnvOrDirect::Direct(value) => value,
			EnvOrDirect::Env(value) => env::var(&value).unwrap_or_default(),
		}
	}
}

impl AppConfig {
	pub fn new(path: ExpandedPath) -> Result<Self, ConfigError> {
		let builder = config::Config::builder()
			.set_default("comparison_offset_h", 24)?
			.add_source(File::with_name(&path.to_string()));

		let settings: config::Config = builder.build()?;
		let settings: Self = settings.try_deserialize()?;

		if settings.comparison_offset_h > 24 {
			return Err(ConfigError::Message("comparison limits above a day are not supported".into()));
		}
		Ok(settings)
	}
}
