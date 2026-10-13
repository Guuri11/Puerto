//! The `local` feature (ADR-0021 § 2): argon2id password hashes behind `PasswordHasher`.

use puerto_auth::local::{Argon2Hasher, hash_password, verify_password};
use puerto_core::auth::PasswordHasher;

#[test]
fn should_accept_the_password_a_hash_was_made_from() {
    // Arrange
    let hash = hash_password("correct horse battery").expect("hashed");

    // Act / Assert
    assert!(verify_password("correct horse battery", &hash));
    assert!(!verify_password("correct horse battery!", &hash));
}

#[test]
fn should_store_an_argon2id_hash_with_its_own_salt() {
    // Act
    let first = hash_password("correct horse battery").expect("hashed");
    let second = hash_password("correct horse battery").expect("hashed");

    // Assert
    assert!(first.starts_with("$argon2id$"), "{first}");
    assert_ne!(first, second, "a salt per hash");
    assert!(!first.contains("correct horse battery"));
}

#[test]
fn should_refuse_any_password_when_the_stored_hash_is_not_one() {
    assert!(!verify_password("anything", "not a hash"));
    assert!(!verify_password("", ""));
}

#[tokio::test]
async fn should_check_a_password_through_the_port() {
    // Arrange
    let hasher: &dyn PasswordHasher = &Argon2Hasher::new();
    let hash = hasher.hash("correct horse battery").await.expect("hashed");

    // Act
    let right = hasher
        .verify("correct horse battery", Some(hash.clone()))
        .await;
    let wrong = hasher.verify("tr0ub4dor&3 guess", Some(hash)).await;

    // Assert
    assert!(right);
    assert!(!wrong);
}

#[tokio::test]
async fn should_refuse_without_a_hash_after_doing_the_same_work() {
    // Arrange
    let hasher = Argon2Hasher::new();
    let hash = hasher.hash("correct horse battery").await.expect("hashed");
    // Warm the fixed hash up: its first use computes it.
    assert!(!hasher.verify("x", None).await);

    // Act
    let started = std::time::Instant::now();
    let unknown = hasher.verify("correct horse battery", None).await;
    let without = started.elapsed();
    let started = std::time::Instant::now();
    let _ = hasher.verify("wrong password!", Some(hash)).await;
    let with = started.elapsed();

    // Assert: an unknown email is refused, and not in a fraction of a real check's time.
    assert!(!unknown);
    assert!(without * 4 > with, "{without:?} against {with:?}");
}
