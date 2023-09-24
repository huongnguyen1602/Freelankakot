#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod freelancer { 
 
    use ink::{storage::Mapping};
    use ink::prelude::string::String;
    use ink::prelude::vec::Vec;

    pub type JobId = u128;


    /// Defines the storage of your contract.
    /// Add new fields to the below struct in order
    /// to add new static storage fields to your contract.
    #[ink(storage)]
    #[derive(Default)]
    pub struct Freelancer {
        jobs : Mapping<JobId, Job>,
        owner_job : Mapping<AccountId, JobId>,
        doing_job: Mapping<AccountId, JobId>,

        assigned_job: Mapping<JobId, AccountId>,
        current_job_id: JobId,
    }


    #[derive(scale::Decode, scale::Encode, Default, Debug)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    
    pub struct Job {
        name: String, 
        description: String,
        result: Option<String>,
        status: Status,
        budget: Balance,
    }

    #[derive(scale::Decode, scale::Encode, Default, Debug, PartialEq)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub enum Status {
        #[default]
        OPEN, 
        DOING, 
        REVIEW, 
        REOPEN, 
        FINISH
    }

    #[derive(scale::Decode, scale::Encode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo)
    )]
    pub enum JobError {
        JobInvalid
    }


    impl Freelancer {
        /// Constructor that initializes the `bool` value to the given `init_value`.
        #[ink(constructor)]
        pub fn new() -> Self {
            Self::default()
        }

        #[ink(message, payable)]
        pub fn create(&mut self, name: String, description: String) {
            let budget = self.env().transferred_value();
            let caller = self.env().caller();

            let job = Job {
                name: name, 
                description: description, 
                budget: budget, 
                status: Status::default(),
                result: None
            };

            self.jobs.insert(self.current_job_id, &job);
            self.owner_job.insert(caller, &self.current_job_id);
            self.current_job_id = self.current_job_id + 1;

        }


        #[ink(message)]
        pub fn get_open_jobs(&self) -> Vec<Job> {
            let mut jobs = Vec::new();
            for index in 0..self.current_job_id {
                let job = self.jobs.get(index).unwrap();
                if job.status == Status::OPEN {
                    jobs.push(self.jobs.get(index).unwrap());
                }
            }

            jobs
        }
        
        #[ink(message)]
        pub fn obtain(&mut self, job_id: JobId) -> Result<(), JobError>{
            let caller = self.env().caller();

            // check job assigned or not
            let a = self.assigned_job.get(job_id);

            if a == None {
                return Err(JobError::JobInvalid)
            }
                
            // check caller doing job or not
            let doing_job = self.doing_job.get(caller); 
            if doing_job == None {
                return Err(JobError::JobInvalid)
            }

            // update job status
            let mut job = self.jobs.get(job_id).unwrap(); 
            assert_eq!(job.status, Status::OPEN, "");

            if job.status == Status::OPEN {
                return Err(JobError::JobInvalid)
            }

            job.status = Status::DOING;

            // insert assigned_job
            self.assigned_job.insert(job_id, &caller);
            // insert doing_job
            self.doing_job.insert(caller, &job_id);

            Ok(())

        }



        #[ink(message)]
        pub fn submit(&self, job_id: JobId, result: String) {

            // check exits job or not

            // check freelancer doing the job id or not 

            // 

        }

        #[ink(message)]
        pub fn reject(&self, job_id: JobId) {}

        #[ink(message)]
        pub fn aproval(&self, job_id: JobId) {}


 
    }




    /// This is how you'd write end-to-end (E2E) or integration tests for ink! contracts.
    ///
    /// When running these you need to make sure that you:
    /// - Compile the tests with the `e2e-tests` feature flag enabled (`--features e2e-tests`)
    /// - Are running a Substrate node which contains `pallet-contracts` in the background
    #[cfg(all(test, feature = "e2e-tests"))]
    mod e2e_tests {
        /// Imports all the definitions from the outer scope so we can use them here.
        use super::*;

        /// A helper function used for calling contract messages.
        use ink_e2e::build_message;

        /// The End-to-End test `Result` type.
        type E2EResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

        /// We test that we can upload and instantiate the contract using its default constructor.
        #[ink_e2e::test]
        async fn default_works(mut client: ink_e2e::Client<C, E>) -> E2EResult<()> {
            // Given
            let constructor = HelloInkRef::default();

            // When
            let contract_account_id = client
                .instantiate("hello_ink", &ink_e2e::alice(), constructor, 0, None)
                .await
                .expect("instantiate failed")
                .account_id;

            // Then
            let get = build_message::<HelloInkRef>(contract_account_id.clone())
                .call(|hello_ink| hello_ink.get());
            let get_result = client.call_dry_run(&ink_e2e::alice(), &get, 0, None).await;
            assert!(matches!(get_result.return_value(), false));

            Ok(())
        }

        /// We test that we can read and write a value from the on-chain contract contract.
        #[ink_e2e::test]
        async fn it_works(mut client: ink_e2e::Client<C, E>) -> E2EResult<()> {
            // Given
            let constructor = HelloInkRef::new(false);
            let contract_account_id = client
                .instantiate("hello_ink", &ink_e2e::bob(), constructor, 0, None)
                .await
                .expect("instantiate failed")
                .account_id;

            let get = build_message::<HelloInkRef>(contract_account_id.clone())
                .call(|hello_ink| hello_ink.get());
            let get_result = client.call_dry_run(&ink_e2e::bob(), &get, 0, None).await;
            assert!(matches!(get_result.return_value(), false));

            // When
            let flip = build_message::<HelloInkRef>(contract_account_id.clone())
                .call(|hello_ink| hello_ink.flip());
            let _flip_result = client
                .call(&ink_e2e::bob(), flip, 0, None)
                .await
                .expect("flip failed");

            // Then
            let get = build_message::<HelloInkRef>(contract_account_id.clone())
                .call(|hello_ink| hello_ink.get());
            let get_result = client.call_dry_run(&ink_e2e::bob(), &get, 0, None).await;
            assert!(matches!(get_result.return_value(), true));

            Ok(())
        }
    }
 
}