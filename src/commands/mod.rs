//! Command implementations

mod add_trailers;
mod agent;
mod check;
mod check_cmd;
mod clear_staged;
mod init;
mod status;
mod task;
mod task_prompt;
#[cfg(feature = "ui")]
mod ui;
mod verify;

pub use add_trailers::add_trailers;
pub use agent::agent_cmd;
pub use check::check_run;
pub use check_cmd::check_cmd;
pub use clear_staged::clear_staged;
pub use init::init;
pub use status::status;
pub use task::task_cmd;
pub use task_prompt::task_prompt;
#[cfg(feature = "ui")]
pub use ui::ui;
pub use verify::verify;
