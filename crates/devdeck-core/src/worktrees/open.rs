//! Open a worktree in VS Code or a terminal, without a shell string. Builders are pure (unit-tested);
//! only `open_*` start processes.

use crate::error::{Error, ErrorKind, Result};
use std::path::Path;
use std::process::Command;

/// A program plus its arguments (never a shell string).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenCommand {
    pub program: String,
    pub args: Vec<String>,
    /// Needs its own console window (console programs launched from a GUI app).
    pub new_console: bool,
}

fn cmd(program: &str, args: &[&str], path: &Path, new_console: bool) -> OpenCommand {
    let mut a: Vec<String> = args.iter().map(|s| (*s).to_owned()).collect();
    a.push(path.to_string_lossy().into_owned());
    OpenCommand {
        program: program.to_owned(),
        args: a,
        new_console,
    }
}

/// `code <path>` (`code.cmd` on Windows, where `code` is a batch shim).
pub fn vscode_command(path: &Path) -> OpenCommand {
    let program = if cfg!(windows) { "code.cmd" } else { "code" };
    cmd(program, &[], path, false)
}

/// Terminal candidates in preference order: `wt -d <path>` → `pwsh -WorkingDirectory <path>` →
/// `cmd /K cd /d <path>` (Windows); common Linux/macOS terminals otherwise.
pub fn terminal_commands(path: &Path) -> Vec<OpenCommand> {
    if cfg!(windows) {
        vec![
            cmd("wt", &["-d"], path, false),
            cmd("pwsh", &["-NoExit", "-WorkingDirectory"], path, true),
            cmd("cmd", &["/K", "cd", "/d"], path, true),
        ]
    } else if cfg!(target_os = "macos") {
        vec![cmd("open", &["-a", "Terminal"], path, false)]
    } else {
        vec![
            cmd("x-terminal-emulator", &["--working-directory"], path, false),
            cmd("gnome-terminal", &["--working-directory"], path, false),
        ]
    }
}

fn validate(path: &Path) -> Result<()> {
    if !path.is_absolute() {
        return Err(Error::not_allowed("not_allowed"));
    }
    if !path.is_dir() {
        return Err(Error::not_found("not_found"));
    }
    Ok(())
}

fn spawn(c: &OpenCommand) -> std::io::Result<()> {
    let mut p = Command::new(&c.program);
    p.args(&c.args);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // CREATE_NEW_CONSOLE for console programs, otherwise CREATE_NO_WINDOW (GUI launchers).
        p.creation_flags(if c.new_console { 0x10 } else { 0x0800_0000 });
    }
    p.spawn().map(|_| ())
}

pub fn open_vscode(path: &Path) -> Result<()> {
    validate(path)?;
    spawn(&vscode_command(path))
        .map_err(|e| Error::new(ErrorKind::NotFound, "vscode_not_found").with_detail(e.to_string()))
}

/// Tries each terminal candidate until one starts.
pub fn open_terminal(path: &Path) -> Result<()> {
    validate(path)?;
    let mut last = None;
    for c in terminal_commands(path) {
        match spawn(&c) {
            Ok(()) => return Ok(()),
            Err(e) => last = Some(e),
        }
    }
    Err(Error::new(ErrorKind::NotFound, "terminal_not_found")
        .with_detail(last.map(|e| e.to_string()).unwrap_or_default()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vscode_command_has_path_as_single_argument() {
        let c = vscode_command(Path::new("/a b/c"));
        assert!(c.program == "code" || c.program == "code.cmd");
        assert_eq!(c.args, vec!["/a b/c".to_string()]);
    }

    #[cfg(windows)]
    #[test]
    fn terminal_order_is_wt_then_pwsh_then_cmd() {
        let cs = terminal_commands(Path::new(r"C:\my repo"));
        let progs: Vec<_> = cs.iter().map(|c| c.program.as_str()).collect();
        assert_eq!(progs, ["wt", "pwsh", "cmd"]);
        assert_eq!(cs[0].args, ["-d", r"C:\my repo"]);
        assert_eq!(cs[1].args, ["-NoExit", "-WorkingDirectory", r"C:\my repo"]);
        assert_eq!(cs[2].args, ["/K", "cd", "/d", r"C:\my repo"]);
        assert!(!cs[0].new_console && cs[1].new_console && cs[2].new_console);
    }

    #[test]
    fn terminal_commands_never_embed_a_shell_string() {
        for c in terminal_commands(Path::new("/x y")) {
            assert_eq!(c.args.last().unwrap(), "/x y");
        }
    }

    #[test]
    fn relative_and_missing_paths_are_rejected_before_spawning() {
        assert_eq!(
            open_vscode(Path::new("relative")).unwrap_err().kind,
            ErrorKind::NotAllowed
        );
        let missing = std::env::temp_dir().join("devdeck-no-such-dir-xyz");
        assert_eq!(
            open_terminal(&missing).unwrap_err().kind,
            ErrorKind::NotFound
        );
    }
}
