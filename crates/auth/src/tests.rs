use super::*;
use kartoteka_db::test_helpers::test_pool;
use kartoteka_domain::auth::register;

#[tokio::test]
async fn authenticate_correct_password() {
    let pool = test_pool().await;
    register(&pool, "user@example.com", "secret123", None)
        .await
        .unwrap();
    let backend = KartotekaBackend::new(pool);
    let user = backend
        .authenticate(LoginCredentials {
            email: "user@example.com".to_string(),
            password: "secret123".to_string(),
        })
        .await
        .unwrap();
    assert!(user.is_some());
    assert_eq!(user.unwrap().email, "user@example.com");
}

#[tokio::test]
async fn authenticate_wrong_password_returns_none() {
    let pool = test_pool().await;
    register(&pool, "user@example.com", "secret123", None)
        .await
        .unwrap();
    let backend = KartotekaBackend::new(pool);
    let user = backend
        .authenticate(LoginCredentials {
            email: "user@example.com".to_string(),
            password: "wrongpass".to_string(),
        })
        .await
        .unwrap();
    assert!(user.is_none());
}

#[tokio::test]
async fn authenticate_unknown_email_returns_none() {
    let pool = test_pool().await;
    let backend = KartotekaBackend::new(pool);
    let user = backend
        .authenticate(LoginCredentials {
            email: "nobody@example.com".to_string(),
            password: "anything".to_string(),
        })
        .await
        .unwrap();
    assert!(user.is_none());
}

#[tokio::test]
async fn get_user_returns_registered_user() {
    let pool = test_pool().await;
    let info = register(&pool, "user@example.com", "pass", Some("Alice"))
        .await
        .unwrap();
    let backend = KartotekaBackend::new(pool);
    let user = backend.get_user(&info.id).await.unwrap().unwrap();
    assert_eq!(user.name.as_deref(), Some("Alice"));
    assert_eq!(user.role, "admin");
}

#[tokio::test]
async fn get_user_unknown_id_returns_none() {
    let pool = test_pool().await;
    let backend = KartotekaBackend::new(pool);
    assert!(
        backend
            .get_user(&"no-such-id".to_string())
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn get_user_by_id_finds_registered_user() {
    let pool = test_pool().await;
    let info = register(&pool, "user@example.com", "pass", Some("Bob"))
        .await
        .unwrap();
    let user = get_user_by_id(&pool, &info.id).await.unwrap().unwrap();
    assert_eq!(user.email, "user@example.com");
    assert_eq!(user.name.as_deref(), Some("Bob"));
}

#[tokio::test]
async fn get_user_by_id_unknown_returns_none() {
    let pool = test_pool().await;
    assert!(get_user_by_id(&pool, "no-such-id").await.unwrap().is_none());
}
