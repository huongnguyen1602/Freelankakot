#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod freelancer { 
 
    use ink::storage::Mapping;
    use ink::prelude::string::String;
    use ink::prelude::vec::Vec;

    pub type JobId = u128;

    #[ink(storage)]
    #[derive(Default)]
    pub struct Freelancer {
        jobs : Mapping<JobId, Job>,
        owner_job : Mapping<(AccountId, AccountRole), JobId>,
        doing_job: Mapping<AccountId, JobId>,
        assigned_job: Mapping<JobId, AccountId>,
        current_job_id: JobId,
        account_role: Mapping<AccountId, AccountRole>, //bỏ cái này vì trong personal account đã có thông tin rồi
        personal_account_info: Mapping<AccountId, UserInfo>,
        all_created_projects: Mapping<AccountId, Vec<(JobId, Status)>>,
        all_obtain: Mapping<AccountId, Vec<(JobId, Status)>>
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
        // fee_percentage: Balance, //Phí để up việc
        // duration: BlockNumber,
        // dealine: Option<BlockNumber>, 
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
    pub enum AccountRole {
        #[default]
        INDIVIDUAL, // khách hàng cá nhân
        ENTERPRISE(OnwerRoleInEnterprise), // khác hàng doanh nghiệp
        FREELANCER, 
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


    #[derive(scale::Decode, scale::Encode, Default, Debug)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct UserInfo{
        name: String,
        detail: String, //liên kết đến IPFS lưu trữ thông tin cá nhân
        role: AccountRole, // vai trò
        rating_points: u32, // điểm dánh giá
    }


    

    #[derive(scale::Decode, scale::Encode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo)
    )]
    pub enum JobError {
        Registered, //đã đăng kí tài khoản (đăng kí), không đăng kí nữa
        NotRegistered, // chưa đăng kí tài khoản.
        NotJobAssigner, // bạn không phải là người giao việc
        NotFreelancer, // bạn không phải là freelancer
        CreatedJob, //Job đã tạo
        NotExisted, // Job không tồn tại
        NotTaked, // chưa có người nhận job
        Taked, //đã có người nhận
        NotTakeThisJob, // bạn ko có nhận job này
        NotAssignThisJob, //bạn ko phải là người giao việc này
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

        #[ink(message)]
        pub fn register(&mut self, role: AccountRole) -> Result<(), JobError>{
            let caller = self.env().caller();
            match self.account_role.get(caller) {
                None => self.account_role.insert(caller, &role),
                _ => return Err(JobError::Registered),
            };
            Ok(())
        }

        //check role tài khoản
        #[ink(message, payable)]
        pub fn check_role(&self) -> Option<AccountRole> {
            let caller = self.env().caller();
            self.account_role.get(caller)
        }

        #[ink(message, payable)]
        pub fn create(&mut self, name: String, description: String) -> Result<(), JobError> {
            match self.check_role() {
                None => return Err(JobError::NotRegistered),
                Some(AccountRole::FREELANCER) => return Err(JobError::NotJobAssigner),
                _ => ()
            }
            let caller = self.env().caller();
            let budget = self.env().transferred_value();
            let role = self.account_role.get(caller).unwrap();
            let job = Job {
                name: name, 
                description: description, 
                budget: budget, 
                status: Status::default(),
                result: None
            };
            // mỗi tài khoản chỉ push 1 công việc
            if self.owner_job.get((caller, role)).is_some() {return Err(JobError::CreatedJob)}; 
            // job đầu đánh số 0, các job tiếp theo thì cộng 1 vào
            self.jobs.insert(self.current_job_id, &job); 
            self.owner_job.insert((caller, role), &self.current_job_id);
            self.current_job_id = self.current_job_id + 1; 
            
            Ok(())

        }

        // có thể tùy chỉnh thêm lọc công việc theo status hoặc theo owner hoặc theo freelancer
        // lọc theo owner khi 1 owner có thể tạo nhiều công việc (chưa làm)
        // freelancer có thể apply job open va reopen
        #[ink(message)]
        pub fn get_jobs_with_status (&self, status: Status) -> Vec<Job> {
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
            //kiểm tra role
            match self.check_role() {
                None => return Err(JobError::NotRegistered),
                Some(AccountRole::FREELANCER) => (),
                _ => return Err(JobError::NotFreelancer),
            }

            // kiểm tra người nhận có đang làm job nào hay không
            let caller = self.env().caller();
            if self.doing_job.get(caller).is_some() {
                return Err(JobError::CurrentJobIncomplete)
            }

            // check job assigned or not
            // Chỗ này cần chỉnh lại là is_some
            // if self.assigned_job.get(job_id).is_some() {
            //     return Err(JobError::Proccesing)
            // }

            // kiểm tra job hợp lệ hay không? và tiến hành update
            let mut job = self.jobs.get(job_id).unwrap(); 

            match job.status {
                Status::DOING => return Err(JobError::Proccesing), //đang trong quá trình thực  hiện
                Status::REVIEW => return Err(JobError::Proccesing),
                Status::OPEN => (),
                Status::REOPEN => (),
                Status::FINISH => return Err(JobError::Finish) //job đã kết thúc

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
            //kiểm tra role
            let caller = self.env().caller(); 
            match self.check_role() {
                None => return Err(JobError::NotRegistered),
                Some(AccountRole::FREELANCER) => (),
                _ => return Err(JobError::NotFreelancer),
            }
            // kiểm tra người đó có apply job đó hay không, chú ý kiểm tra None
            if self.assigned_job.get(job_id) == None || self.assigned_job.get(job_id).unwrap() != caller {return Err(JobError::NotTakeThisJob)};

            let mut job = self.jobs.get(job_id).unwrap();
            //job phải ở trạng thái doing mới submit được
            match job.status {
                Status::OPEN => return Err(JobError::NotTaked),
                Status::REOPEN => return Err(JobError::NotTaked),
                Status::REVIEW => return Err(JobError::Submited),
                Status::FINISH => return Err(JobError::Finish),
                Status::DOING => {
                    job.result = Some(result);

                    job.status = Status::REVIEW;

                    self.jobs.insert(job_id, &job);
                }
            }
            Ok(())
        }

        #[ink(message)]
        pub fn reject(&mut self, job_id: JobId) -> Result<(), JobError>{

            // kiểm tra id job có lớn hơn curren_id hay không
            if job_id >= self.current_job_id {return Err(JobError::NotExisted)};
            // kiểm tra role
            let caller = self.env().caller();
            match self.check_role() {
                None => return Err(JobError::NotRegistered),
                Some(AccountRole::FREELANCER) => return Err(JobError::NotJobAssigner),
                _ => (),
            }
            let role = self.account_role.get(caller).unwrap();
            // kiểm tra người đó có phải là giao job đó hay không, không cần kiểm tra none vì khi có id job thì sẽ
            // chắc chắn có người giao job đó.
            if self.owner_job.get((caller, role)).unwrap() != job_id {
                return Err(JobError::NotAssignThisJob)
            };

            let mut job = self.jobs.get(job_id).unwrap();
            //job phải ở trạng thái review mới reject được
            match job.status {
                Status::OPEN => return Err(JobError::NotTaked),
                Status::REOPEN => return Err(JobError::NotTaked),
                Status::DOING => return Err(JobError::Proccesing),
                Status::FINISH => return Err(JobError::Finish),
                Status::REVIEW => {
                    job.status = Status::REOPEN;
                    // xóa kết quả của người làm trước
                    job.result = None;
                    self.jobs.insert(job_id, &job);
                    //xóa người đang làm job để có thể nhận job mới
                    let freelancer = self.assigned_job.get(job_id).unwrap();
                    // self.assigned_job.remove(job_id);
                    self.doing_job.remove(freelancer);
                }
            }

            Ok(())

        }

        #[ink(message)]
        pub fn aproval(&mut self, job_id: JobId) -> Result<(), JobError>{
            // kiểm tra id job có lớn hơn curren_id hay không
            if job_id >= self.current_job_id {return Err(JobError::NotExisted)};
            // kiểm tra role
            let caller = self.env().caller();
            match self.check_role() {
                None => return Err(JobError::NotRegistered),
                Some(AccountRole::FREELANCER) => return Err(JobError::NotJobAssigner),
                _ => (),
            }
            let role = self.account_role.get(caller).unwrap();
            // kiểm tra người đó có phải là giao job đó hay không, không cần kiểm tra none vì khi có id job thì sẽ
            // chắc chắn có người giao job đó.
            if self.owner_job.get((caller, role)).unwrap() != job_id {
                return Err(JobError::NotAssignThisJob)
            };

            let mut job = self.jobs.get(job_id).unwrap();
            //job phải ở trạng thái review mới aproval được
            match job.status {
                Status::OPEN => return Err(JobError::NotTaked),
                Status::REOPEN => return Err(JobError::NotTaked),
                Status::DOING => return Err(JobError::Proccesing),
                Status::FINISH => return Err(JobError::Finish),
                Status::REVIEW => {
                    job.status = Status::FINISH;
                    self.jobs.insert(job_id, &job);
                    //xóa người đang làm job để có thể nhận job mới
                    let freelancer = self.assigned_job.get(job_id).unwrap();
                    // self.assigned_job.remove(job_id);
                    self.doing_job.remove(freelancer);
                    // thanh toán tiền job
                    let budget = job.budget;
                    self.env().transfer(freelancer, budget);
                }
            }

            Ok(())
        } 

        #[ink(message)]
        pub fn check_balance_of_contract(&self) -> Balance {
            self.env().balance()
        }
    }



    // viết test
    #[cfg(test)]
    mod tests {
        use super::*;

        #[ink::test]
        fn new_works() {
            // let mut new_freelancer = Freelancer::new();
            // assert_eq!(new_freelancer.current_job_id, 0);
            
            // // role cá nhân hoặc role doanh nghiệp
            // let individual_role = OnwerRole::INDIVIDUAL;
            // // let enterprise_role =OnwerRole::ENTERPRISE(OnwerRoleInEnterprise::TEAMLEAD);


            // new_freelancer.create("TaskOne".to_string(), "Submit on one week".to_string(), individual_role);
            // assert_eq!(new_freelancer.current_job_id, 1);
            // assert_eq!(new_freelancer.jobs.get(1).unwrap().name, "TaskOne".to_string());
            // assert_eq!(new_freelancer.jobs.get(1).unwrap().description, "Submit on one week".to_string());
            // assert_eq!(new_freelancer.jobs.get(1).unwrap().result, None);
            // assert_eq!(new_freelancer.jobs.get(1).unwrap().status, Status::OPEN);
            // assert_eq!(new_freelancer.jobs.get(1).unwrap().budget, 0); //buget khi đưa vào mặc định là 0
            

        }
    }
}
