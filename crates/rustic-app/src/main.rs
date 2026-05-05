//! RusticV3 entry point. See `PLAN.md` Sections 7, 11, 14.

use anyhow::Result;
use rustic_app::AppOptions;

fn main() -> Result<()> {
    rustic_app::run(AppOptions::default())
}
