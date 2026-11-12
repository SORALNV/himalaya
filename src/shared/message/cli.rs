//! # Message command
//!
//! The `message` command, dispatching onto its subcommands.

use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

#[cfg(backend)]
use crate::shared::message::{
    add::MessageAddCommand, copy::MessageCopyCommand, delete::MessageDeleteCommand,
    forward::MessageForwardCommand, mv::MessageMoveCommand, parse::MessageParseCommand,
    read::MessageReadCommand, reply::MessageReplyCommand,
};
use crate::{
    account::context::Account,
    shared::{
        client::EmailClient,
        message::{compose::MessageComposeCommand, send::MessageSendCommand},
    },
};

/// Manage messages using the shared API.
///
/// `read` prints a message of a mailbox, `parse` the same view of a raw
/// message given as a file or on stdin, without an account.
///
/// `compose`, `reply` and `forward` build a message from flags and print
/// it, unless `--save` saves it to a mailbox or `--send` sends it.
///
/// `send` and `add` take a raw RFC 5322 message as it is, to send it or to
/// save it to a mailbox. Richer composition belongs to a standalone tool
/// such as mml, its output piped into them.
///
/// `copy`, `move` and `delete` act on messages already in a mailbox.
#[derive(Debug, Subcommand)]
pub enum MessageCommand {
    #[cfg(backend)]
    Read(MessageReadCommand),
    #[cfg(backend)]
    Parse(MessageParseCommand),
    #[command(visible_alias = "write", alias = "new")]
    Compose(MessageComposeCommand),
    #[cfg(backend)]
    Reply(MessageReplyCommand),
    #[cfg(backend)]
    #[command(visible_alias = "fwd")]
    Forward(MessageForwardCommand),
    Send(MessageSendCommand),
    #[cfg(backend)]
    #[command(visible_alias = "save")]
    Add(MessageAddCommand),
    #[cfg(backend)]
    #[command(visible_alias = "cp")]
    Copy(MessageCopyCommand),
    #[cfg(backend)]
    #[command(visible_alias = "mv")]
    Move(MessageMoveCommand),
    #[cfg(backend)]
    #[command(visible_alias = "rm", alias = "remove")]
    Delete(MessageDeleteCommand),
}

impl MessageCommand {
    /// Runs the subcommand against the active account's client.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut EmailClient,
    ) -> Result<()> {
        match self {
            #[cfg(backend)]
            Self::Add(cmd) => cmd.execute(printer, account, client),
            Self::Compose(cmd) => cmd.execute(printer, account, client),
            #[cfg(backend)]
            Self::Copy(cmd) => cmd.execute(printer, account, client),
            #[cfg(backend)]
            Self::Delete(cmd) => cmd.execute(printer, account, client),
            #[cfg(backend)]
            Self::Forward(cmd) => cmd.execute(printer, account, client),
            #[cfg(backend)]
            Self::Move(cmd) => cmd.execute(printer, account, client),
            #[cfg(backend)]
            Self::Parse(cmd) => cmd.execute(printer),
            #[cfg(backend)]
            Self::Read(cmd) => cmd.execute(printer, account, client),
            #[cfg(backend)]
            Self::Reply(cmd) => cmd.execute(printer, account, client),
            Self::Send(cmd) => cmd.execute(printer, account, client),
        }
    }
}
