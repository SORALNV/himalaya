//! # Bounded provider checks
//!
//! Authenticates fixed presets in a subprocess with no inherited proxies,
//! debug logging, secret arguments or configuration files.

use std::{
    env,
    io::{Read, Write, stdin},
    path::PathBuf,
    process::{Child, Command, Stdio, exit},
    thread,
    time::{Duration, Instant},
};

use anyhow::{Result, bail};
use pimalaya_config::secret::{Secret, SecretResolver};
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use crate::{
    account::check::{connect_imap, connect_smtp},
    wizard::web::draft::{Credential, Draft, Provider},
};

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct CheckInput {
    provider: Provider,
    name: String,
    email: String,
    imap_login: String,
    password: Zeroizing<String>,
}

#[derive(Serialize)]
struct CheckInputRef<'a> {
    provider: Provider,
    name: &'a str,
    email: &'a str,
    imap_login: &'a str,
    password: &'a str,
}

/// Checks both protocols, killing the isolated process after thirty seconds.
pub fn check(draft: &Draft, password: &str, cancelled: &dyn Fn() -> bool) -> Result<()> {
    let payload = Zeroizing::new(serde_json::to_vec(&CheckInputRef {
        provider: draft.provider,
        name: &draft.name,
        email: &draft.email,
        imap_login: &draft.imap_login,
        password,
    })?);
    let (code, _) = call(
        "--web-check-worker",
        &payload,
        Duration::from_secs(30),
        cancelled,
    )?;
    match code {
        Some(0) => Ok(()),
        Some(40) => Err(anyhow::anyhow!(
            "IMAP authentication failed; check the address, app password and provider policy"
        )),
        Some(41) => Err(anyhow::anyhow!(
            "SMTP authentication failed; check the address, app password and provider policy"
        )),
        _ => Err(anyhow::anyhow!("Provider check failed")),
    }
}

/// Runs a fixed helper with bounded private IPC and an enforceable deadline.
pub fn call(
    flag: &str,
    payload: &[u8],
    limit: Duration,
    cancelled: &dyn Fn() -> bool,
) -> Result<(Option<i32>, Zeroizing<Vec<u8>>)> {
    if cancelled() {
        bail!("Setup session ended");
    }
    if payload.len() > 32768 {
        bail!("Worker input exceeds limit");
    }
    let child = Command::new(executable()?)
        .args(["--log-level", "off", "configure", flag])
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| anyhow::anyhow!("Cannot start provider check"))?;
    exchange(child, payload, limit, cancelled)
}

fn exchange(
    mut child: Child,
    payload: &[u8],
    limit: Duration,
    cancelled: &dyn Fn() -> bool,
) -> Result<(Option<i32>, Zeroizing<Vec<u8>>)> {
    use std::{io::ErrorKind, os::fd::AsRawFd, process::ExitStatus};
    let result = (|| {
        let mut input = child.stdin.take();
        let mut output = child
            .stdout
            .take()
            .ok_or_else(|| anyhow::anyhow!("Worker output unavailable"))?;
        let mut descriptors = vec![output.as_raw_fd()];
        if let Some(input) = &input {
            descriptors.push(input.as_raw_fd());
        }
        for fd in descriptors {
            // SAFETY: these are live owned pipe descriptors.
            let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
            if flags < 0 || unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0
            {
                bail!("Cannot initialize worker pipes");
            }
        }
        let deadline = Instant::now() + limit;
        let mut offset = 0;
        let mut bytes = Zeroizing::new(Vec::new());
        let mut chunk = Zeroizing::new([0u8; 2048]);
        let mut status: Option<ExitStatus> = None;
        let mut eof = false;
        loop {
            if cancelled() {
                bail!("Setup session ended");
            }
            if Instant::now() >= deadline {
                bail!("Provider check timed out or failed");
            }
            if let Some(pipe) = &mut input {
                match pipe.write(&payload[offset..]) {
                    Ok(n) => offset += n,
                    Err(e) if e.kind() == ErrorKind::WouldBlock => {}
                    Err(_) => bail!("Cannot submit provider check"),
                }
                if offset == payload.len() {
                    input = None;
                }
            }
            loop {
                match output.read(chunk.as_mut()) {
                    Ok(0) => {
                        eof = true;
                        break;
                    }
                    Ok(n) => {
                        if bytes.len() + n > 32768 {
                            bail!("Worker output exceeds limit");
                        }
                        bytes.extend_from_slice(&chunk[..n]);
                    }
                    Err(e) if e.kind() == ErrorKind::WouldBlock => break,
                    Err(_) => bail!("Worker output failed"),
                }
            }
            if status.is_none() {
                status = child.try_wait()?;
            }
            if let Some(status) = status
                && eof
            {
                return Ok((status.code(), bytes));
            }
            thread::sleep(Duration::from_millis(20));
        }
    })();
    let _ = child.kill();
    let _ = child.wait();
    result
}

#[cfg(not(test))]
fn executable() -> Result<PathBuf> {
    Ok(env::current_exe()?)
}

#[cfg(test)]
fn executable() -> Result<PathBuf> {
    env::var_os("HIMALAYA_WEB_TEST_BINARY")
        .map(Into::into)
        .ok_or_else(|| {
            anyhow::anyhow!("Set HIMALAYA_WEB_TEST_BINARY for explicit native worker tests")
        })
}

/// Runs only the fixed provider draft received through bounded standard input.
pub fn execute() -> Result<()> {
    let mut bytes = Zeroizing::new(Vec::new());
    stdin().take(8193).read_to_end(&mut bytes)?;
    if bytes.len() > 8192 {
        bail!("Provider check input exceeds limit");
    }
    let input: CheckInput = serde_json::from_slice(&bytes)
        .map_err(|_| anyhow::anyhow!("Invalid provider check input"))?;
    let mut draft = Draft {
        provider: input.provider,
        name: input.name,
        email: input.email,
        imap_login: input.imap_login,
        credential: Credential::Existing {
            entry: "worker".into(),
        },
    };
    draft.validate()?;
    if input.password.len() != 16 || !input.password.bytes().all(|b| b.is_ascii_alphanumeric()) {
        bail!("Invalid provider password");
    }
    let account = draft.account_with_secret(Secret::Raw(input.password.to_string().into()));
    let mut resolver = SecretResolver::new();
    if connect_imap(
        account.imap.as_ref().expect("fixed IMAP preset"),
        &mut resolver,
    )
    .is_err()
    {
        exit(40);
    }
    if connect_smtp(
        account.smtp.as_ref().expect("fixed SMTP preset"),
        &mut resolver,
    )
    .is_err()
    {
        exit(41);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::wizard::web::worker::exchange;
    use std::{
        process::{Command, Stdio},
        time::{Duration, Instant},
    };

    #[test]
    fn stalled_worker_is_killed_and_reaped_on_deadline_or_revocation() {
        for cancelled in [false, true] {
            let child = Command::new("/bin/sleep")
                .arg("5")
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
                .unwrap();
            let started = Instant::now();
            let deadline = if cancelled {
                Duration::from_secs(5)
            } else {
                Duration::from_millis(30)
            };
            let result = exchange(child, b"", deadline, &|| {
                cancelled && started.elapsed() > Duration::from_millis(30)
            });
            assert!(result.is_err());
            assert!(started.elapsed() < Duration::from_secs(2));
        }
    }
}
