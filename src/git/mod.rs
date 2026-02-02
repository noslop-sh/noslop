//! Git integration
//!
//! Provides git-native operations:
//! - Hooks installation
//! - Staged file detection
//! - Repository information
//!
//! This module re-exports from `noslop::adapters::git` for backwards compatibility.

// Re-export from adapters (some may be unused but kept for backwards compatibility)
#[allow(unused_imports)]
pub use noslop::adapters::git::{
    GitVersionControl, get_repo_name, get_staged_files, install_commit_msg, install_post_commit,
    install_pre_commit,
};

// Keep hooks and staged as submodules for backwards compatibility
pub mod hooks {
    //! Git hooks re-exports
    pub use noslop::adapters::git::hooks::{
        install_commit_msg, install_post_commit, install_pre_commit,
    };
}

pub mod staged {
    //! Staged files re-exports
    pub use noslop::adapters::git::staging::get_staged_files;
}
