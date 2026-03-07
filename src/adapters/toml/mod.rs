//! TOML-based check repository
//!
//! Implements `CheckRepository` using `.noslop.toml` files.
//!
//! - [`parser`] - Read and deserialize .noslop.toml files
//! - [`writer`] - Create and modify .noslop.toml files
//! - [`repository`] - `CheckRepository` implementation

pub mod parser;
pub mod repository;
pub mod writer;

pub use parser::{
    AgentSection, CheckEntry, NoslopFile, ProjectConfig, ReviewSection, find_noslop_files,
    load_file,
};
pub use repository::TomlCheckRepository;
pub use writer::{add_check, format_noslop_file, generate_prefix_from_repo};
