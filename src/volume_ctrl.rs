use std::process::Command;

pub enum VolumePeriph {
    DefaultSource,
    DefaultSink,
    DefaultVideoSource,
    Custom(usize),
}

impl VolumePeriph {
    fn get_identifier(&self) -> String {
        match self {
            VolumePeriph::DefaultSource => "@DEFAULT_AUDIO_SOURCE@".into(),
            VolumePeriph::DefaultSink => "@DEFAULT_AUDIO_SINK@".into(),
            VolumePeriph::DefaultVideoSource => "@DEFAULT_VIDEO_SOURCE@".into(),
            VolumePeriph::Custom(id) => id.to_string(),
        }
    }
}

pub struct Volume {
    pub volume: f64,
    pub muted: bool,
}

impl Volume {
    pub fn get(periph: VolumePeriph) -> Self {
        let output = Command::new("wpctl")
            .args(["get-volume", periph.get_identifier().as_str()])
            .output()
            .expect("Failed to run wpctl");

        let output_str = String::from_utf8_lossy(&output.stdout);
        let parts: Vec<&str> = output_str.trim().split_whitespace().collect();
        let volume = parts
            .get(1)
            .and_then(|v| v.parse::<f64>().ok())
            .unwrap_or(0.0);
        let muted = output_str.contains("[MUTED]");

        Self { volume, muted }
    }

    pub fn set(value: f64, periph: VolumePeriph) {
        let out = Command::new("wpctl")
            .args([
                "set-volume",
                periph.get_identifier().as_str(),
                value.to_string().as_str(),
            ])
            .output()
            .expect("Failed to run wpctl");
    }
}
