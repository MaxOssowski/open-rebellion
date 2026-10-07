//! Developer commands for play-testing: the command palette's gate and the
//! native command script.
//!
//! The palette (backtick) is on in debug builds, and in release builds when
//! `OPEN_REBELLION_DEV` is set. A native run with `OPEN_REBELLION_COMMANDS`
//! naming a file runs that file's lines as palette commands, one per frame,
//! so an acceptance run reaches its state without the clicks that lead to
//! it, and `OPEN_REBELLION_SEED` fixes every new campaign's seed so the
//! script meets the same galaxy each run. None is compiled into a release
//! browser build.

use std::collections::VecDeque;

/// True when the palette answers the backtick key.
#[must_use]
pub fn palette_enabled() -> bool {
    palette_switch(std::env::var("OPEN_REBELLION_DEV").ok().as_deref())
}

/// Whether the palette is on for an `OPEN_REBELLION_DEV` value: always in
/// a debug build, and in a native release build when the value is on.
fn palette_switch(value: Option<&str>) -> bool {
    #[cfg(not(target_arch = "wasm32"))]
    if value.is_some_and(crate::env_flag_on) {
        return true;
    }
    #[cfg(target_arch = "wasm32")]
    let _ = value;
    cfg!(debug_assertions)
}

/// The campaign seed `OPEN_REBELLION_SEED` fixes, if it is set.
#[cfg(not(target_arch = "wasm32"))]
#[must_use]
pub fn campaign_seed() -> Option<u64> {
    seed_value(std::env::var("OPEN_REBELLION_SEED").ok().as_deref())
}

/// An `OPEN_REBELLION_SEED` value as a seed: a whole number, spaces around
/// it ignored.
fn seed_value(value: Option<&str>) -> Option<u64> {
    value?.trim().parse().ok()
}

/// The lines of a command script still to run.
#[derive(Debug, Default)]
pub struct CommandScript {
    lines: VecDeque<String>,
}

impl CommandScript {
    /// Each non-blank line of `text` that is not a `#` comment, trimmed: a
    /// palette command's label.
    #[must_use]
    pub fn parse(text: &str) -> Self {
        Self {
            lines: text
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty() && !line.starts_with('#'))
                .map(str::to_string)
                .collect(),
        }
    }

    /// The script `OPEN_REBELLION_COMMANDS` names, if it is set and reads.
    #[cfg(not(target_arch = "wasm32"))]
    #[must_use]
    pub fn from_env() -> Option<Self> {
        Self::from_file(std::path::Path::new(&std::env::var_os(
            "OPEN_REBELLION_COMMANDS",
        )?))
    }

    /// The script in `path`, or `None`, logged, when it does not read.
    #[cfg(not(target_arch = "wasm32"))]
    fn from_file(path: &std::path::Path) -> Option<Self> {
        match std::fs::read_to_string(path) {
            Ok(text) => Some(Self::parse(&text)),
            Err(error) => {
                eprintln!("[dev-command] cannot read {}: {error}", path.display());
                None
            }
        }
    }

    /// The next line to run, removed from the script.
    pub fn next_line(&mut self) -> Option<String> {
        self.lines.pop_front()
    }

    /// True once every line has run.
    #[must_use]
    pub fn is_done(&self) -> bool {
        self.lines.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_palette_is_on_in_debug_builds_or_when_the_dev_switch_is_on() {
        for on in ["1", "true", "Yes", " on "] {
            assert!(palette_switch(Some(on)), "{on:?}");
        }
        for off in [None, Some(""), Some("0"), Some("off")] {
            assert_eq!(palette_switch(off), cfg!(debug_assertions), "{off:?}");
        }
    }

    #[test]
    fn the_seed_switch_takes_a_whole_number() {
        assert_eq!(seed_value(Some(" 42\n")), Some(42));
        assert_eq!(seed_value(Some("seed")), None);
        assert_eq!(seed_value(Some("-1")), None);
        assert_eq!(seed_value(None), None);
    }

    #[test]
    fn a_script_file_reads_into_its_lines_and_a_missing_one_into_none() {
        let path = std::env::temp_dir().join(format!(
            "open-rebellion-dev-commands-{}.txt",
            std::process::id()
        ));
        std::fs::write(&path, "# setup\nStart game: Empire\n").unwrap();
        let mut script = CommandScript::from_file(&path).unwrap();
        std::fs::remove_file(&path).unwrap();
        assert_eq!(script.next_line().as_deref(), Some("Start game: Empire"));
        assert!(script.is_done());
        assert!(CommandScript::from_file(&path).is_none());
    }

    #[test]
    fn a_script_runs_its_command_lines_in_order_without_comments_or_blanks() {
        let mut script = CommandScript::parse(
            "# reach Sullust's fleet\n\n  Start game: Alliance  \nOpen Fleet window: Sullust\n",
        );
        assert_eq!(script.next_line().as_deref(), Some("Start game: Alliance"));
        assert!(!script.is_done());
        assert_eq!(
            script.next_line().as_deref(),
            Some("Open Fleet window: Sullust")
        );
        assert!(script.is_done());
        assert_eq!(script.next_line(), None);
    }
}
