//! `xtask` — workspace tooling. See `PLAN.md` Sections 6 and 15.
//!
//! Subcommands:
//!   - `bake [--check]` — bake source assets into `assets/baked/` and write
//!     a manifest. Currently a no-op that emits the manifest shape so CI
//!     and `--check` can verify staleness once real bakers land.
//!   - `regression` — placeholder for the visual regression runner.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() -> Result<()> {
    let mut args = env::args().skip(1);
    let cmd = args.next().unwrap_or_default();
    match cmd.as_str() {
        "bake" => {
            let check = args.any(|a| a == "--check");
            cmd_bake(check)
        }
        "regression" => cmd_regression(),
        "" | "help" | "--help" | "-h" => {
            print_help();
            Ok(())
        }
        other => {
            print_help();
            anyhow::bail!("unknown xtask subcommand: {other}")
        }
    }
}

fn print_help() {
    println!("xtask <command>");
    println!();
    println!("Commands:");
    println!("  bake [--check]   Bake assets/source/ into assets/baked/.");
    println!("                   --check fails if the manifest would change.");
    println!("  regression       Run the visual regression suite (TODO).");
    println!("  help             Show this message.");
}

#[derive(Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
struct BakeManifest {
    /// Manifest schema version. Bump on breaking changes.
    version: u32,
    /// Logical-path -> baked-relative-path map. Empty until real bakers ship.
    entries: std::collections::BTreeMap<String, String>,
}

fn cmd_bake(check: bool) -> Result<()> {
    let root = workspace_root()?;
    let baked = root.join("assets/baked");
    fs::create_dir_all(&baked).with_context(|| format!("create {}", baked.display()))?;

    let manifest = BakeManifest {
        version: 1,
        ..Default::default()
    };
    let manifest_path = baked.join("manifest.json");
    let new_text = serde_json::to_string_pretty(&manifest)? + "\n";

    if check {
        let existing = fs::read_to_string(&manifest_path).unwrap_or_default();
        if existing != new_text {
            anyhow::bail!(
                "baked manifest is stale: run `cargo xtask bake` ({})",
                manifest_path.display()
            );
        }
        println!("bake: manifest up-to-date");
    } else {
        fs::write(&manifest_path, &new_text)
            .with_context(|| format!("write {}", manifest_path.display()))?;
        println!(
            "bake: wrote {} ({} entries)",
            manifest_path.display(),
            manifest.entries.len()
        );
    }
    Ok(())
}

fn cmd_regression() -> Result<()> {
    eprintln!("regression: not implemented yet (Phase 11). See docs/ci.md.");
    Ok(())
}

fn workspace_root() -> Result<PathBuf> {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let candidate = Path::new(manifest_dir)
        .parent()
        .context("xtask is not at workspace root/xtask")?;
    if !candidate.join("Cargo.toml").exists() {
        anyhow::bail!(
            "workspace Cargo.toml not found above xtask: {}",
            candidate.display()
        );
    }
    Ok(candidate.to_path_buf())
}
