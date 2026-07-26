mod imap;
mod message;
mod proxy;
mod smtp;
mod tls;

pub use imap::{connect_session as connect_imap_session, test as test_imap};
pub use message::{ParsedMail, parse_message};
pub use proxy::{BoxStream, connect as connect_via_proxy};
pub use smtp::{send as send_smtp, test as test_smtp};
