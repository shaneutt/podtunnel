use std::{
    fs::{File, OpenOptions},
    io::{BufWriter, Write},
    process::{Command, Stdio},
    sync::{LazyLock, Mutex},
};

use anyhow::{Context, anyhow};

const LOG_FILE: &str = "/tmp/podtunnel.log";

pub struct FileLogger {
    pub writer: Mutex<BufWriter<File>>,
}

pub static LOGGER: LazyLock<FileLogger> = LazyLock::new(|| {
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(LOG_FILE)
        .expect("could not open logfile");
    FileLogger {
        writer: Mutex::new(BufWriter::new(file)),
    }
});

#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {{
        use $crate::system::linux::LOGGER;

        use std::io::Write;
        use std::time::{SystemTime, UNIX_EPOCH};

        let mut buf = LOGGER.writer.lock().expect("could not lock log file");
        let now = SystemTime::now();
        let timestamp = now.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
        let log_line = format!("{}: {}\n", timestamp, format!($($arg)*));

        buf.write_all(log_line.as_bytes()).expect("could not write to log file");
        buf.flush().expect("could not flush logger buffer");
    }};
}

pub fn run<S>(program: S, args: impl IntoIterator<Item = S>) -> anyhow::Result<String>
where
    S: AsRef<str>,
{
    let mut command = Command::new(program.as_ref());
    for arg in args {
        command.arg(arg.as_ref());
    }

    let output = command.output().context("command failed")?;
    if !output.status.success() {
        return Err(anyhow!(
            "failed to get output for program {} stderr: {}",
            program.as_ref(),
            String::from_utf8_lossy(&output.stderr),
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn run_with_stdin<S>(
    program: S,
    args: impl IntoIterator<Item = S>,
    stdin: S,
) -> anyhow::Result<String>
where
    S: AsRef<str>,
{
    let mut command = Command::new(program.as_ref());
    for arg in args {
        command.arg(arg.as_ref());
    }
    command.stdin(Stdio::piped());
    command.stdout(Stdio::piped());
    let mut spawned_command = command.spawn().context("command failed")?;

    {
        let spawned_command_stdin = spawned_command
            .stdin
            .as_mut()
            .context("failed to open stdin for program")?;
        spawned_command_stdin.write_all(stdin.as_ref().as_bytes())?;
    }

    let output = spawned_command
        .wait_with_output()
        .context("failed to run program")?;

    if !output.status.success() {
        return Err(anyhow!(
            "failed to get output for program {} stderr: {}",
            program.as_ref(),
            String::from_utf8_lossy(&output.stderr),
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
