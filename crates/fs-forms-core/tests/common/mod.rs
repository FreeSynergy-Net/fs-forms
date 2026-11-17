//! Shared helpers of the integration tests: real in-memory stores, an in-memory group tree, a collecting delivery.

#![allow(dead_code)] // each test file uses a different subset

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub mod ai;

use async_trait::async_trait;
use fs_forms_core::{
    Anonymity, Answer, AnswerValue, Credential, Error, GroupLookup, Question, QuestionKind, Result,
    Survey, SurveyRepo, TokenDelivery,
};
use fs_objects::{ActivityFilter, ObjectId, Query, Store, TursoStore};

pub const ME: &str = "https://example.org/users/owner";

/// Delivery that remembers every token it was given.
#[derive(Default)]
pub struct Collect(pub Mutex<Vec<(String, String)>>);

impl Collect {
    pub fn token_of(&self, person: &str) -> String {
        let all = self.0.lock().unwrap();
        all.iter().find(|(p, _)| p == person).unwrap().1.clone()
    }
    pub fn all(&self) -> Vec<String> {
        self.0
            .lock()
            .unwrap()
            .iter()
            .map(|(_, t)| t.clone())
            .collect()
    }
}

#[async_trait]
impl TokenDelivery for Collect {
    async fn send(&self, person: &str, _survey: ObjectId, token: &str) -> Result<()> {
        self.0.lock().unwrap().push((person.into(), token.into()));
        Ok(())
    }
}

/// Delivery that always fails.
pub struct Failing;

#[async_trait]
impl TokenDelivery for Failing {
    async fn send(&self, _p: &str, _s: ObjectId, _t: &str) -> Result<()> {
        Err(Error::Invalid("mail server is down".into()))
    }
}

/// A group the tests made.
pub struct Made {
    pub id: ObjectId,
}

/// Who can be in a group (the tests only need people).
pub enum Member {
    Person(String),
}

/// A small in-memory group tree and its members, answering `GroupLookup`.
///
/// The tests used the real `fs-groups-core` (built on fs-objects 0.0.1). A tree with two sources of a library is no longer
/// allowed (one source per library, ADR 0092 rule 2), and fs-groups has no tag yet on the same pins as the rest, so this
/// stand-in carries the same questions (ancestors and effective members) until that tag exists.
#[derive(Clone, Default)]
pub struct GroupRepo {
    parents: Arc<Mutex<HashMap<ObjectId, ObjectId>>>,
    members: Arc<Mutex<HashMap<ObjectId, Vec<String>>>>,
}

impl GroupRepo {
    #[allow(clippy::unused_self)] // same shape as the real `GroupRepo::create`
    pub fn create(&self, _name: &str, _kind: &str) -> Made {
        Made {
            id: ObjectId::new(),
        }
    }

    pub fn add(&self, group: ObjectId, member: Member, _role: &str) {
        let Member::Person(person) = member;
        self.members
            .lock()
            .unwrap()
            .entry(group)
            .or_default()
            .push(person);
    }

    fn above(&self, group: ObjectId) -> Vec<ObjectId> {
        let parents = self.parents.lock().unwrap();
        let mut found = Vec::new();
        let mut at = group;
        while let Some(up) = parents.get(&at) {
            if found.contains(up) {
                break;
            }
            found.push(*up);
            at = *up;
        }
        found
    }
}

#[async_trait]
impl GroupLookup for GroupRepo {
    async fn ancestors(&self, group: ObjectId) -> Result<Vec<ObjectId>> {
        Ok(self.above(group))
    }
    async fn members(&self, group: ObjectId) -> Result<Vec<String>> {
        let all: Vec<ObjectId> = self.members.lock().unwrap().keys().copied().collect();
        let mut people = Vec::new();
        for g in all {
            if g == group || self.above(g).contains(&group) {
                for p in self
                    .members
                    .lock()
                    .unwrap()
                    .get(&g)
                    .cloned()
                    .unwrap_or_default()
                {
                    if !people.contains(&p) {
                        people.push(p);
                    }
                }
            }
        }
        Ok(people)
    }
}

/// Make `child` a child of `parent`.
pub fn nest(groups: &GroupRepo, child: ObjectId, parent: ObjectId) {
    groups.parents.lock().unwrap().insert(child, parent);
}

/// Everything a test needs.
pub struct Env {
    pub repo: SurveyRepo,
    pub main: Arc<TursoStore>,
    pub ballots: Arc<TursoStore>,
    pub delivery: Arc<Collect>,
    pub groups: GroupRepo,
}

pub async fn env() -> Env {
    let main = Arc::new(TursoStore::open_in_memory().await.unwrap());
    let ballots = Arc::new(TursoStore::open_in_memory().await.unwrap());
    let groups = GroupRepo::default();
    let delivery = Arc::new(Collect::default());
    let repo = SurveyRepo::new(main.clone(), ballots.clone(), ME)
        .with_instance("example.org")
        .with_groups(Arc::new(groups.clone()))
        .with_delivery(delivery.clone())
        .with_ai_marking(Arc::new(ai::Marks));
    Env {
        repo,
        main,
        ballots,
        delivery,
        groups,
    }
}

/// The credential of a person on an `anonymousOpen` survey: only used to ask the marking.
pub fn human() -> Credential {
    Credential::Person(person("h"))
}

pub fn person(n: &str) -> String {
    format!("https://example.org/people/{n}")
}

/// A single-choice question with choices `a`, `b`, `c`.
pub fn pick() -> Question {
    Question::new(QuestionKind::SingleChoice, "Which?").with_choices(&[
        ("a", "A"),
        ("b", "B"),
        ("c", "C"),
    ])
}

/// A published survey with one single-choice question; returns the survey and the question id.
pub async fn open_survey(e: &Env, anonymity: Anonymity) -> (Survey, fs_objects::ObjectId) {
    let s = e.repo.create("Lunch", anonymity, &[]).await.unwrap();
    let (_, q) = e
        .repo
        .add_question(s.id(), pick().required())
        .await
        .unwrap();
    (e.repo.publish(s.id()).await.unwrap(), q)
}

pub fn choice(q: ObjectId, c: &str) -> Vec<Answer> {
    vec![Answer {
        question: q,
        value: AnswerValue::Choice(c.into()),
    }]
}

/// `n` anonymous submits of `c`.
#[allow(clippy::many_single_char_names)] // short fixture names (env, survey, question)
pub async fn submit_open(e: &Env, s: &Survey, q: ObjectId, c: &str, n: usize) {
    for _ in 0..n {
        e.repo.submit(s.id(), human(), choice(q, c)).await.unwrap();
    }
}

/// Everything a store holds, as one text: objects and the activity log.
pub async fn raw(store: &TursoStore) -> String {
    let objects = store.query(&Query::new()).await.unwrap();
    let log = store.activities(&ActivityFilter::default()).await.unwrap();
    format!(
        "{}\n{}",
        serde_json::to_string(&objects).unwrap(),
        serde_json::to_string(&log).unwrap()
    )
}

/// The same for the ballot store: its objects are unlogged, so they are read by id (the box of `s`).
pub async fn raw_box(e: &Env, s: &Survey) -> String {
    use fs_objects::traits::UnloggedObjects;
    let whole = e.ballots.read_unlogged(s.id()).await.unwrap();
    let log = e
        .ballots
        .activities(&ActivityFilter::default())
        .await
        .unwrap();
    format!(
        "{}\n{}",
        serde_json::to_string(&whole).unwrap(),
        serde_json::to_string(&log).unwrap()
    )
}

pub async fn log_len(store: &TursoStore) -> usize {
    store
        .activities(&ActivityFilter::default())
        .await
        .unwrap()
        .len()
}

/// The raw box content of a survey.
pub async fn box_json(e: &Env, s: &Survey) -> serde_json::Value {
    use fs_objects::traits::UnloggedObjects;
    e.ballots
        .read_unlogged(s.id())
        .await
        .unwrap()
        .unwrap()
        .content
}

pub fn count(sum: &fs_forms_core::Summary, q: ObjectId, choice: &str) -> u32 {
    let t = sum.questions.iter().find(|t| t.question == q).unwrap();
    t.counts
        .iter()
        .find(|c| c.choice == choice)
        .map_or(0, |c| c.count)
}

impl Env {
    /// True if the box holds no token hash (or there is no box yet).
    pub async fn ballots_empty_tokens(&self, s: &Survey) -> bool {
        use fs_objects::traits::UnloggedObjects;
        match self.ballots.read_unlogged(s.id()).await.unwrap() {
            None => true,
            Some(o) => o.content["people"]["tokens"].as_array().unwrap().is_empty(),
        }
    }
}
