use std::{
    io::{self, Write},
    process::{Command, Stdio},
};

/// Format the given source code using the first of the given commands that exists.
/// Input will be piped to the command's stdin and the output will be collected from stdout.
///
/// When none can be started, because none is installed or because the target
/// (such as wasm) can't start processes, the error is `NotFound`.
pub fn format_with<'a>(
    cmds: impl IntoIterator<Item = &'a mut Command>,
    src: String,
) -> io::Result<String> {
    let mut unsupported = false;
    for cmd in cmds {
        let mut child = match cmd
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(child) => child,
            Err(err) if err.kind() == io::ErrorKind::NotFound => continue,
            Err(err) if err.kind() == io::ErrorKind::Unsupported => {
                unsupported = true;
                continue;
            }
            Err(err) => return Err(err),
        };

        let stdin = child.stdin.as_mut().unwrap();

        // A formatter that rejects its input can exit before draining stdin,
        // which turns this write into a `BrokenPipe` error. Swallow only that
        // case so the real failure (exit status + stderr) is reported by
        // `wait_with_output` below, instead of being masked by "broken pipe".
        if let Err(err) = stdin.write_all(src.as_bytes()) {
            if err.kind() != io::ErrorKind::BrokenPipe {
                return Err(err);
            }
        }
        let output = child.wait_with_output()?;
        if !output.status.success() {
            return Err(io::Error::other(format!(
                "command failed with exit code {:?}\nstderr:\n{}",
                output.status.code(),
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        return Ok(String::from_utf8(output.stdout).expect("utf-8 output from formatter"));
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        if unsupported {
            "no formatter can run: this target can't start processes; \
             generate without formatting and format the files in the host"
        } else {
            "no formatters found"
        },
    ))
}
