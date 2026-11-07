//! # Pinned legacy transport
//!
//! Explicit compatibility, isolated from normal TLS providers.
use std::{
    net::{Shutdown, TcpStream},
    sync::mpsc::{RecvTimeoutError, channel},
    thread,
    time::Duration,
};

use anyhow::{Result, anyhow, bail};
use native_tls::{Protocol, TlsConnector, TlsStream};
use sha2::{Digest, Sha256};

const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);

fn pin_bytes(pin: &str) -> Result<[u8; 32]> {
    if pin.len() != 64 || !pin.bytes().all(|b| b.is_ascii_hexdigit()) {
        bail!("Legacy TLS requires an explicit 64-digit SHA-256 leaf pin");
    }
    let mut bytes = [0u8; 32];
    for (out, pair) in bytes.iter_mut().zip(pin.as_bytes().chunks_exact(2)) {
        let digit = |b: u8| {
            if b.is_ascii_digit() {
                b - b'0'
            } else {
                b.to_ascii_lowercase() - b'a' + 10
            }
        };
        *out = digit(pair[0]) * 16 + digit(pair[1]);
    }
    Ok(bytes)
}

pub(crate) fn connect(
    host: &str,
    tcp: TcpStream,
    expected_host: &str,
    pin: &str,
) -> Result<TlsStream<TcpStream>> {
    if expected_host.is_empty() || !host.eq_ignore_ascii_case(expected_host) {
        bail!("Legacy TLS profile is not valid for this hostname");
    }
    let expected = pin_bytes(pin)?;
    let read_timeout = tcp.read_timeout()?;
    let write_timeout = tcp.write_timeout()?;
    let bounded = |timeout: Option<Duration>| {
        Some(timeout.unwrap_or(HANDSHAKE_TIMEOUT).min(HANDSHAKE_TIMEOUT))
    };
    tcp.set_read_timeout(bounded(read_timeout))?;
    tcp.set_write_timeout(bounded(write_timeout))?;
    let connector = TlsConnector::builder()
        .min_protocol_version(Some(Protocol::Tlsv10))
        .max_protocol_version(Some(Protocol::Tlsv10))
        // NOTE: normal chain/hostname checks are replaced by the mandatory
        // the same connection's mandatory exact leaf pin is checked below.
        .danger_accept_invalid_certs(true)
        .danger_accept_invalid_hostnames(true)
        .build()?;
    let deadline_socket = tcp.try_clone()?;
    let (cancel, cancelled) = channel::<()>();
    // NOTE: a socket timeout alone resets on each read and permits trickling.
    // The owned watchdog is cancelled and joined before handing back a stream.
    let watchdog = thread::Builder::new()
        .name("legacy-tls-deadline".into())
        .spawn(move || {
            if matches!(
                cancelled.recv_timeout(HANDSHAKE_TIMEOUT),
                Err(RecvTimeoutError::Timeout)
            ) {
                let _ = deadline_socket.shutdown(Shutdown::Both);
                true
            } else {
                false
            }
        })?;
    let result = (|| -> Result<_> {
        let stream = connector
            .connect(host, tcp)
            .map_err(|_| anyhow!("Pinned legacy TLS handshake failed before authentication"))?;
        let leaf = stream
            .peer_certificate()?
            .ok_or_else(|| anyhow!("Legacy TLS peer did not supply a leaf certificate"))?
            .to_der()?;
        let actual: [u8; 32] = Sha256::digest(&leaf).into();
        if actual != expected {
            bail!(
                "Legacy TLS leaf pin mismatch; authentication was not sent; explicitly reverify the server certificate"
            );
        }
        Ok(stream)
    })();
    drop(cancel);
    if watchdog
        .join()
        .map_err(|_| anyhow!("Legacy TLS deadline monitor failed before authentication"))?
    {
        bail!("Pinned legacy TLS handshake failed before authentication: total deadline expired");
    }
    let stream = result?;
    stream.get_ref().set_read_timeout(read_timeout)?;
    stream.get_ref().set_write_timeout(write_timeout)?;
    Ok(stream)
}

#[cfg(test)]
mod tests {
    use crate::legacy::pin_bytes;
    #[test]
    fn strict_pin_encoding() {
        assert_eq!(pin_bytes(&"Ab".repeat(32)).unwrap(), [0xab; 32]);
        for value in ["", "00", &"g".repeat(64), &format!("{}\n", "a".repeat(63))] {
            assert!(pin_bytes(value).is_err());
        }
    }
}
