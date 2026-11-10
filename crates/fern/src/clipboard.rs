use std::io::Write;
use std::process::{Command, Stdio};

type Tool = (&'static str, &'static [&'static str]);

#[cfg(target_os = "macos")]
const TOOLS: &[Tool] = &[("pbcopy", &[])];
#[cfg(windows)]
const TOOLS: &[Tool] = &[("clip", &[])];
#[cfg(not(any(target_os = "macos", windows)))]
const TOOLS: &[Tool] = &[
    ("wl-copy", &[]),
    ("xclip", &["-selection", "clipboard"]),
    ("xsel", &["--clipboard", "--input"]),
];

pub fn copy(text: &str) -> bool {
    TOOLS
        .iter()
        .any(|(program, args)| pipe(program, args, text))
}

fn pipe(program: &str, args: &[&str], text: &str) -> bool {
    let Ok(mut child) = Command::new(program)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    else {
        return false;
    };

    if let Some(mut stdin) = child.stdin.take()
        && stdin.write_all(text.as_bytes()).is_err()
    {
        return false;
    }

    child.wait().is_ok_and(|status| status.success())
}
