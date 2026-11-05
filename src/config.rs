use std::path::PathBuf;

#[derive(Debug, serde::Serialize, serde::Deserialize, clap::Parser)]
pub struct PartialConfig {
    #[arg(short, long)]
    pub style: Option<PathBuf>,
    #[arg(short, long)]
    pub config: Option<PathBuf>,
    #[arg(long, action)]
    pub horizontal: bool,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub max_value: Option<f64>,
    //TODO: step, val pos, val shown
}

impl PartialConfig {
    pub fn join(self, other: Option<Self>) -> Self {
        if let Some(other) = other {
            Self {
                style: self.style.or(other.style),
                config: self.config.or(other.config),
                horizontal: self.horizontal || other.horizontal,
                width: self.width.or(other.width),
                height: self.height.or(other.height),
                max_value: self.max_value.or(other.max_value),
            }
        } else {
            self
        }
    }

    pub fn to_config(self) -> Config {
        let default = Config::default();
        Config {
            style: self.style,
            horizontal: self.horizontal,
            width: self.width.unwrap_or(default.width),
            height: self.height.unwrap_or(default.height),
            max_value: self.max_value.unwrap_or(default.max_value),
        }
    }
}

#[derive(Debug)]
pub struct Config {
    pub style: Option<PathBuf>,
    pub horizontal: bool,
    pub width: i32,
    pub height: i32,
    pub max_value: f64,
    //TODO: step, val pos, val shown
}

impl Default for Config {
    fn default() -> Self {
        Self {
            style: None,
            horizontal: false,
            width: (400),
            height: (100),
            max_value: (100.0),
        }
    }
}
