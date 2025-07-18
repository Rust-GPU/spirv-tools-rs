use crate::error::Message;
use std::process::{Command, Stdio};

pub enum CmdError {
    /// The binary failed to spawn, probably because it's not installed
    /// or not in PATH
    BinaryNotFound(std::io::Error),
    /// An I/O error occurred accessing the process' pipes
    Io(std::io::Error),
    /// The binary ran, but returned a non-zero exit code and (hopefully)
    /// diagnostics
    ToolErrors {
        exit_code: i32,
        /// Messages that were parsed from the output
        messages: Vec<Message>,
    },
}

impl From<CmdError> for crate::error::Error {
    fn from(ce: CmdError) -> Self {
        use crate::SpirvResult;

        match ce {
            CmdError::BinaryNotFound(err) => Self {
                inner: SpirvResult::Unsupported,
                diagnostic: Some(format!("failed to spawn executable: {err}").into()),
            },
            CmdError::Io(err) => Self {
                inner: SpirvResult::EndOfStream,
                diagnostic: Some(
                    format!("i/o error occurred communicating with executable: {err}").into(),
                ),
            },
            CmdError::ToolErrors {
                exit_code,
                messages,
            } => {
                // The C API just puts the last message as the diagnostic, so just do the
                // same for now
                let diagnostic = messages.into_iter().next_back().map_or_else(
                    || {
                        crate::error::Diagnostic::from(format!(
                            "tool exited with code `{exit_code}` and no output"
                        ))
                    },
                    crate::error::Diagnostic::from,
                );

                Self {
                    // this isn't really correct, but the spirv binaries don't
                    // provide the error code in any meaningful way, either by the
                    // status code of the binary, or in diagnostic output
                    inner: SpirvResult::InternalError,
                    diagnostic: Some(diagnostic),
                }
            }
        }
    }
}

pub struct CmdOutput {
    /// The output the command is actually supposed to give back
    pub binary: Vec<u8>,
    /// Warning or Info level diagnostics that were gathered during execution
    pub messages: Vec<Message>,
}

#[derive(PartialEq, Eq, Copy, Clone)]
pub enum Output {
    /// Doesn't try to read stdout for tool output (other than diagnostics)
    Ignore,
    /// Attempts to retrieve the tool's output from stdout
    Retrieve,
}

pub fn exec(
    mut cmd: Command,
    input: Option<&[u8]>,
    retrieve_output: Output,
) -> Result<CmdOutput, CmdError> {
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

    // Create a temp dir for the input and/or output of the tool
    let temp_dir = tempfile::tempdir().map_err(CmdError::Io)?;

    // Output
    let output_path = temp_dir.path().join("output");
    if retrieve_output == Output::Retrieve {
        cmd.arg("-o").arg(&output_path);
    }

    // Input
    if let Some(input) = input {
        let input_path = temp_dir.path().join("input");
        std::fs::write(&input_path, input).map_err(CmdError::Io)?;

        cmd.arg(&input_path);
    }

    let child = cmd.spawn().map_err(CmdError::BinaryNotFound)?;

    let output = child.wait_with_output().map_err(CmdError::Io)?;

    let code = if let Some(code) = output.status.code() {
        code
    } else {
        #[cfg(unix)]
        let message = {
            use std::os::unix::process::ExitStatusExt;
            format!(
                "process terminated by signal: {}",
                output.status.signal().unwrap_or(666)
            )
        };
        #[cfg(not(unix))]
        let message = "process ended in an unknown state".to_owned();

        return Err(CmdError::ToolErrors {
            exit_code: -1,
            messages: vec![Message::fatal(message)],
        });
    };

    // stderr should only ever contain error+ level diagnostics
    if code != 0 {
        use crate::error::*;
        let messages: Vec<_> = match String::from_utf8(output.stderr) {
            Ok(errors) => {
                let mut messages = Vec::new();

                for line in errors.lines() {
                    if let Some(msg) = Message::parse(line) {
                        messages.push(msg);
                    } else if let Some(msg) = messages.last_mut() {
                        if !msg.notes.is_empty() {
                            msg.notes.push('\n');
                        }

                        msg.notes.push_str(line);
                    } else {
                        // We somewhow got a message that didn't conform to how
                        // messages are supposed to look, as the first one
                        messages.push(Message {
                            level: MessageLevel::Error,
                            source: None,
                            line: 0,
                            column: 0,
                            index: 0,
                            message: line.to_owned(),
                            notes: String::new(),
                        });
                    }
                }

                messages
            }
            Err(err) => vec![Message::fatal(format!(
                "unable to read stderr ({err}) but process exited with code {code}",
            ))],
        };

        return Err(CmdError::ToolErrors {
            exit_code: code,
            messages,
        });
    }

    fn split(haystack: &[u8], needle: u8) -> impl Iterator<Item = &[u8]> {
        struct Split<'a> {
            haystack: &'a [u8],
            needle: u8,
        }

        impl<'a> Iterator for Split<'a> {
            type Item = &'a [u8];

            fn next(&mut self) -> Option<&'a [u8]> {
                if self.haystack.is_empty() {
                    return None;
                }
                let (ret, remaining) = match memchr::memchr(self.needle, self.haystack) {
                    Some(pos) => (
                        self.haystack.get(..pos).unwrap(),
                        self.haystack.get(pos + 1..).unwrap(),
                    ),
                    None => (self.haystack, &[][..]),
                };
                self.haystack = remaining;
                Some(ret)
            }
        }

        Split { haystack, needle }
    }

    let binary = match retrieve_output {
        Output::Retrieve => std::fs::read(&output_path).map_err(CmdError::Io)?,
        Output::Ignore => Vec::new(),
    };

    // Since we are retrieving the results via stdout, but it can also contain
    // diagnostic messages, we need to be careful
    let mut messages = Vec::new();

    for line in split(&output.stdout, b'\n') {
        if let Ok(s) = std::str::from_utf8(line) {
            if let Some(msg) = crate::error::Message::parse(s) {
                messages.push(msg);
                continue;
            }
        }

        break;
    }

    Ok(CmdOutput { binary, messages })
}
