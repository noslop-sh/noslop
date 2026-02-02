//! Git integration adapter
//!
//! Implements `VersionControl` trait using git commands and libgit2.

mod hooks;
mod staging;
mod vcs;

pub use hooks::{install_hooks, HookInstaller};
pub use staging::get_staged_files;
pub use vcs::GitVersionControl;
