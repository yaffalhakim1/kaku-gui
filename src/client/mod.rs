mod types;

use anyhow::{bail, Context as _, Result};
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::process::{ChildStdin, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

pub use types::*;

#[derive(Clone)]
pub struct CodexClient {
    stdin: Arc<Mutex<ChildStdin>>,
    next_id: Arc<AtomicU64>,
}

impl CodexClient {
    pub fn spawn() -> Result<(Self, BufReader<ChildStdout>)> {
        let mut child = Command::new("codex")
            .arg("app-server")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .context("spawn codex app-server (is `codex` on PATH?)")?;

        let mut stdin = child.stdin.take().expect("stdin piped");
        let mut stdout = BufReader::new(child.stdout.take().expect("stdout piped"));
        drop(child); // the OS keeps the process running; we keep the pipes

        // -- initialize: the server rejects everything before this --
        write_line(
            &mut stdin,
            &json!({
                "method": "initialize",
                "id": 0,
                "params": {
                    "clientInfo": {
                        "name": "kaku-gui",
                        "title": "Kaku GUI",
                        "version": "0.1.0"
                    }
                }
            }),
        )?;

        loop {
            let msg = read_msg(&mut stdout)?.context("codex closed stdout during initialize")?;
            if msg.get("id") == Some(&json!(0)) {
                if let Some(err) = msg.get("error") {
                    bail!("initialize failed: {err}");
                }
                break;
            }
            // anything else is a stray notification; ignore it
        }

        write_line(
            &mut stdin,
            &json!({ "method": "initialized", "params": {} }),
        )?;

        Ok((
            Self {
                stdin: Arc::new(Mutex::new(stdin)),
                next_id: Arc::new(AtomicU64::new(1)),
            },
            stdout,
        ))
    }

    pub fn request(&self, method: &str, params: Value) -> Result<u64> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let msg = json!({ "method": method, "id": id, "params": params });
        let mut stdin = self.stdin.lock().expect("stdin poisoned");
        write_line(&mut stdin, &msg)?;
        Ok(id)
    }
}

fn write_line(stdin: &mut ChildStdin, value: &Value) -> Result<()> {
    let line = serde_json::to_string(value)?;
    writeln!(stdin, "{line}").context("write to codex stdin")?;
    Ok(())
}

pub fn read_msg(reader: &mut BufReader<ChildStdout>) -> Result<Option<Value>> {
    let mut line = String::new();
    let n = reader
        .read_line(&mut line)
        .context("read from codex stdout")?;
    if n == 0 {
        return Ok(None); // EOF: the child exited
    }
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return read_msg(reader); // skip blank lines
    }
    Ok(serde_json::from_str(trimmed).context("bad JSON from codex")?)
}
