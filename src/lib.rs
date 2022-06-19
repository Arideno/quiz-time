use near_sdk::borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::collections::{LookupMap, UnorderedSet};
use near_sdk::{near_bindgen, AccountId, PanicOnDefault, env};
use near_sdk::serde::{Deserialize, Serialize};

// 1 near prize
const PRIZE_AMOUNT: u128 = 1_000_000_000_000_000_000_000_000;

#[derive(BorshDeserialize, BorshSerialize, Deserialize, Serialize, Debug)]
#[serde(crate = "near_sdk::serde")]
pub enum QuizStatus {
    Published,
    Unpublished
}

#[derive(Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct UnsolvedQuizzes {
    puzzles: Vec<JsonQuiz>,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct JsonQuiz {
    hash: String,
    question: String,
    answers: Vec<String>
}

#[derive(BorshDeserialize, BorshSerialize, Debug)]
pub struct Quiz {
    status: QuizStatus,
    question: String,
    answers: Vec<String>,
    correct_index: usize
}

#[near_bindgen]
#[derive(BorshSerialize, BorshDeserialize, PanicOnDefault)]
pub struct QuizContract {
    owner_id: AccountId,
    quizzes: LookupMap<String, Quiz>,
}

#[near_bindgen]
impl QuizContract {
    #[init]
    pub fn new(owner_id: AccountId) -> Self {
        Self {
            owner_id,
            quizzes: LookupMap::new(b"q"),
        }
    }

    pub fn create_quiz(&mut self, hash: String, question: String, answers: Vec<String>, correct_index: usize, publish: bool) {
        self.check_owner();

        let status = if publish { QuizStatus::Published } else { QuizStatus::Unpublished };
        let existing_quiz = self.quizzes.insert(&hash, &Quiz {
            question, answers, correct_index, status
        });

        assert!(existing_quiz.is_none(), "Quiz with the same hash already exists");
    }

    pub fn get_quiz_status(&self, hash: String) -> Option<QuizStatus> {
        self.check_owner();

        if let Some(quiz) = self.quizzes.get(&hash) {
            return Some(quiz.status)
        }

        None
    }

    pub fn publish_quiz(&mut self, hash: String) {
        self.check_owner();

        let mut quiz = self.quizzes.get(&hash).expect("No such quiz found");
        quiz.status = QuizStatus::Published;

        self.quizzes.insert(&hash, &quiz);
    }

    #[private]
    pub fn check_owner(&self) {
        assert_eq!(self.owner_id, env::signer_account_id(), "This method can only be called by owner");
    }
}