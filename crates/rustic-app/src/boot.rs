//! Boot-time logging, panic dump policy, and Android logger setup.
//! See `PLAN.md` Section 14.

use std::path::PathBuf;
use tracing_subscriber::EnvFilter;

/// Install the global tracing subscriber. Reads `RUST_LOG` if set,
/// otherwise defaults to `info` for our crates.
pub fn init_logging() {
    #[cfg(target_os = "android")]
    {
        let _ = android_logger::init_once(
            android_logger::Config::default()
                .with_max_level(log::LevelFilter::Info)
                .with_tag("rustic"),
        );
    }

    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,rustic=debug"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .try_init();
}

/// Install a panic hook that writes a dump file in the platform settings
/// directory and preserves stderr where available. Windows release builds
/// have no guaranteed console; the dump file is the primary signal.
pub fn install_panic_hook() {
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let payload = info.to_string();
        let backtrace = std::backtrace::Backtrace::force_capture();

        eprintln!("rustic panic: {payload}\n{backtrace}");

        if let Some(dump_path) = panic_dump_path() {
            let _ = write_dump(&dump_path, &payload, &backtrace.to_string());
        }

        prev(info);
    }));
}

fn panic_dump_path() -> Option<PathBuf> {
    let dir = settings_dir()?;
    let _ = std::fs::create_dir_all(&dir);
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    Some(dir.join(format!("panic-{stamp}.log")))
}

fn write_dump(path: &PathBuf, payload: &str, backtrace: &str) -> std::io::Result<()> {
    use std::io::Write;
    let mut f = std::fs::File::create(path)?;
    writeln!(f, "RusticV3 panic dump")?;
    writeln!(f, "===")?;
    writeln!(f, "{payload}")?;
    writeln!(f, "---")?;
    writeln!(f, "{backtrace}")?;
    Ok(())
}

/// Platform settings directory. See `PLAN.md` Section 12.
pub fn settings_dir() -> Option<PathBuf> {
    #[cfg(target_os = "android")]
    {
        // Android uses app-private storage; resolution requires the JNI
        // context, which we don't have yet at boot. Defer until the
        // android winit shim lands.
        return None;
    }
    #[cfg(not(target_os = "android"))]
    {
        let base = if cfg!(target_os = "windows") {
            std::env::var_os("APPDATA").map(PathBuf::from)
        } else if cfg!(target_os = "macos") {
            home_dir().map(|h| h.join("Library/Application Support"))
        } else {
            std::env::var_os("XDG_CONFIG_HOME")
                .map(PathBuf::from)
                .or_else(|| home_dir().map(|h| h.join(".config")))
        };
        base.map(|b| b.join("RusticV3"))
    }
}

#[cfg(not(target_os = "android"))]
fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("USERPROFILE").map(PathBuf::from))
}
