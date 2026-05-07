//! `xtask` — workspace tooling. See `PLAN.md` Sections 6 and 15.
//!
//! Subcommands:
//!   - `bake [--check]` — bake source assets into `assets/baked/` and write
//!     a manifest. Source assets are copied byte-for-byte for now; later
//!     phases can replace individual extensions with real transforms while
//!     preserving the manifest contract.
//!   - `import-week1` — import normalized Tutorial/Week 1 v-slice chart,
//!     metadata, level, and compatibility list data from the pinned local
//!     `references/Funkin` checkout.
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
        "import-week1" => cmd_import_week1(),
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
    println!("  import-week1     Import OG Tutorial/Week 1 data from references/Funkin.");
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
    let source = root.join("assets/source");
    let baked = root.join("assets/baked");
    fs::create_dir_all(&baked).with_context(|| format!("create {}", baked.display()))?;

    let manifest = build_manifest(&source)?;
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
        check_baked_files(&source, &baked, &manifest)?;
        println!("bake: manifest up-to-date");
    } else {
        write_baked_files(&source, &baked, &manifest)?;
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

fn build_manifest(source: &Path) -> Result<BakeManifest> {
    let mut files = Vec::new();
    collect_files(source, &mut files)?;

    let mut entries = std::collections::BTreeMap::new();
    for file in files {
        let logical = logical_path(source, &file)?;
        entries.insert(logical.clone(), logical);
    }

    Ok(BakeManifest {
        version: 1,
        entries,
    })
}

fn collect_files(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    if !dir.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(dir).with_context(|| format!("read {}", dir.display()))? {
        let entry = entry.with_context(|| format!("read entry in {}", dir.display()))?;
        let path = entry.path();
        let ty = entry
            .file_type()
            .with_context(|| format!("file type {}", path.display()))?;
        if ty.is_dir() {
            collect_files(&path, out)?;
        } else if ty.is_file() {
            out.push(path);
        }
    }
    out.sort();
    Ok(())
}

fn logical_path(source_root: &Path, path: &Path) -> Result<String> {
    let relative = path
        .strip_prefix(source_root)
        .with_context(|| format!("strip {} from {}", source_root.display(), path.display()))?;
    let parts = relative
        .components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>();
    Ok(parts.join("/"))
}

fn write_baked_files(source: &Path, baked: &Path, manifest: &BakeManifest) -> Result<()> {
    for (logical, baked_relative) in &manifest.entries {
        let src = source.join(logical);
        let dst = baked.join(baked_relative);
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
        }
        fs::copy(&src, &dst)
            .with_context(|| format!("copy {} -> {}", src.display(), dst.display()))?;
    }
    Ok(())
}

fn check_baked_files(source: &Path, baked: &Path, manifest: &BakeManifest) -> Result<()> {
    for (logical, baked_relative) in &manifest.entries {
        let src = source.join(logical);
        let dst = baked.join(baked_relative);
        let src_bytes = fs::read(&src).with_context(|| format!("read {}", src.display()))?;
        let dst_bytes = fs::read(&dst).with_context(|| {
            format!(
                "baked file is missing or unreadable: {} (run `cargo xtask bake`)",
                dst.display()
            )
        })?;
        if src_bytes != dst_bytes {
            anyhow::bail!(
                "baked file is stale: {} (run `cargo xtask bake`)",
                dst.display()
            );
        }
    }
    Ok(())
}

fn cmd_import_week1() -> Result<()> {
    let root = workspace_root()?;
    let reference_data = root.join("references/Funkin/assets/preload/data");
    let reference_songs = reference_data.join("songs");
    if !reference_songs.exists() {
        anyhow::bail!(
            "missing pinned v-slice song data at {}",
            reference_songs.display()
        );
    }

    let source_data = root.join("assets/source/data");
    let mut written = 0usize;
    for song in WEEK1_SONGS {
        written += import_song_json_dir(
            &reference_songs.join(song),
            &source_data.join("songs").join(song),
        )?;
    }

    for relative in WEEK1_LEVELS {
        import_text_asset(&reference_data.join(relative), &source_data.join(relative))?;
        written += 1;
    }

    write_freeplay_songlist(&reference_songs, &source_data.join("freeplaySonglist.txt"))?;
    written += 1;

    println!("import-week1: wrote {written} source assets from references/Funkin");
    Ok(())
}

const WEEK1_SONGS: &[&str] = &["tutorial", "bopeebo", "fresh", "dadbattle"];
const WEEK1_LEVELS: &[&str] = &["levels/tutorial.json", "levels/week1.json"];

fn import_song_json_dir(src_dir: &Path, dst_dir: &Path) -> Result<usize> {
    let mut files = Vec::new();
    for entry in fs::read_dir(src_dir).with_context(|| format!("read {}", src_dir.display()))? {
        let entry = entry.with_context(|| format!("read entry in {}", src_dir.display()))?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
            files.push(path);
        }
    }
    files.sort();

    let mut written = 0usize;
    for src in files {
        let file_name = src
            .file_name()
            .context("song json path has no file name")?
            .to_owned();
        import_text_asset(&src, &dst_dir.join(file_name))?;
        written += 1;
    }
    Ok(written)
}

fn write_freeplay_songlist(reference_songs: &Path, dst: &Path) -> Result<()> {
    let mut text = String::new();
    for song in WEEK1_SONGS {
        let metadata_path = reference_songs
            .join(song)
            .join(format!("{song}-metadata.json"));
        let bytes = fs::read(&metadata_path)
            .with_context(|| format!("read {}", metadata_path.display()))?;
        let value: serde_json::Value = serde_json::from_slice(trim_reference_text(&bytes))
            .with_context(|| format!("parse {}", metadata_path.display()))?;
        let name = value
            .get("songName")
            .and_then(serde_json::Value::as_str)
            .with_context(|| format!("missing songName in {}", metadata_path.display()))?;
        text.push_str(name);
        text.push('\n');
    }

    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    fs::write(dst, text).with_context(|| format!("write {}", dst.display()))?;
    Ok(())
}

fn import_text_asset(src: &Path, dst: &Path) -> Result<()> {
    let bytes = fs::read(src).with_context(|| format!("read {}", src.display()))?;
    let trimmed = trim_reference_text(&bytes);
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }

    let mut out = Vec::with_capacity(trimmed.len() + 1);
    out.extend_from_slice(trimmed);
    out.push(b'\n');
    fs::write(dst, out).with_context(|| format!("write {}", dst.display()))?;
    Ok(())
}

fn trim_reference_text(mut bytes: &[u8]) -> &[u8] {
    while let Some((&last, rest)) = bytes.split_last() {
        if last == 0 || last.is_ascii_whitespace() {
            bytes = rest;
        } else {
            break;
        }
    }
    bytes
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
