mod imap;
mod locks;
mod message;
mod proxy;
mod smtp;
mod tls;

pub use imap::{
    connect_session as connect_imap_session, delete_uid_set as delete_imap_uid_set,
    test as test_imap, update_flags as update_imap_flags,
};
pub use locks::MailboxLocks;
pub use message::{MailAttachment, ParsedMail, normalize_thread_subject, parse_message};
pub use proxy::{BoxStream, connect as connect_via_proxy};
pub use smtp::{send as send_smtp, test as test_smtp};
