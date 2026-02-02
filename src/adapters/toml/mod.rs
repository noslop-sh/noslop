//! TOML-based assertion repository
//!
//! Implements `AssertionRepository` using `.noslop.toml` files.
//!
//! - [`parser`] - Read and deserialize .noslop.toml files
//! - [`writer`] - Create and modify .noslop.toml files
//! - [`repository`] - `AssertionRepository` implementation

pub mod parser;
pub mod repository;
pub mod writer;

pub use parser::{AssertionEntry, NoslopFile, ProjectConfig, find_noslop_files, load_file};
pub use repository::TomlAssertionRepository;
pub use writer::{add_assertion, format_noslop_file, generate_prefix_from_repo};
