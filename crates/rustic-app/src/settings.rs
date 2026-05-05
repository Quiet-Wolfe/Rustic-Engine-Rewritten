//! Settings loader. See `PLAN.md` Section 12.
//!
//! Settings are human-readable TOML. Corrupt files are preserved with a
//! `.bad` suffix before defaults are regenerated. Writes go to a temp file
//! in the same directory, flush, then rename into place.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
#[non_exhaustive]
pub struct Settings {
    pub render: RenderSettings,
    pub audio: AudioSettings,
    pub input: InputSettings,
}

impl Settings {
    pub fn load_or_default(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }

        let text =
            std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
        match toml::from_str(&text) {
            Ok(settings) => Ok(settings),
            Err(_) => {
                preserve_bad_file(path)?;
                Ok(Self::default())
            }
        }
    }

    pub fn save_atomic(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create {}", parent.display()))?;
        }

        let text = toml::to_string_pretty(self).context("serialize settings toml")?;
        let tmp = temp_path(path);
        {
            let mut file =
                std::fs::File::create(&tmp).with_context(|| format!("create {}", tmp.display()))?;
            file.write_all(text.as_bytes())
                .with_context(|| format!("write {}", tmp.display()))?;
            file.sync_all()
                .with_context(|| format!("sync {}", tmp.display()))?;
        }
        std::fs::rename(&tmp, path)
            .with_context(|| format!("rename {} -> {}", tmp.display(), path.display()))?;
        Ok(())
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
#[non_exhaustive]
pub struct RenderSettings {
    /// Optional backend override, e.g. "vulkan", "dx12". `None` lets
    /// `wgpu::Backends::PRIMARY` choose. Debug-only.
    pub backend_override: Option<String>,
    /// Whether to surface `wgpu` adapter/backend/limits in the F3 overlay.
    pub debug_show_adapter: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[non_exhaustive]
pub struct AudioSettings {
    pub master_volume: f32,
    pub music_volume: f32,
    pub sfx_volume: f32,
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            master_volume: 1.0,
            music_volume: 1.0,
            sfx_volume: 1.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[non_exhaustive]
pub struct InputSettings {
    /// Touch enable for Android. See `PLAN.md` Section 10.
    pub touch_enabled: bool,
    pub touch_lane_height_px: u32,
    pub touch_opacity: f32,
}

impl Default for InputSettings {
    fn default() -> Self {
        Self {
            touch_enabled: false,
            touch_lane_height_px: 240,
            touch_opacity: 0.45,
        }
    }
}

fn temp_path(path: &Path) -> PathBuf {
    let mut tmp = path.to_path_buf();
    let next_ext = match path.extension().and_then(|e| e.to_str()) {
        Some(ext) => format!("{ext}.tmp"),
        None => "tmp".to_string(),
    };
    tmp.set_extension(next_ext);
    tmp
}

fn preserve_bad_file(path: &Path) -> Result<PathBuf> {
    let bad = bad_path(path);
    std::fs::rename(path, &bad).with_context(|| {
        format!(
            "rename corrupt settings {} -> {}",
            path.display(),
            bad.display()
        )
    })?;
    Ok(bad)
}

fn bad_path(path: &Path) -> PathBuf {
    let base = path.with_file_name(format!(
        "{}.bad",
        path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("settings")
    ));
    if !base.exists() {
        return base;
    }

    for i in 1..1000 {
        let candidate = path.with_file_name(format!(
            "{}.bad.{i}",
            path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("settings")
        ));
        if !candidate.exists() {
            return candidate;
        }
    }
    path.with_file_name("settings.bad.overflow")
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_playable_values() {
        let settings = Settings::default();
        assert_eq!(settings.audio.master_volume, 1.0);
        assert_eq!(settings.audio.music_volume, 1.0);
        assert_eq!(settings.audio.sfx_volume, 1.0);
        assert_eq!(settings.input.touch_lane_height_px, 240);
        assert_eq!(settings.input.touch_opacity, 0.45);
    }

    #[test]
    fn saves_and_loads_toml() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.toml");
        let mut settings = Settings::default();
        settings.audio.master_volume = 0.75;
        settings.input.touch_enabled = true;

        settings.save_atomic(&path).unwrap();
        let loaded = Settings::load_or_default(&path).unwrap();

        assert_eq!(loaded.audio.master_volume, 0.75);
        assert!(loaded.input.touch_enabled);
    }

    #[test]
    fn corrupt_settings_are_preserved_as_bad() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.toml");
        std::fs::write(&path, "not = [valid").unwrap();

        let loaded = Settings::load_or_default(&path).unwrap();

        assert_eq!(loaded.audio.master_volume, 1.0);
        assert!(!path.exists());
        assert!(dir.path().join("settings.toml.bad").exists());
    }
}
