use meowmail::{
    AppState,
    calendar::{CalendarRepository, LocalCalendarEventInput},
    config::Config,
    db::entities::user,
    error::AppError,
    users::UserRepository,
};
use sea_orm::{ActiveModelTrait, Set};
use time::OffsetDateTime;
use uuid::Uuid;

#[tokio::test]
async fn local_calendar_events_support_crud_and_user_isolation() {
    let directory = tempfile::tempdir().unwrap();
    let state = AppState::initialize(
        Config::new(
            "correct horse battery staple".into(),
            directory.path().to_path_buf(),
        )
        .unwrap(),
    )
    .await
    .unwrap();
    let owner = UserRepository::new(state.db.clone())
        .authenticate_local("admin", "correct horse battery staple")
        .await
        .unwrap();
    let other_id = Uuid::new_v4();
    let now = OffsetDateTime::now_utc().unix_timestamp();
    user::ActiveModel {
        id: Set(other_id.to_string()),
        username: Set("calendar.other".into()),
        nickname: Set("Calendar Other".into()),
        email: Set(None),
        role: Set("user".into()),
        password_hash: Set(None),
        pin_hash: Set(None),
        avatar_mime: Set(None),
        avatar_data: Set(None),
        ai_enabled: Set(false),
        auto_lock_minutes: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        last_login_at: Set(None),
    }
    .insert(state.db.connection())
    .await
    .unwrap();

    let repository = CalendarRepository::new(state.db.clone(), state.vault.clone());
    let created = repository
        .create_local_event(
            owner.id,
            LocalCalendarEventInput {
                summary: "Project review".into(),
                description: "Review the release plan".into(),
                location: "Room 3".into(),
                starts_at: 2_000_000_000,
                ends_at: 2_000_003_600,
                all_day: false,
            },
        )
        .await
        .unwrap();

    let listed = repository
        .list_local_events(owner.id, 1_999_999_000, 2_000_004_000)
        .await
        .unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].summary, "Project review");
    assert!(
        repository
            .list_local_events(other_id, 1_999_999_000, 2_000_004_000)
            .await
            .unwrap()
            .is_empty()
    );

    let denied = repository
        .update_local_event(
            other_id,
            created.id,
            LocalCalendarEventInput {
                summary: "Changed".into(),
                description: String::new(),
                location: String::new(),
                starts_at: 2_000_000_000,
                ends_at: 2_000_003_600,
                all_day: false,
            },
        )
        .await;
    assert!(matches!(denied, Err(AppError::NotFound)));

    let updated = repository
        .update_local_event(
            owner.id,
            created.id,
            LocalCalendarEventInput {
                summary: "Release review".into(),
                description: "Final review".into(),
                location: "Room 5".into(),
                starts_at: 2_000_000_000,
                ends_at: 2_000_007_200,
                all_day: false,
            },
        )
        .await
        .unwrap();
    assert_eq!(updated.summary, "Release review");
    assert_eq!(updated.location, "Room 5");

    repository
        .delete_local_event(owner.id, created.id)
        .await
        .unwrap();
    assert!(
        repository
            .list_local_events(owner.id, 1_999_999_000, 2_000_008_000)
            .await
            .unwrap()
            .is_empty()
    );
}
