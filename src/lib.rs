use near_sdk::borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::collections::{LookupMap, UnorderedSet};
use near_sdk::{near_bindgen, AccountId, PanicOnDefault, env, BorshStorageKey, Promise};
use near_sdk::serde::{Deserialize, Serialize};

#[derive(BorshSerialize, BorshStorageKey)]
enum StorageKey {
    Quizzes,
    PublishedQuizzes,
    SolvedQuizzes,
    RetriesLeft
}

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
    answers: Vec<String>,
    prize_amount: String
}

#[derive(BorshDeserialize, BorshSerialize, Debug)]
pub struct Quiz {
    status: QuizStatus,
    question: String,
    answers: Vec<String>,
    correct_index: usize,
    max_prize_amount: u128
}

#[near_bindgen]
#[derive(BorshSerialize, BorshDeserialize, PanicOnDefault)]
pub struct QuizContract {
    owner_id: AccountId,
    quizzes: LookupMap<String, Quiz>,
    published_hashes: UnorderedSet<String>,
    solved_quizzes: LookupMap<AccountId, UnorderedSet<String>>,
    retries_left: LookupMap<AccountId, LookupMap<String, usize>>
}

#[near_bindgen]
impl QuizContract {
    #[init]
    pub fn new(owner_id: AccountId) -> Self {
        Self {
            owner_id: owner_id,
            quizzes: LookupMap::new(StorageKey::Quizzes),
            published_hashes: UnorderedSet::new(StorageKey::PublishedQuizzes),
            solved_quizzes: LookupMap::new(StorageKey::SolvedQuizzes),
            retries_left: LookupMap::new(StorageKey::RetriesLeft)
        }
    }

    pub fn submit_answer(&mut self, hash: String, index: usize) -> String {
        let quiz = self.quizzes.get(&hash).expect("No such quiz found");
        assert!(quiz.status == QuizStatus::Published, "Cannot submit an answer to unpublished quiz");
        let account_id = env::predecessor_account_id();
        let mut solved_quizzes_set = self.solved_quizzes.get(&account_id).unwrap_or_else(|| {
            let mut prefix = Vec::with_capacity(33);
            prefix.push(b's');
            prefix.extend(env::sha256(account_id.as_bytes()));
            UnorderedSet::new(prefix)
        });

        if solved_quizzes_set.contains(&hash) {
            env::panic_str("This quiz is already solved by you");
        }

        let mut retries_left_map: LookupMap<String, usize> = self.retries_left.get(&account_id).unwrap_or_else(|| {
            let mut prefix = Vec::with_capacity(33);
            prefix.push(b'r');
            prefix.extend(env::sha256(account_id.as_bytes()));
            LookupMap::new(prefix)
        });

        let mut retries_left = retries_left_map.get(&hash).unwrap_or(3);

        if retries_left == 0 {
            env::panic_str("You can no longer solve this quiz. You are out of tries.");
        }

        if index == quiz.correct_index {
            solved_quizzes_set.insert(&hash);
            self.solved_quizzes.insert(&account_id, &solved_quizzes_set);

            let amount = quiz.max_prize_amount / (4 - retries_left) as u128;

            Promise::new(account_id.clone()).transfer(amount);

            return format!("Your answer is correct. You've got {} yoctoNEAR", amount);
        } else {
            retries_left -= 1;

            retries_left_map.insert(&hash, &retries_left);

            self.retries_left.insert(&account_id, &retries_left_map);

            if retries_left == 0 {
                return format!("The answer is not right, you are out of tries. The correct answer is `{}`", quiz.answers[quiz.correct_index]);
            }

            return format!("The answer is not right. You have {} retries left", retries_left);
        }
    }

    pub fn create_quiz(&mut self, hash: String, question: String, answers: Vec<String>, correct_index: usize, max_prize_amount: String, publish: bool) {
        self.check_owner();

        let status = if publish { QuizStatus::Published } else { QuizStatus::Unpublished };
        let existing_quiz = self.quizzes.insert(&hash, &Quiz {
            question, answers, correct_index, max_prize_amount: max_prize_amount.parse::<u128>().unwrap(), status
        });

        assert!(existing_quiz.is_none(), "Quiz with the same hash already exists");

        if publish {
            self.published_hashes.insert(&hash);
        }
    }

    pub fn get_quiz_status(&self, hash: String) -> Option<QuizStatus> {
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
                answers: quiz.answers,
                prize_amount: quiz.max_prize_amount.to_string()
            };
            quizzes.push(json_quiz);
        }
        PublishedQuizzes { 
            quizzes
        }
    }

    #[private]
    pub fn check_owner(&self) {
        assert_eq!(self.owner_id, env::predecessor_account_id(), "This method can only be called by owner");
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use std::panic::PanicInfo;

    use super::*;
    use near_sdk::test_utils::VMContextBuilder;
    use near_sdk::{testing_env};

    fn get_context(signer: AccountId, is_view: bool) -> VMContextBuilder {
        let mut builder = VMContextBuilder::new();
        builder.predecessor_account_id(signer);
        builder.is_view(is_view);
        builder
    }

    #[test]
    fn create_new_contract() {
        let account_id = AccountId::new_unchecked("bob.near".to_owned());

        let context = get_context(account_id.clone(), false);
        testing_env!(context.build());

        let contract = QuizContract::new(account_id);
        assert_eq!(contract.owner_id, env::signer_account_id());
    }

    #[test]
    fn get_published_quizzes_empty() {
        let account_id = AccountId::new_unchecked("bob.near".to_owned());

        let context = get_context(account_id.clone(), true);
        testing_env!(context.build());

        let contract = QuizContract::new(account_id);

        let published_quizzes = contract.get_published_quizzes();

        assert_eq!(published_quizzes.quizzes.len(), 0);
    }

    #[test]
    fn get_published_quizzes() {
        let account_id = AccountId::new_unchecked("bob.near".to_owned());

        let context = get_context(account_id.clone(), false);
        testing_env!(context.build());

        let mut contract = QuizContract::new(account_id);
        contract.create_quiz("1".to_owned(), "What is the capital of France".to_owned(), vec!["Kyiv".to_owned(), "Madrid".to_owned(), "Paris".to_owned(), "Berlin".to_owned()], 2, "1".to_owned(), true);

        let published_quizzes = contract.get_published_quizzes();
        assert_eq!(published_quizzes.quizzes.len(), 1);
        assert_eq!(published_quizzes.quizzes[0].hash, "1");
    }

    #[test]
    fn create_new_quiz() {
        let account_id = AccountId::new_unchecked("bob.near".to_owned());

        let context = get_context(account_id.clone(), false);
        testing_env!(context.build());

        let mut contract = QuizContract::new(account_id);
        contract.create_quiz("1".to_owned(), "What is the capital of France".to_owned(), vec!["Kyiv".to_owned(), "Madrid".to_owned(), "Paris".to_owned(), "Berlin".to_owned()], 2, "1".to_owned(), true);

        let quiz = contract.quizzes.get(&"1".to_owned()).unwrap();
        assert_eq!(quiz.question, "What is the capital of France");
    }

    #[test]
    #[should_panic]
    fn crate_quiz_only_by_owner() {
        let f  = |_: &PanicInfo| {};
        std::panic::set_hook(Box::new(f));

        let account_id = AccountId::new_unchecked("bob.near".to_owned());

        let context = get_context(account_id.clone(), false);
        testing_env!(context.build());

        let mut contract = QuizContract::new(account_id);

        let alice = AccountId::new_unchecked("alice.near".to_owned());

        let context = get_context(alice, false);
        testing_env!(context.build());

        contract.create_quiz("1".to_owned(), "What is the capital of France".to_owned(), vec!["Kyiv".to_owned(), "Madrid".to_owned(), "Paris".to_owned(), "Berlin".to_owned()], 2, "1".to_owned(), true);
    }

    #[test]
    fn get_publish_status() {
        let account_id = AccountId::new_unchecked("bob.near".to_owned());

        let context = get_context(account_id.clone(), false);
        testing_env!(context.build());

        let mut contract = QuizContract::new(account_id);

        contract.create_quiz("1".to_owned(), "What is the capital of France".to_owned(), vec!["Kyiv".to_owned(), "Madrid".to_owned(), "Paris".to_owned(), "Berlin".to_owned()], 2, "1".to_owned(), false);
        assert_eq!(contract.get_quiz_status("1".to_owned()).unwrap(), QuizStatus::Unpublished);

        contract.create_quiz("2".to_owned(), "What is the capital of France".to_owned(), vec!["Kyiv".to_owned(), "Madrid".to_owned(), "Paris".to_owned(), "Berlin".to_owned()], 2, "1".to_owned(), true);
        assert_eq!(contract.get_quiz_status("2".to_owned()).unwrap(), QuizStatus::Published);
    }

    #[test]
    #[should_panic]
    fn submit_answer_to_unpublished() {
        let f  = |_: &PanicInfo| {};
        std::panic::set_hook(Box::new(f));
        
        let account_id = AccountId::new_unchecked("bob.near".to_owned());

        let context = get_context(account_id.clone(), false);
        testing_env!(context.build());

        let mut contract = QuizContract::new(account_id);
        contract.create_quiz("1".to_owned(), "What is the capital of France".to_owned(), vec!["Kyiv".to_owned(), "Madrid".to_owned(), "Paris".to_owned(), "Berlin".to_owned()], 2, "1".to_owned(), false);

        contract.submit_answer("1".to_owned(), 2);
    }

    #[test]
    fn submit_correct_answer_to_published() {
        let account_id = AccountId::new_unchecked("bob.near".to_owned());

        let context = get_context(account_id.clone(), false);
        testing_env!(context.build());

        let mut contract = QuizContract::new(account_id.clone());
        contract.create_quiz("1".to_owned(), "What is the capital of France".to_owned(), vec!["Kyiv".to_owned(), "Madrid".to_owned(), "Paris".to_owned(), "Berlin".to_owned()], 2, "1".to_owned(), true);

        contract.submit_answer("1".to_owned(), 2);

        assert_eq!(contract.solved_quizzes.get(&account_id).unwrap().contains(&"1".to_owned()), true);
    }

    #[test]
    fn submit_incorrect_answer_to_published() {
        let account_id = AccountId::new_unchecked("bob.near".to_owned());

        let context = get_context(account_id.clone(), false);
        testing_env!(context.build());

        let mut contract = QuizContract::new(account_id.clone());
        contract.create_quiz("1".to_owned(), "What is the capital of France".to_owned(), vec!["Kyiv".to_owned(), "Madrid".to_owned(), "Paris".to_owned(), "Berlin".to_owned()], 2, "1".to_owned(), true);

        contract.submit_answer("1".to_owned(), 1);

        assert_eq!(contract.retries_left.get(&account_id).unwrap().get(&"1".to_owned()).unwrap(), 2);
        contract.submit_answer("1".to_owned(), 0);
        assert_eq!(contract.retries_left.get(&account_id).unwrap().get(&"1".to_owned()).unwrap(), 1);
    }

    #[test]
    fn publish_unpublished_quiz() {
        let account_id = AccountId::new_unchecked("bob.near".to_owned());

        let context = get_context(account_id.clone(), false);
        testing_env!(context.build());

        let mut contract = QuizContract::new(account_id.clone());
        contract.create_quiz("1".to_owned(), "What is the capital of France".to_owned(), vec!["Kyiv".to_owned(), "Madrid".to_owned(), "Paris".to_owned(), "Berlin".to_owned()], 2, "1".to_owned(), false);

        assert_eq!(contract.quizzes.get(&"1".to_owned()).unwrap().status, QuizStatus::Unpublished);
        contract.publish_quiz("1".to_owned());
        assert_eq!(contract.quizzes.get(&"1".to_owned()).unwrap().status, QuizStatus::Published);
    }
}