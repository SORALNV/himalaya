//! # Native credential storage
//!
//! Creates new macOS generic-password items without update or shell APIs.

use std::{
    io::{Read, Write, stdin, stdout},
    ptr,
    str::from_utf8,
    time::Duration,
};

use anyhow::{Result, bail};
use core_foundation::{
    base::TCFType,
    data::{CFData, CFDataRef},
};
use security_framework::{
    os::macos::{keychain::SecKeychain, keychain_item::SecKeychainItem},
    passwords::get_generic_password,
};
use security_framework_sys::{
    base::SecKeychainItemRef,
    keychain::{SecKeychainAddGenericPassword, SecKeychainSetUserInteractionAllowed},
    keychain_item::SecKeychainItemDelete,
};
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use crate::wizard::web::worker;

unsafe extern "C" {
    fn SecKeychainItemCreatePersistentReference(
        item: SecKeychainItemRef,
        reference: *mut CFDataRef,
    ) -> i32;
    fn SecKeychainItemCopyFromPersistentReference(
        reference: CFDataRef,
        item: *mut SecKeychainItemRef,
    ) -> i32;
}

/// A rollback capability owning only an item created by this operation.
pub struct CreatedItem {
    /// Deletes the exact newly created item, returning cleanup failures.
    pub rollback: Box<dyn FnOnce() -> Result<()> + Send>,
}

/// Disables implicit Keychain prompts so OS interactions fail closed.
pub fn noninteractive() -> Result<()> {
    // SAFETY: the process-wide preference receives a valid Boolean value.
    let status = unsafe { SecKeychainSetUserInteractionAllowed(0) };
    if status != 0 {
        bail!("Cannot set noninteractive Keychain access");
    }
    Ok(())
}

/// Reads only the explicitly named generic password without echoing it.
pub fn read(entry: &str) -> Result<Zeroizing<String>> {
    let bytes = Zeroizing::new(get_generic_password(entry, entry).map_err(|_| {
        anyhow::anyhow!(
            "Keychain item unavailable; check its name, access permission and unlocked state"
        )
    })?);
    let value =
        from_utf8(&bytes).map_err(|_| anyhow::anyhow!("Keychain item is not a text password"))?;
    Ok(Zeroizing::new(value.to_string()))
}

/// Adds a new item and retains its native identity for exact rollback.
fn add(entry: &str, password: &str) -> Result<SecKeychainItem> {
    let keychain =
        SecKeychain::default().map_err(|_| anyhow::anyhow!("Default Keychain unavailable"))?;
    let mut item = ptr::null_mut();
    // SAFETY: all buffers and the keychain stay live through the call;
    // SecKeychainAddGenericPassword creates, never updates, an item.
    let status = unsafe {
        SecKeychainAddGenericPassword(
            keychain.as_concrete_TypeRef(),
            entry.len() as u32,
            entry.as_ptr().cast(),
            entry.len() as u32,
            entry.as_ptr().cast(),
            password.len() as u32,
            password.as_ptr().cast(),
            &mut item,
        )
    };
    if status != 0 || item.is_null() {
        bail!("Cannot create a new Keychain item");
    }
    // SAFETY: a successful Add returned one owned, non-null item reference.
    let item = unsafe { SecKeychainItem::wrap_under_create_rule(item) };
    Ok(item)
}

#[cfg(test)]
/// Creates a disposable in-process item for explicit native regression tests.
pub fn create(entry: &str, password: &str) -> Result<CreatedItem> {
    let item = add(entry, password)?;
    Ok(CreatedItem {
        rollback: Box::new(move || {
            // SAFETY: the retained item names exactly this operation's item.
            let status = unsafe { SecKeychainItemDelete(item.as_concrete_TypeRef()) };
            if status != 0 {
                bail!("Cannot remove the newly created Keychain item");
            }
            Ok(())
        }),
    })
}

#[derive(Deserialize, Serialize)]
#[serde(tag = "operation", rename_all = "lowercase", deny_unknown_fields)]
enum KeychainRequest {
    Read {
        entry: String,
    },
    Create {
        entry: String,
        password: Zeroizing<String>,
    },
    Delete {
        reference: Zeroizing<Vec<u8>>,
    },
}

#[derive(Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct KeychainReply {
    #[serde(skip_serializing_if = "Option::is_none")]
    password: Option<Zeroizing<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reference: Option<Zeroizing<Vec<u8>>>,
}

fn invoke(request: &KeychainRequest) -> Result<KeychainReply> {
    let input = Zeroizing::new(serde_json::to_vec(request)?);
    let (status, output) = worker::call(
        "--web-keychain-worker",
        &input,
        Duration::from_secs(10),
        &|| false,
    )?;
    if status != Some(0) {
        bail!("Keychain operation failed; check item name, permissions and unlocked state");
    }
    serde_json::from_slice(&output).map_err(|_| anyhow::anyhow!("Invalid native Keychain result"))
}

/// Reads one named item through a deadline-bounded native helper.
pub fn read_bounded(entry: &str) -> Result<Zeroizing<String>> {
    invoke(&KeychainRequest::Read {
        entry: entry.into(),
    })?
    .password
    .ok_or_else(|| anyhow::anyhow!("Native Keychain password unavailable"))
}

/// Creates a new item with exact persistent-reference rollback and a deadline.
pub fn create_bounded(entry: &str, password: &str) -> Result<CreatedItem> {
    let reply = invoke(&KeychainRequest::Create { entry: entry.into(), password: Zeroizing::new(password.into()) })
        .map_err(|_| anyhow::anyhow!("Keychain creation did not complete; configuration unchanged; inspect item {entry} before retrying because creation may be uncertain"))?;
    let reference = reply
        .reference
        .ok_or_else(|| anyhow::anyhow!("Native Keychain item reference unavailable"))?;
    Ok(CreatedItem {
        rollback: Box::new(move || {
            invoke(&KeychainRequest::Delete { reference })?;
            Ok(())
        }),
    })
}

/// Runs native credential operations only over bounded, private standard I/O.
pub fn execute_worker() -> Result<()> {
    noninteractive()?;
    let mut bytes = Zeroizing::new(Vec::new());
    stdin().take(32769).read_to_end(&mut bytes)?;
    if bytes.len() > 32768 {
        bail!("Keychain input exceeds limit");
    }
    let request: KeychainRequest =
        serde_json::from_slice(&bytes).map_err(|_| anyhow::anyhow!("Invalid Keychain input"))?;
    let mut reply = KeychainReply::default();
    match request {
        KeychainRequest::Read { entry } => {
            let password = read(&entry)?;
            if password.len() > 4096 {
                bail!("Keychain password exceeds limit");
            }
            reply.password = Some(password);
        }
        KeychainRequest::Create { entry, password } => {
            if entry.len() > 128
                || !entry.starts_with("himalaya-web-")
                || password.is_empty()
                || password.len() > 4096
                || password.chars().any(char::is_control)
            {
                bail!("Invalid new Keychain item");
            }
            let item = add(&entry, &password)?;
            let mut reference = ptr::null();
            // SAFETY: the item is retained and the out pointer is valid.
            let status = unsafe {
                SecKeychainItemCreatePersistentReference(item.as_concrete_TypeRef(), &mut reference)
            };
            if status != 0 || reference.is_null() {
                // SAFETY: this operation owns the exact newly created item.
                unsafe {
                    SecKeychainItemDelete(item.as_concrete_TypeRef());
                }
                bail!("Cannot retain new Keychain item identity");
            }
            // SAFETY: a successful call returned an owned CFData reference.
            let reference = unsafe { CFData::wrap_under_create_rule(reference) };
            reply.reference = Some(Zeroizing::new(reference.bytes().to_vec()));
        }
        KeychainRequest::Delete { reference } => {
            if reference.is_empty() || reference.len() > 4096 {
                bail!("Invalid Keychain item reference");
            }
            let data = CFData::from_buffer(&reference);
            let mut item = ptr::null_mut();
            // SAFETY: data remains live and the out pointer is valid.
            let status = unsafe {
                SecKeychainItemCopyFromPersistentReference(data.as_concrete_TypeRef(), &mut item)
            };
            if status != 0 || item.is_null() {
                bail!("Created Keychain item unavailable for cleanup");
            }
            // SAFETY: a successful Copy returned an owned item reference.
            let item = unsafe { SecKeychainItem::wrap_under_create_rule(item) };
            // SAFETY: the reference identifies the exact originally created item.
            if unsafe { SecKeychainItemDelete(item.as_concrete_TypeRef()) } != 0 {
                bail!("Created Keychain item cleanup failed");
            }
        }
    }
    let output = Zeroizing::new(serde_json::to_vec(&reply)?);
    if output.len() > 32768 {
        bail!("Keychain output exceeds limit");
    }
    stdout().write_all(&output)?;
    Ok(())
}
