pub mod auth;
pub mod common;
pub mod output;
pub mod storage;
pub mod target;
pub mod utils;

pub const VERSION: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    "-",
    env!("VERGEN_GIT_SHA"),
    " (",
    env!("VERGEN_BUILD_DATE"),
    ")"
);
