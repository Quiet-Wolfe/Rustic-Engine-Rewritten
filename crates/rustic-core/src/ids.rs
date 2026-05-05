//! Strong IDs and stable handles. See `PLAN.md` Section 5.

use serde::{Deserialize, Serialize};

macro_rules! id_type {
    ($(#[$meta:meta])* $name:ident, $inner:ty) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        pub struct $name(pub $inner);

        impl $name {
            #[inline]
            pub const fn new(raw: $inner) -> Self { Self(raw) }
            #[inline]
            pub const fn raw(self) -> $inner { self.0 }
        }

        impl From<$inner> for $name {
            #[inline]
            fn from(raw: $inner) -> Self { Self(raw) }
        }
    };
}

id_type!(
    /// Stable identifier for an asset handle from the resolver.
    AssetId,
    u64
);

id_type!(
    /// Stable identifier for a camera. See `PLAN.md` Section 7.
    CameraId,
    u32
);

id_type!(
    /// Identifier for a note instance within a chart.
    NoteId,
    u32
);

id_type!(
    /// Identifier for a loaded song.
    SongId,
    u32
);
