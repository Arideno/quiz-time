use near_sdk::borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::collections::{LookupMap, UnorderedSet};
use near_sdk::{near_bindgen, AccountId, PanicOnDefault, env, BorshStorageKey, Promise};
use near_sdk::serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};

type QuizId = u64;

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
    quiz_id: QuizId,
    question: String,
    prize_amount: String
}

#[derive(BorshDeserialize, BorshSerialize, Debug)]
pub struct Quiz {
    status: QuizStatus,
    question: String,
    correct_hash: String,
    max_prize_amount: u128
}

#[near_bindgen]
#[derive(BorshSerialize, BorshDeserialize, PanicOnDefault)]
pub struct QuizContract {
    owner_id: AccountId,
    quizzes: LookupMap<QuizId, Quiz>,
    published_quiz_ids: UnorderedSet<QuizId>,
    solved_quizzes: LookupMap<AccountId, UnorderedSet<QuizId>>,
    retries_left: LookupMap<AccountId, LookupMap<QuizId, usize>>,
    current_quiz_id: QuizId
}

#[near_bindgen]
impl QuizContract {
    #[init]
    pub fn new(owner_id: AccountId) -> Self {
        Self {
            owner_id: owner_id,
            quizzes: LookupMap::new(StorageKey::Quizzes),
            published_quiz_ids: UnorderedSet::new(StorageKey::PublishedQuizzes),
            solved_quizzes: LookupMap::new(StorageKey::SolvedQuizzes),
            retries_left: LookupMap::new(StorageKey::RetriesLeft),
            current_quiz_id: 0
        }
    }

    pub fn submit_answer(&mut self, quiz_id: QuizId, answer: String) -> String {
        let quiz = self.quizzes.get(&quiz_id).expect("No such quiz found");
        assert!(quiz.status == QuizStatus::Published, "Cannot submit an answer to unpublished quiz");
        let account_id = env::predecessor_account_id();
        let mut solved_quizzes_set = self.solved_quizzes.get(&account_id).unwrap_or_else(|| {
            let mut prefix = Vec::with_capacity(33);
            prefix.push(b's');
            prefix.extend(env::sha256(account_id.as_bytes()));
            UnorderedSet::new(prefix)
        });

        if solved_quizzes_set.contains(&quiz_id) {
            env::panic_str("This quiz is already solved by you");
        }

        let mut retries_left_map: LookupMap<QuizId, usize> = self.retries_left.get(&account_id).unwrap_or_else(|| {
            let mut prefix = Vec::with_capacity(33);
            prefix.push(b'r');
            prefix.extend(env::sha256(account_id.as_bytes()));
            LookupMap::new(prefix)
        });

        let mut retries_left = retries_left_map.get(&quiz_id).unwrap_or(3);

        if retries_left == 0 {
            env::panic_str("You can no longer solve this quiz. You are out of tries.");
        }

        let answer_hash = format!("{:x}", Sha256::digest(answer.as_bytes()));

        if answer_hash == quiz.correct_hash {
            solved_quizzes_set.insert(&quiz_id);
            self.solved_quizzes.insert(&account_id, &solved_quizzes_set);

            let amount = quiz.max_prize_amount / (4 - retries_left) as u128;

            Promise::new(account_id.clone()).transfer(amount);

            return format!("Your answer is correct. You've got {} yoctoNEAR", amount);
        } else {
            retries_left -= 1;

            retries_left_map.insert(&quiz_id, &retries_left);

            self.retries_left.insert(&account_id, &retries_left_map);

            if retries_left == 0 {
                return format!("The answer is not right, you are out of tries");
            }

            return format!("The answer is not right. You have {} retries left", retries_left);
        }
    }

    pub fn create_quiz(&mut self, question: String, correct_hash: String, max_prize_amount: String, publish: bool) -> QuizId {
        self.check_owner();

        let status = if publish { QuizStatus::Published } else { QuizStatus::Unpublished };
        let quiz_id = self.current_quiz_id;
        let existing_quiz = self.quizzes.insert(&quiz_id, &Quiz {
            question, correct_hash, max_prize_amount: max_prize_amount.parse::<u128>().unwrap(), status
        });

        assert!(existing_quiz.is_none(), "Quiz with the same quiz_id already exists");

        if publish {
            self.published_quiz_ids.insert(&quiz_id);
        }

        self.current_quiz_id += 1;

        quiz_id
    }

    pub fn get_quiz_status(&self, quiz_id: QuizId) -> Option<QuizStatus> {
        if let Some(quiz) = self.quizzes.get(&quiz_id) {
            return Some(quiz.status)
        }

        None
    }

    pub fn publish_quiz(&mut self, quiz_id: QuizId) {
        self.check_owner();

        let mut quiz = self.quizzes.get(&quiz_id).expect("No such quiz found");
        if quiz.status == QuizStatus::Unpublished {
            quiz.status = QuizStatus::Published;
            self.published_quiz_ids.insert(&quiz_id);
        }

        self.quizzes.insert(&quiz_id, &quiz);
    }

    pub fn get_published_quizzes(&self) -> PublishedQuizzes {
        let quiz_ids = self.published_quiz_ids.to_vec();
        let mut quizzes = vec![];
        for quiz_id in quiz_ids {
            let quiz = self.quizzes.get(&quiz_id).unwrap_or_else(|| env::panic_str("Cannot load quiz"));
            let json_quiz = JsonQuiz {
                quiz_id,
                question: quiz.question,
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
        let quiz_id = contract.create_quiz("What is the capital of France".to_owned(), "5dd272b4f316b776a7b8e3d0894b37e1e42be3d5d3b204b8a5836cc50597a6b1".to_owned(), "1".to_owned(), true);

        let published_quizzes = contract.get_published_quizzes();
        assert_eq!(published_quizzes.quizzes.len(), 1);
        assert_eq!(published_quizzes.quizzes[0].quiz_id, quiz_id);
    }

    #[test]
    fn create_new_quiz() {
        let account_id = AccountId::new_unchecked("bob.near".to_owned());

        let context = get_context(account_id.clone(), false);
        testing_env!(context.build());

        let mut contract = QuizContract::new(account_id);
        let quiz_id = contract.create_quiz("What is the capital of France".to_owned(), "5dd272b4f316b776a7b8e3d0894b37e1e42be3d5d3b204b8a5836cc50597a6b1".to_owned(), "1".to_owned(), true);

        let quiz = contract.quizzes.get(&quiz_id).unwrap();
        assert_eq!(quiz.question, "What is the capital of France".to_owned());
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

        contract.create_quiz("What is the capital of France".to_owned(), "5dd272b4f316b776a7b8e3d0894b37e1e42be3d5d3b204b8a5836cc50597a6b1".to_owned(), "1".to_owned(), true);
    }

    #[test]
    fn get_publish_status() {
        let account_id = AccountId::new_unchecked("bob.near".to_owned());

        let context = get_context(account_id.clone(), false);
        testing_env!(context.build());

        let mut contract = QuizContract::new(account_id);

        let quiz_id = contract.create_quiz("What is the capital of France".to_owned(), "5dd272b4f316b776a7b8e3d0894b37e1e42be3d5d3b204b8a5836cc50597a6b1".to_owned(), "1".to_owned(), false);
        assert_eq!(contract.get_quiz_status(quiz_id).unwrap(), QuizStatus::Unpublished);

        let quiz_id = contract.create_quiz("What is the capital of France".to_owned(), "5dd272b4f316b776a7b8e3d0894b37e1e42be3d5d3b204b8a5836cc50597a6b1".to_owned(), "1".to_owned(), true);
        assert_eq!(contract.get_quiz_status(quiz_id).unwrap(), QuizStatus::Published);
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
        let quiz_id = contract.create_quiz("What is the capital of France".to_owned(), "5dd272b4f316b776a7b8e3d0894b37e1e42be3d5d3b204b8a5836cc50597a6b1".to_owned(), "1".to_owned(), false);

        contract.submit_answer(quiz_id, "Paris".to_owned());
    }

    #[test]
    fn submit_correct_answer_to_published() {
        let account_id = AccountId::new_unchecked("bob.near".to_owned());

        let context = get_context(account_id.clone(), false);
        testing_env!(context.build());

        let mut contract = QuizContract::new(account_id.clone());
        let quiz_id = contract.create_quiz("What is the capital of France".to_owned(), "5dd272b4f316b776a7b8e3d0894b37e1e42be3d5d3b204b8a5836cc50597a6b1".to_owned(), "1".to_owned(), true);

        contract.submit_answer(quiz_id.clone(), "Paris".to_owned());

        assert_eq!(contract.solved_quizzes.get(&account_id).unwrap().contains(&quiz_id), true);
    }

    #[test]
    fn submit_incorrect_answer_to_published() {
        let account_id = AccountId::new_unchecked("bob.near".to_owned());

        let context = get_context(account_id.clone(), false);
        testing_env!(context.build());

        let mut contract = QuizContract::new(account_id.clone());
        let quiz_id = contract.create_quiz("What is the capital of France".to_owned(), "5dd272b4f316b776a7b8e3d0894b37e1e42be3d5d3b204b8a5836cc50597a6b1".to_owned(), "1".to_owned(), true);

        contract.submit_answer(quiz_id.clone(), "Berlin".to_owned());

        assert_eq!(contract.retries_left.get(&account_id).unwrap().get(&quiz_id).unwrap(), 2);
        contract.submit_answer(quiz_id.clone(), "Madrid".to_owned());
        assert_eq!(contract.retries_left.get(&account_id).unwrap().get(&quiz_id).unwrap(), 1);
    }

    #[test]
    fn publish_unpublished_quiz() {
        let account_id = AccountId::new_unchecked("bob.near".to_owned());

        let context = get_context(account_id.clone(), false);
        testing_env!(context.build());

        let mut contract = QuizContract::new(account_id.clone());
        let quiz_id = contract.create_quiz("What is the capital of France".to_owned(), "5dd272b4f316b776a7b8e3d0894b37e1e42be3d5d3b204b8a5836cc50597a6b1".to_owned(), "1".to_owned(), false);

        assert_eq!(contract.quizzes.get(&quiz_id).unwrap().status, QuizStatus::Unpublished);
        contract.publish_quiz(quiz_id.clone());
        assert_eq!(contract.quizzes.get(&quiz_id).unwrap().status, QuizStatus::Published);
    }
}