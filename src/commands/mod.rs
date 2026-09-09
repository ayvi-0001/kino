crate::mod_flat!(help, register, version, watchlist);

pub use self::{help::*, register::*, version::*, watchlist::*};

#[cfg(feature = "tmdb")]
crate::mod_flat!(movies);

#[cfg(feature = "tmdb")]
pub use self::movies::*;
