//! Fork-only regression fixtures. No real mail, credentials, or remote hosts.
//! All listeners bind 127.0.0.1:0; Proxy::None prevents ambient proxy use.
//! Generated identities live in a private temporary directory. On macOS,
//! native-tls imports PKCS#12 into its own temporary keychain, not login.keychain.
//!
//! Registered explicitly because the vendored package has autotests=false.
#![cfg(all(target_os = "macos", feature = "legacy-pinned"))]

use std::{
    fs,
    io::{self, Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    path::Path,
    process::Command,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use native_tls::{Identity, Protocol, TlsAcceptor};
use pimalaya_stream::{
    proxy::Proxy,
    retry::Retry,
    stream::{Stream, TcpConnectOptions},
    tls::{Tls, TlsProvider},
};
use sha2::{Digest, Sha256};
use tempfile::TempDir;

const HOST: &str = "127.0.0.1";
// Deliberately synthetic application bytes, never a real authentication token.
const APPLICATION: &[u8] = b"fixture-auth-sentinel\r\n";
const REPLY: &[u8] = b"fixture-ok\r\n";
const IO_TIMEOUT: Duration = Duration::from_secs(4);

fn openssl(dir: &Path, args: &[&str]) {
    let output = Command::new("/usr/bin/openssl")
        .current_dir(dir)
        .args(args)
        .output()
        .expect("run macOS LibreSSL for disposable fixture identity");
    assert!(
        output.status.success(),
        "fixture openssl failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

struct Certificate {
    identity: Identity,
    pin: String,
    _directory: TempDir,
}

impl Certificate {
    fn generate() -> Self {
        let directory = tempfile::Builder::new()
            .prefix("himalaya-legacy-fixture-")
            .tempdir()
            .unwrap();
        openssl(
            directory.path(),
            &[
                "req",
                "-x509",
                "-newkey",
                "rsa:2048",
                "-nodes",
                "-sha256",
                "-days",
                "1",
                "-subj",
                "/CN=deliberately-not-the-loopback-host.invalid",
                "-keyout",
                "key.pem",
                "-out",
                "cert.pem",
            ],
        );
        openssl(
            directory.path(),
            &[
                "x509", "-in", "cert.pem", "-outform", "DER", "-out", "cert.der",
            ],
        );
        openssl(
            directory.path(),
            &[
                "pkcs12",
                "-export",
                "-inkey",
                "key.pem",
                "-in",
                "cert.pem",
                "-out",
                "identity.p12",
                "-passout",
                "pass:fixture-only",
            ],
        );
        let der = fs::read(directory.path().join("cert.der")).unwrap();
        let pin = Sha256::digest(&der)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        let identity = Identity::from_pkcs12(
            &fs::read(directory.path().join("identity.p12")).unwrap(),
            "fixture-only",
        )
        .expect("import fixture into native-tls's temporary keychain");
        Self {
            identity,
            pin,
            _directory: directory,
        }
    }

    fn acceptor(&self) -> TlsAcceptor {
        TlsAcceptor::builder(self.identity.clone())
            .min_protocol_version(Some(Protocol::Tlsv10))
            .max_protocol_version(Some(Protocol::Tlsv10))
            .build()
            .unwrap()
    }
}

fn policy(expected_host: &str, pin: &str) -> Tls {
    Tls {
        provider: Some(TlsProvider::LegacyPinned {
            expected_host: expected_host.into(),
            sha256: pin.into(),
        }),
        ..Default::default()
    }
}

fn client(address: SocketAddr) -> Stream {
    Stream::connect_tcp(
        HOST,
        address.port(),
        TcpConnectOptions {
            proxy: Proxy::None,
            retry: Retry::Never,
        },
    )
    .unwrap()
}

fn listener() -> TcpListener {
    let listener = TcpListener::bind((HOST, 0)).unwrap();
    listener.set_nonblocking(true).unwrap();
    listener
}

fn accept(listener: TcpListener) -> TcpStream {
    let deadline = Instant::now() + IO_TIMEOUT;
    loop {
        match listener.accept() {
            Ok((tcp, _)) => {
                // macOS may inherit O_NONBLOCK from the listening socket.
                tcp.set_nonblocking(false).unwrap();
                tcp.set_read_timeout(Some(IO_TIMEOUT)).unwrap();
                tcp.set_write_timeout(Some(IO_TIMEOUT)).unwrap();
                return tcp;
            }
            Err(error)
                if error.kind() == io::ErrorKind::WouldBlock && Instant::now() < deadline =>
            {
                thread::sleep(Duration::from_millis(5));
            }
            Err(error) => panic!("fixture accept failed: {error}"),
        }
    }
}

/// Observe decrypted application data; timeout is a fixture failure, not EOF.
fn read_until_closed(mut stream: impl Read) -> Vec<u8> {
    let mut received = Vec::new();
    let mut buf = [0; 256];
    loop {
        match stream.read(&mut buf) {
            Ok(0) => return received,
            Ok(n) => received.extend_from_slice(&buf[..n]),
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::ConnectionReset
                        | io::ErrorKind::ConnectionAborted
                        | io::ErrorKind::BrokenPipe
                        | io::ErrorKind::UnexpectedEof
                ) =>
            {
                return received;
            }
            Err(error) => panic!("fixture read failed before peer closure: {error}"),
        }
    }
}

#[test]
fn matching_leaf_pin_allows_application_roundtrip() {
    let certificate = Certificate::generate();
    let listener = listener();
    let address = listener.local_addr().unwrap();
    let acceptor = certificate.acceptor();
    let server = thread::spawn(move || {
        let mut tls = acceptor
            .accept(accept(listener))
            .expect("fixture TLS1.0 accept");
        let mut request = vec![0; APPLICATION.len()];
        tls.read_exact(&mut request).unwrap();
        assert_eq!(request, APPLICATION);
        tls.write_all(REPLY).unwrap();
        tls.flush().unwrap();
    });
    let mut tls = client(address)
        .upgrade_tls(&policy(HOST, &certificate.pin))
        .unwrap();
    tls.set_read_timeout(Some(IO_TIMEOUT)).unwrap();
    tls.write_all(APPLICATION).unwrap();
    let mut reply = vec![0; REPLY.len()];
    tls.read_exact(&mut reply).unwrap();
    assert_eq!(reply, REPLY);
    drop(tls);
    server.join().unwrap();
}

fn assert_rejected_leaf(certificate: &Certificate, pin: &str) {
    let listener = listener();
    let address = listener.local_addr().unwrap();
    let acceptor = certificate.acceptor();
    let server = thread::spawn(move || {
        let tls = acceptor
            .accept(accept(listener))
            .expect("TLS must reach the leaf check");
        read_until_closed(tls)
    });
    // If a regression returns an unverified stream, exercise the caller's
    // immediate authentication write and let the peer observe the leak.
    let result = client(address).upgrade_tls(&policy(HOST, pin));
    let error = match result {
        Ok(mut stream) => {
            let _ = stream.write_all(APPLICATION);
            drop(stream);
            None
        }
        Err(error) => Some(error),
    };
    let received = server.join().unwrap();
    assert!(
        received.is_empty(),
        "pin failure leaked application bytes: {received:?}"
    );
    let error = error.expect("incorrect leaf pin was accepted");
    assert!(
        error.to_string().contains("leaf pin mismatch"),
        "unexpected failure: {error:#}"
    );
}

#[test]
fn wrong_pin_sends_zero_application_bytes() {
    let certificate = Certificate::generate();
    let wrong = if certificate.pin.starts_with('0') {
        "1"
    } else {
        "0"
    };
    assert_rejected_leaf(&certificate, &format!("{wrong}{}", &certificate.pin[1..]));
}

#[test]
fn changed_certificate_with_old_pin_sends_zero_application_bytes() {
    let previous = Certificate::generate();
    let replacement = Certificate::generate();
    assert_ne!(previous.pin, replacement.pin);
    assert_rejected_leaf(&replacement, &previous.pin);
}

#[test]
fn wrong_host_and_malformed_pin_fail_before_client_hello() {
    for (host, pin, expected_error) in [
        ("other.invalid", "a".repeat(64), "hostname"),
        (HOST, "a".repeat(63), "SHA-256"),
        (HOST, "g".repeat(64), "SHA-256"),
    ] {
        let listener = listener();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || read_until_closed(accept(listener)));
        let result = client(address).upgrade_tls(&policy(host, &pin));
        let received = server.join().unwrap();
        assert!(received.is_empty(), "invalid profile sent handshake bytes");
        let error = result.unwrap_err();
        assert!(error.to_string().contains(expected_error), "{error:#}");
    }
}

fn assert_handshake_deadline(trickle: bool) {
    let listener = listener();
    let address = listener.local_addr().unwrap();
    let (stop, stopped) = mpsc::channel();
    let server = thread::spawn(move || {
        let mut tcp = accept(listener);
        let mut hello = [0; 8192];
        assert!(tcp.read(&mut hello).unwrap() > 0, "no ClientHello");
        if trickle {
            // An incomplete TLS1.0 handshake record, kept below its announced
            // size. Every read gets progress; only a TOTAL deadline stops it.
            tcp.write_all(&[22, 3, 1, 0x40, 0]).unwrap();
        }
        let safety = Instant::now() + Duration::from_secs(15);
        while Instant::now() < safety {
            match stopped.recv_timeout(Duration::from_millis(200)) {
                Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
                Err(mpsc::RecvTimeoutError::Timeout) => {}
            }
            if trickle && tcp.write_all(&[0]).is_err() {
                break;
            }
        }
    });
    let plain = client(address);
    let start = Instant::now();
    let result = plain.upgrade_tls(&policy(HOST, &"a".repeat(64)));
    let elapsed = start.elapsed();
    let _ = stop.send(());
    server.join().unwrap();
    assert!(
        result.is_err(),
        "incomplete handshake unexpectedly succeeded"
    );
    assert!(
        elapsed >= Duration::from_secs(8),
        "fixture failed before testing the deadline: {elapsed:?}; {result:?}"
    );
    assert!(
        elapsed < Duration::from_secs(13),
        "10-second handshake deadline was not enforced: {elapsed:?}"
    );
}

#[test]
fn silent_handshake_expires() {
    assert_handshake_deadline(false);
}

#[test]
fn trickling_handshake_cannot_extend_total_deadline() {
    assert_handshake_deadline(true);
}
