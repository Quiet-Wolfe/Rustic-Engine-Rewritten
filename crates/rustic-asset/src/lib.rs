//! `rustic-asset` — asset resolver, logical paths, watcher, parsers.
//!
//! See `PLAN.md` Section 6. The resolver is the modding contract: every
//! asset access in release crates goes through `AssetResolver`. Direct
//! `std::fs` is allowed only here, in `xtask`, and in `rustic-dev`.

#![deny(clippy::unwrap_used, clippy::expect_used)]
#![deny(unsafe_code)]

pub mod error;
pub mod loaders;
pub mod parsers;
pub mod path;
pub mod resolver;
pub mod source;
pub mod watcher;

pub use error::{AssetError, AssetResult};
pub use loaders::{
    load_bytes, load_character, load_chart, load_png, load_sparrow, load_stage, load_text_list,
};
pub use parsers::character::{CharacterAnimation, CharacterDefinition};
pub use parsers::chart::{Chart, ChartNote, ChartSection, ParsedSong};
pub use parsers::png::PngImage;
pub use parsers::sparrow::{SparrowAtlas, SparrowFrame};
pub use parsers::stage::{
    stage_id_for_song_name, StageCharacterSlot, StageDefinition, StageObject,
};
pub use parsers::text_list::TextList;
pub use parsers::types::AssetVec2;
pub use path::AssetPath;
pub use resolver::{AssetResolver, OverlayResolver};
pub use source::AssetSource;
pub use watcher::Watcher;
