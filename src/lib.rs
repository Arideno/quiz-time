use near_sdk::borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::collections::{LookupMap, UnorderedSet};
use near_sdk::{near_bindgen, AccountId, PanicOnDefault, env, BorshStorageKey};
use near_sdk::serde::{Deserialize, Serialize};

#[derive(BorshSerialize, BorshStorageKey)]
enum StorageKey {
    Quizzes,
    Published
}

// 1 near prize
const PRIZE_AMOUNT: u128 = 1_000_000_000_000_000_000_000_000;

#[derive(BorshDeserialize, BorshSerialize, Deserialize, Serialize, Debug, PartialEq)]
#[serde(crate = "near_sdk::serde")]
pub enum QuizStatus {
    Published,
    Unpublished
}

#[derive(Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct PublishedQuizzes {
    quizzes: Vec<JsonQuiz>,
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
    published_hashes: UnorderedSet<String>
}

#[near_bindgen]
impl QuizContract {
    #[init]
    pub fn new(owner_id: AccountId) -> Self {
        Self {
            owner_id,
            quizzes: LookupMap::new(StorageKey::Quizzes),
            published_hashes: UnorderedSet::new(StorageKey::Published)
        }
    }

    pub fn create_quiz(&mut self, hash: String, question: String, answers: Vec<String>, correct_index: usize, publish: bool) {
        self.check_owner();

        let status = if publish { QuizStatus::Published } else { QuizStatus::Unpublished };
        let existing_quiz = self.quizzes.insert(&hash, &Quiz {
            question, answers, correct_index, status
        });

        assert!(existing_quiz.is_none(), "Quiz with the same hash already exists");

        if publish {
            self.published_hashes.insert(&hash);
        }
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
        if quiz.status == QuizStatus::Unpublished {
            quiz.status = QuizStatus::Published;
            self.published_hashes.insert(&hash);
        }

        self.quizzes.insert(&hash, &quiz);
    }

    pub fn get_published_quizzes(&self) -> PublishedQuizzes {
        let hashes = self.published_hashes.to_vec();
        let mut quizzes = vec![];
        for hash in hashes {
            let quiz = self.quizzes.get(&hash).unwrap_or_else(|| env::panic_str("Cannot load quiz"));
            let json_quiz = JsonQuiz {
                hash,
                question: quiz.question,
                answers: quiz.answers
            };
            quizzes.push(json_quiz);
        }
        PublishedQuizzes { 
            quizzes
        }
    }

    #[private]
    pub fn check_owner(&self) {
        assert_eq!(self.owner_id, env::signer_account_id(), "This method can only be called by owner");
    }
}