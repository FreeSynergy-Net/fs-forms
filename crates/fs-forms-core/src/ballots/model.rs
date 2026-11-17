//! The content of one box: pending ballots, released batches, token hashes, once for people and once for AIs.
//!
//! Pattern: Aggregate — one value per survey, changed only through its methods. A ballot has no
//! person, time or id; it enters at a random position; release shuffles it. The two classes have a lane
//! each, so AI ballots never fill a batch of people, nor the other way round. A box of version 0.1.0 has
//! no classes; what it holds is read as AI (doubt).

use fs_ballot::Lane;
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::model::{Answer, Class, Signals, Voluntary};

/// One anonymous ballot: answers in question order, nothing else.
pub type Ballot = Vec<Answer>;

/// The lane of one class: the logic of the box is `fs_ballot::Lane`; the pattern counts travel with its batches.
type ClassLane = Lane<Ballot, Signals>;

/// The box of a survey.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(from = "Stored")]
pub struct BallotBox {
    people: ClassLane,
    ai: ClassLane,
    /// Details given after the answers, in random order, tied to no ballot.
    voluntary: Vec<Vec<Voluntary>>,
}

/// The stored form. Version 0.1.0 wrote one lane at the top with no class; such ballots and token
/// hashes are not known to be people's, so doubt counts them as AI and they are moved into that lane.
#[derive(Deserialize)]
struct Stored {
    #[serde(default)]
    people: ClassLane,
    #[serde(default)]
    ai: ClassLane,
    #[serde(default)]
    voluntary: Vec<Vec<Voluntary>>,
    #[serde(flatten)]
    legacy: ClassLane,
}

impl From<Stored> for BallotBox {
    fn from(mut s: Stored) -> Self {
        s.ai.merge_from(s.legacy);
        Self {
            people: s.people,
            ai: s.ai,
            voluntary: s.voluntary,
        }
    }
}

/// `None` for "all stages" (`0xFFFF` in the contract), else the stage.
fn only(stage: u16) -> Option<u16> {
    (stage != crate::model::ALL_STAGES).then_some(stage)
}

impl BallotBox {
    fn lane(&self, class: Class) -> &ClassLane {
        if class.is_ai() {
            &self.ai
        } else {
            &self.people
        }
    }

    fn lane_mut(&mut self, class: Class) -> &mut ClassLane {
        if class.is_ai() {
            &mut self.ai
        } else {
            &mut self.people
        }
    }

    /// Add a ballot at a random position among the pending ones of its class.
    pub fn add_ballot(&mut self, class: Class, ballot: Ballot) -> Result<()> {
        Ok(self.lane_mut(class).add_ballot(ballot)?)
    }

    /// Add a ballot of an open survey together with its pattern counts (kept as a count, not with the ballot).
    pub fn add_open_ballot(
        &mut self,
        class: Class,
        ballot: Ballot,
        signals: &Signals,
    ) -> Result<()> {
        Ok(self.lane_mut(class).add_ballot_with(ballot, signals)?)
    }

    /// Add voluntary details at a random position among the others.
    pub fn add_voluntary(&mut self, details: Vec<Voluntary>) -> Result<()> {
        Ok(fs_ballot::random::insert_random(
            &mut self.voluntary,
            details,
        )?)
    }

    /// The voluntary details given so far.
    pub fn voluntary(&self) -> &[Vec<Voluntary>] {
        &self.voluntary
    }

    /// Pattern counts of the released batches of `class` and `stage` (`0xFFFF` = all).
    pub fn released_signals(&self, class: Class, stage: u16) -> Signals {
        let mut total = Signals::default();
        for b in self.lane(class).batches() {
            if only(stage).is_none_or(|s| b.stage() == s) {
                total.add(b.meta());
            }
        }
        total
    }

    /// Keep a token hash in the lane of the class it was issued to (sorted, so the order says nothing
    /// about who was invited when).
    pub fn add_token(&mut self, class: Class, hash: String) {
        self.lane_mut(class).add_token(hash);
    }

    /// The class a token hash was issued to; none if it is spent or was never issued.
    pub fn token_class(&self, hash: &str) -> Option<Class> {
        Class::BOTH
            .into_iter()
            .find(|c| self.lane(*c).has_token(hash))
    }

    /// Remove a token hash from either lane; false if it is not there (spent or never issued).
    pub fn spend(&mut self, hash: &str) -> bool {
        let Some(class) = self.token_class(hash) else {
            return false;
        };
        self.lane_mut(class).spend(hash)
    }

    /// Release the pending ballots of each class as one shuffled batch of `stage`, if there are at
    /// least `min_group` of that class; a smaller rest stays.
    pub fn release(&mut self, stage: u16, min_group: u16) -> Result<()> {
        for c in Class::BOTH {
            self.lane_mut(c).release(stage, min_group)?;
        }
        Ok(())
    }

    /// Released ballots of `class` and `stage` (`0xFFFF` = all), each with the number of its batch.
    pub fn released(&self, class: Class, stage: u16) -> Vec<(usize, &Ballot)> {
        self.lane(class).released(only(stage))
    }

    /// Ballots of `class` not released yet.
    pub fn held_back(&self, class: Class) -> usize {
        self.lane(class).held_back()
    }

    /// Whether the AI lane holds any ballot, released or not.
    pub fn has_ai_ballots(&self) -> bool {
        self.ai.has_ballots()
    }
}
