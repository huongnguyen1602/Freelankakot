#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod freelancer { 
 
    use ink::storage::Mapping;
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
        owner_job : Mapping<(AccountId, OnwerRole), JobId>,
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
        FINISH, 
    }

    #[derive(scale::Decode, scale::Encode, Default, Debug, PartialEq, Clone, Copy)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub enum OnwerRole {
        #[default]
        INDIVIDUAL, 
        ENTERPRISE(OnwerRoleInEnterprise),
    }


    #[derive(scale::Decode, scale::Encode, Default, Debug, PartialEq, Clone, Copy)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub enum OnwerRoleInEnterprise {
        #[default]
        TEAMLEAD,
        ACCOUNTANT, //có thể bổ sung các role khác
    }

    #[derive(scale::Decode, scale::Encode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo)
    )]
    pub enum JobError {
        CreatedJob,
        NotExisted, // Job không tồn tại
        Taked, //đã có người nhận
        NotTakeThisJob,
        Submited, //đã submit 
        Proccesing, //đang có người làm
        CurrentJobIncomplete, //hoàn thành job hiện tại đã
        JobInvalid,
        Finish, //job đã hoàn thành
    }


    impl Freelancer {
        /// Constructor that initializes the `bool` value to the given `init_value`.
        #[ink(constructor)]
        pub fn new() -> Self {
            Self::default()
        }

        #[ink(message, payable)]
        pub fn create(&mut self, name: String, description: String, role: OnwerRole) -> Result<(), JobError> {
            let budget = self.env().transferred_value();
            let caller = self.env().caller();

            let job = Job {
                name: name, 
                description: description, 
                budget: budget, 
                status: Status::default(),
                result: None
            };
            // mỗi tài khoản chỉ push 1 công việc
            if self.owner_job.get((caller, role.clone())).is_some() {return Err(JobError::CreatedJob)}; 
            // job đầu đánh số 0, các job tiếp theo thì cộng 1 vào
            self.jobs.insert(self.current_job_id, &job); 
            self.owner_job.insert((caller, role), &self.current_job_id);
            self.current_job_id = self.current_job_id + 1; 
            
            Ok(())

        }

        // có thể tùy chỉnh thêm lọc công việc theo status hoặc theo owner hoặc theo freelancer
        // freelancer có thể apply job open va reopen
        #[ink(message)]
        pub fn get_jobs_with_status (&self, status: Status, owner: Option<AccountId>) -> Vec<Job> {
            let mut jobs = Vec::new();
            for index in 0..self.current_job_id {
                let job = self.jobs.get(index).unwrap();
                if job.status == status {
                    jobs.push(self.jobs.get(index).unwrap());
                }
            };
            jobs
        }
        
        #[ink(message)]
        pub fn obtain(&mut self, job_id: JobId) -> Result<(), JobError>{
            // kiểm tra id job có lớn hơn hoặc curren_id hay không (curren_id là id của job tiếp theo)
            if job_id >= self.current_job_id {return Err(JobError::NotExisted)};

            let caller = self.env().caller();

            // check job assigned or not
            let a = self.assigned_job.get(job_id);

            //Chỗ này cần chỉnh lại là is_some
            if a.is_some() {
                return Err(JobError::Proccesing)
            }
                
            // check caller doing job or not
            let doing_job = self.doing_job.get(caller); 
            if doing_job.is_some() {
                return Err(JobError::CurrentJobIncomplete)
            }

            // update job status
            let mut job = self.jobs.get(job_id).unwrap(); 

            
            // assert!(job.status==Status::OPEN || job.status==Status::REOPEN);
            // Kiểm tra trạng thái open hay reopen;
            if job.status != Status::OPEN && job.status != Status::REOPEN {
                return Err(JobError::JobInvalid)
            }

            job.status = Status::DOING;

            // insert assigned_job
            self.assigned_job.insert(job_id, &caller);
            // insert doing_job
            self.doing_job.insert(caller, &job_id);
            
            // chỉnh lại trạng thái job
            self.jobs.insert(job_id, &job);

            Ok(())

        }



        #[ink(message)]
        pub fn submit(&mut self, job_id: JobId, result: String) -> Result<(), JobError>{
            // kiểm tra id job có lớn hơn hoặc curren_id hay không (curren_id là id của job tiếp theo)
            if job_id >= self.current_job_id {return Err(JobError::NotExisted)};

            let caller = self.env().caller();
            // kiểm tra người đó có apply job đó hay không, không cần kiểm tra none vì job <= current_id
            if self.assigned_job.get(job_id) == None || self.assigned_job.get(job_id).unwrap() != caller {return Err(JobError::NotTakeThisJob)};

            let mut job = self.jobs.get(job_id).unwrap();
            //job phải ở trạng thái doing mới submit được
            if job.status == Status::DOING || job.status == Status::REOPEN {
                job.result = Some(result);

                job.status = Status::REVIEW;

                self.jobs.insert(job_id, &job);
            } else if job.status == Status::REVIEW {
                return Err(JobError::Submited)
            } else {
                return Err(JobError::Finish)
            }
            Ok(())
        }

        #[ink(message)]
        pub fn reject(&mut self, job_id: JobId, role: OnwerRole) -> Result<(), JobError>{

            // kiểm tra id job có lớn hơn curren_id hay không
            if job_id >= self.current_job_id {return Err(JobError::NotExisted)};

            let caller = self.env().caller();
            // kiểm tra người đó có phải là giao job đó hay không, không cần kiểm tra none vì job <= current_id
            if self.owner_job.get((caller, role)) == None ||
            self.owner_job.get((caller, role)).unwrap() != job_id {
                return Err(JobError::NotExisted)
            };

            let mut job = self.jobs.get(job_id).unwrap();
            //job phải ở trạng thái review mới reject được
            if job.status == Status::REVIEW {

                job.status = Status::REOPEN;

                // xóa kết quả của người làm trước
                job.result = None;

                self.jobs.insert(job_id, &job);
            } else if job.status == Status::DOING || job.status == Status::REOPEN {
                return Err(JobError::Proccesing)
            } else {
                return Err(JobError::Finish)
            }

            Ok(())

        }

        #[ink(message)]
        pub fn aproval(&mut self, job_id: JobId, role: OnwerRole) -> Result<(), JobError>{
            if job_id >= self.current_job_id {return Err(JobError::NotExisted)};

            let caller = self.env().caller();
            // kiểm tra người đó có phải là giao job đó hay không, không cần kiểm tra none vì job <= current_id
            if self.owner_job.get((caller, role)) == None ||
            self.owner_job.get((caller, role)).unwrap() != job_id {
                return Err(JobError::NotExisted)
            };

            let mut job = self.jobs.get(job_id).unwrap();
            //job phải ở trạng thái review mới reject được
            if job.status == Status::REVIEW {

                // chỉnh và update trạng thái finish của job
                job.status = Status::FINISH;
                self.jobs.insert(job_id, &job);

                //remove để có thể up việc mới và freelancer có thể nhận việc mới
                self.owner_job.remove((caller, role));
                let freelancer = self.assigned_job.get(job_id).unwrap();
                self.doing_job.remove(freelancer);
            } else if job.status == Status::DOING || job.status == Status::REOPEN {
                return Err(JobError::Proccesing)
            } else {
                return Err(JobError::Finish)
            }
            // chuyển tiền cho người nhận.
            self.env().transfer(self.assigned_job.get(job_id).unwrap(), job.budget);
            Ok(())
        }        
    }



    // viết test
    #[cfg(test)]
    mod tests {
        use super::*;

        #[ink::test]
        fn new_works() {
            let mut new_freelancer = Freelancer::new();
            assert_eq!(new_freelancer.current_job_id, 0);
            
            // role cá nhân hoặc role doanh nghiệp
            let individual_role = OnwerRole::INDIVIDUAL;
            // let enterprise_role =OnwerRole::ENTERPRISE(OnwerRoleInEnterprise::TEAMLEAD);


            new_freelancer.create("TaskOne".to_string(), "Submit on one week".to_string(), individual_role);
            assert_eq!(new_freelancer.current_job_id, 1);
            assert_eq!(new_freelancer.jobs.get(1).unwrap().name, "TaskOne".to_string());
            assert_eq!(new_freelancer.jobs.get(1).unwrap().description, "Submit on one week".to_string());
            assert_eq!(new_freelancer.jobs.get(1).unwrap().result, None);
            assert_eq!(new_freelancer.jobs.get(1).unwrap().status, Status::OPEN);
            assert_eq!(new_freelancer.jobs.get(1).unwrap().budget, 0); //buget khi đưa vào mặc định là 0
            

        }
    }
}
