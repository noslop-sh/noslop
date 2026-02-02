//! TOML-based assertion repository
//!
//! Implements `AssertionRepository` using `.noslop.toml` files.

mod parser;
mod repository;
mod writer;

pub use parser::{AssertionEntry, NoslopFile, ProjectConfig};
pub use repository::TomlAssertionRepository;
pub use writer::NoslopFileWriter;
