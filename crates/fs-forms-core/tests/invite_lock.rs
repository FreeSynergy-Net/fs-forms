//! Second review: tokens are delivered outside the survey lock.

mod common;

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use common::*;
use fs_forms_core::{Anonymity, Credential, Error, Result, SurveyRepo, TokenDelivery};
use fs_objects::ObjectId;
use tokio::sync::Semaphore;

/// Takes 300 ms per send, counts sends, and fails the first one when asked.
struct Slow {
    sent: AtomicUsize,
    calls: AtomicUsize,
    fail_first: bool,
}

#[async_trait]
impl TokenDelivery for Slow {
    async fn send(&self, _p: &str, _s: ObjectId, _t: &str) -> Result<()> {
        tokio::time::sleep(Duration::from_millis(300)).await;
        if self.calls.fetch_add(1, Ordering::SeqCst) == 0 && self.fail_first {
            return Err(Error::Invalid("down".into()));
        }
        self.sent.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

/// A delivery that stays inside `send` until the test opens it: `entered` gets a permit when a send
/// has begun, `go` is what the send waits for. No clock: the test knows by events where the delivery is.
struct Held {
    entered: Semaphore,
    go: Semaphore,
    sent: AtomicUsize,
}

#[async_trait]
impl TokenDelivery for Held {
    async fn send(&self, _p: &str, _s: ObjectId, _t: &str) -> Result<()> {
        self.entered.add_permits(1);
        self.go.acquire().await.unwrap().forget();
        self.sent.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

/// Far longer than anything a healthy run needs; only a blocked submit (the bug) ever reaches it.
const BLOCKED: Duration = Duration::from_secs(30);

fn slow(fail_first: bool) -> Arc<Slow> {
    Arc::new(Slow {
        sent: AtomicUsize::new(0),
        calls: AtomicUsize::new(0),
        fail_first,
    })
}

#[allow(clippy::many_single_char_names)] // short fixture names (env, survey, question)
#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn a_submit_does_not_wait_for_slow_deliveries() {
    let e = env().await;
    let d = Arc::new(Held {
        entered: Semaphore::new(0),
        go: Semaphore::new(0),
        sent: AtomicUsize::new(0),
    });
    let repo = SurveyRepo::new(e.main.clone(), e.ballots.clone(), ME)
        .with_ai_marking(std::sync::Arc::new(common::ai::Marks))
        .with_delivery(d.clone());
    let (s, q) = open_survey(&e, Anonymity::AnonymousOnce).await;
    let people: Vec<String> = (0..5).map(|n| person(&n.to_string())).collect();
    let (r, id) = (repo.clone(), s.id());
    let inviting = tokio::spawn(async move { r.invite(id, &people, &[]).await.unwrap() });
    // The invite is now inside a delivery, and stays there until the test opens it.
    d.entered.acquire().await.unwrap().forget();
    let r = tokio::time::timeout(
        BLOCKED,
        repo.submit(id, Credential::Token("0".repeat(64)), choice(q, "a")),
    )
    .await
    .expect("the submit waited for a delivery: the survey lock is held while sending");
    assert!(matches!(r, Err(Error::InvalidToken)));
    assert_eq!(
        d.sent.load(Ordering::SeqCst),
        0,
        "still inside the delivery"
    );
    d.go.add_permits(5);
    let report = tokio::time::timeout(BLOCKED, inviting)
        .await
        .expect("the invite never finished: a lock was not given back")
        .unwrap();
    assert_eq!(report.invited.len(), 5);
    assert_eq!(d.sent.load(Ordering::SeqCst), 5);
}

#[allow(clippy::many_single_char_names)] // short fixture names (env, survey, question)
#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn parallel_invites_of_one_person_send_one_token() {
    let e = env().await;
    let d = slow(false);
    let repo = SurveyRepo::new(e.main.clone(), e.ballots.clone(), ME)
        .with_ai_marking(std::sync::Arc::new(common::ai::Marks))
        .with_delivery(d.clone());
    let (s, _) = open_survey(&e, Anonymity::AnonymousOnce).await;
    let (r1, r2, id) = (repo.clone(), repo.clone(), s.id());
    let p = vec![person("ada")];
    let p2 = p.clone();
    let a = tokio::spawn(async move { r1.invite(id, &p, &[]).await.unwrap() });
    let b = tokio::spawn(async move { r2.invite(id, &p2, &[]).await.unwrap() });
    let (a, b) = (a.await.unwrap(), b.await.unwrap());
    assert_eq!(a.invited.len() + b.invited.len(), 1);
    assert_eq!(d.sent.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn a_failed_delivery_frees_the_person_and_keeps_no_hash() {
    let e = env().await;
    let d = slow(true);
    let repo = SurveyRepo::new(e.main.clone(), e.ballots.clone(), ME)
        .with_ai_marking(std::sync::Arc::new(common::ai::Marks))
        .with_delivery(d.clone());
    let (s, _) = open_survey(&e, Anonymity::AnonymousOnce).await;
    let p = vec![person("ada")];
    assert!(matches!(
        repo.invite(s.id(), &p, &[]).await,
        Err(Error::Delivery { .. })
    ));
    assert!(e.ballots_empty_tokens(&s).await, "no usable hash");
    let again = repo.invite(s.id(), &p, &[]).await.unwrap();
    assert_eq!(again.invited, p, "the in-flight mark was cleared");
}
