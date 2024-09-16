use std::{
    io::{self, Write},
    process::{Command, Stdio},
};

/// Format the given source code using the first of the given commands that exists.
/// Input will be piped to the command's stdin and the output will be collected from stdout.
pub fn format_with<'a>(
    cmds: impl IntoIterator<Item = &'a mut Command>,
    src: String,
) -> io::Result<String> {
    for cmd in cmds {
        let mut child = match cmd
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(child) => child,
            Err(err) if err.kind() == io::ErrorKind::NotFound => continue,
            Err(err) => return Err(err),
        };

        let stdin = child.stdin.as_mut().unwrap();

        stdin.write_all(src.as_bytes())?;
        let output = child.wait_with_output()?;
        if !output.status.success() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!(
                    "command failed with exit code {:?}\nstderr:\n{}",
                    output.status.code(),
                    String::from_utf8_lossy(&output.stderr)
                ),
            ));
        }

        return Ok(String::from_utf8(output.stdout).expect("utf-8 output from formatter"));
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "no formatters found",
    ))
}
