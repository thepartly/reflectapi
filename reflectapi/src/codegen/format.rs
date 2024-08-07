use std::{
    io::{self, Write},
    process::{Command, Stdio},
};

/// Format the given source code using the given command.
/// Input will be piped to the command's stdin and the output will be collected from stdout.
pub fn format_with(cmd: &mut Command, src: String) -> io::Result<String> {
    let mut child = match cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            // Don't bother formatting if the command is not found, log an error and return the input.
            eprintln!("skipping formatting as command is not found {cmd:?}");
            return Ok(src);
        }
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

    Ok(String::from_utf8(output.stdout).expect("utf-8 output from formatter"))
}
