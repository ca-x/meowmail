use meowmail::notifications::{NotificationEvent, render_template};
use uuid::Uuid;

#[test]
fn renders_documented_notification_placeholders() {
    let event = NotificationEvent {
        user_id: Uuid::nil(),
        account: "Work".into(),
        email: "me@example.com".into(),
        sender: "Alice".into(),
        sender_email: "alice@example.com".into(),
        subject: "Project update".into(),
        preview: "The build is ready".into(),
    };
    let message = render_template(
        "[{account}] {sender} <{sender_email}>: {subject} — {preview}",
        &event,
        None,
    )
    .unwrap();
    assert_eq!(
        message,
        "[Work] Alice <alice@example.com>: Project update — The build is ready"
    );
    assert!(render_template("{unknown}", &event, None).is_err());
    assert!(render_template("{subject", &event, None).is_err());
}
