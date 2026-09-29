//! # cucco
//!
//! Library behind the `cucco` CLI, an interactive tool for creating
//! conventional commits with git hooks support.
//!
//! ## Feature flags
//!
//! | name       | description                                         | default |
//! |------------|-----------------------------------------------------|---------|
//! | `ast-grep` | Detect commit scopes from staged content with rules | ✔       |

pub mod answers;
pub mod commit;
pub mod config;
pub mod emoji;
mod multiline;
pub mod questions;
pub mod scope;
pub mod status;
