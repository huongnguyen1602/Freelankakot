#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod freelancer { 

    use ink::storage::Mapping;
    use ink::prelude::string::String;
    use ink::prelude::vec::Vec;

    pub type JobId = u128;
    const FEE_PERCENTAGE: u8 = 3;
    #[ink(storage)]
    #[derive(Default)]
    pub struct Freelancer {
        jobs : Mapping<JobId, Job>, //map jobID đến job: luôn là trạng thái cuối cùng của job, như vậy job reopen sẽ ko lưu người làm trước, phần đó lưu trong unsuccessful_job kèm đánh giá
        current_job_id: JobId,
        personal_account_info: Mapping<AccountId, UserInfo>,
        owner_jobs : Mapping<AccountId, Vec<JobId>>,
        freelancer_jobs: Mapping<AccountId, Vec<JobId>>,
        successful_jobs: Mapping<(AccountId, AccountId, JobId), Option<String>>, //(AccountId: người tạo job, AccountId: người nhận job, jobID) => Option<String> là review có thể là string review hoặc là link đến ipfs (do freelancer và người giao việc đánh giá)
        unsuccesful_jobs: Mapping<(AccountId, AccountId, JobId), Option<String>>   //(AccountId: người tạo job, AccountId: Người nhận job, jobID) => Option<String> là report có thể là string nhận xét hoặc là link đến ipfs (do freelancer và người giao việc đánh giá)
        // còn phần đánh giá cá nhân thông qua raiting point trong user_info
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
        fee_percentage: u8, //Phần trăm tiền phí
        start_time: Timestamp, //thời gian bắt đầu tính từ lúc khởi tạo công việc
        end_time: Timestamp, //thời gian kết thúc = thời gian bắt đầu + duration người dùng nhập sẽ tính bằng ngày. (thời gian này bao gồm khởi tạo công việc và xét duyệt quá thời hạn người tạo phải hủy job tránh tình trạng treo người làm xong ko được nghiệm thu)
        person_create: Option<AccountId>, // vì có trait default nên để option cho dễ
        person_obtain: Option<AccountId>
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
        UNQUALIFIED, //chưa đạt yêu cầu, reject => unqualifieds => freelancer nếu đồng ý thì reopen nếu ko thì complaint
        REOPEN, 
        FINISH, 
        CANCELED,
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


    #[derive(scale::Decode, scale::Encode, Default, Debug, Clone)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct UserInfo{
        name: String,
        detail: String, //liên kết đến IPFS lưu trữ thông tin cá nhân
        role: AccountRole, // vai trò
        successful_jobs_and_all_jobs: (u32, u32), //số lượng công việc thành công trên số lượng công việc đã tạo (client) hoặc đã nhận (freelancer).
        rating_points: i32, // điểm dánh giá có thể âm nên để i32
    }

    #[derive(scale::Decode, scale::Encode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo)
    )]
    pub enum JobError {
        //Lỗi liên quan tới đăng kí tài khoản
        Registered, //đã đăng kí tài khoản (đăng kí), không đăng kí nữa
        NotRegistered, // chưa đăng kí tài khoản.

        //Lỗi role
        NotJobAssigner, // bạn không phải là người giao việc
        NotFreelancer, // bạn không phải là freelancer

        //Lỗi check job
        NotExisted, // Job không tồn tại
        NotTaked, // chưa có người nhận job
        Taked, //đã có người nhận
        NotTakeThisJob, // bạn ko có nhận job này
        NotAssignThisJob, //bạn ko phải là người giao việc này

        //lỗi liên quan đến thời gian hoàn thành job
        OutOfDate,

        //Lỗi liên quan tới status job
        Submited, //đã submit 
        Proccesing, //đang có người làm
        CurrentJobIncomplete, //hoàn thành job hiện tại đã
        Finish, //job đã kết thúc (hoàn thành hoặc bị hủy)
    }


    impl Freelancer {
        /// Constructor that initializes the `bool` value to the given `init_value`.
        #[ink(constructor)]
        pub fn new() -> Self {
            Self::default()
        }

        #[ink(message)]
        pub fn register(&mut self, name: String, detail: String, role: AccountRole) -> Result<(), JobError>{
            let caller = self.env().caller();
            let caller_info = UserInfo {
                name: name,
                detail: detail,
                role: role,
                successful_jobs_and_all_jobs: (0,0),
                rating_points: 0
            };
            match self.personal_account_info.get(caller) {
                None => self.personal_account_info.insert(caller, &caller_info),
                _ => return Err(JobError::Registered),
            };
            Ok(())
        }

        // //check role tài khoản
        // #[ink(message)]
        // pub fn get_role_of(&self, account: AccountId) -> AccountRole {
        //     self.personal_account_info.get(account).unwrap().role
        // }

        //show thông tin account
        #[ink(message)]
        pub fn get_account_info(&self, caller: AccountId) -> Option<UserInfo> {
            self.personal_account_info.get(caller)
        }
        // show toàn bộ công việc của người tạo
        #[ink(message)]
        pub fn get_job_id_of_onwer(&self, owner: AccountId) -> Option<Vec<JobId>> {
            self.owner_jobs.get(owner)
        }
        //show toàn bộ công việc của người nhận
        #[ink(message)]
        pub fn get_job_id_of_freelancer(&self, owner: AccountId) -> Option<Vec<JobId>> {
            self.freelancer_jobs.get(owner)
        } 

        //show chi tiết công việc 
        #[ink(message)]
        pub fn show_detail_job_of_id(&self, job_id: JobId) -> Option<Job> {
            self.jobs.get(job_id)
        }

        #[ink(message, payable)]
        pub fn create(&mut self, name: String, description: String, duration: u64) -> Result<(), JobError> { //duration là nhập số ngày chú ý timestamp tính theo mili giây
            let caller = self.env().caller();
            let caller_info = self.personal_account_info.get(caller);
            match caller_info.clone() {
                None => return Err(JobError::NotRegistered), //check đăng kí chưa
                Some(user_info) => { //check role đúng chưa
                    if user_info.role == AccountRole::FREELANCER {return Err(JobError::NotJobAssigner)}
                },
            }
            let budget = self.env().transferred_value();
            let start_time = self.env().block_timestamp();
            let job = Job {
                name: name, 
                description: description,
                result: None,
                status: Status::default(),
                budget: budget,
                fee_percentage: FEE_PERCENTAGE,
                start_time: start_time,
                end_time: start_time + duration * 24 * 60 * 60 * 1000,
                person_create: Some(caller),
                person_obtain: None,
            };
            let current_id = self.current_job_id;
            self.jobs.insert(current_id, &job);
            // update owner_jobs
            match self.owner_jobs.contains(caller) {
                true => {
                    let mut jobs_of_caller = self.owner_jobs.get(caller).unwrap();
                    jobs_of_caller.push(current_id);
                    self.owner_jobs.insert(caller, &jobs_of_caller);
                }
                false => {
                    let mut jobs_of_caller = Vec::new();
                    jobs_of_caller.push(current_id);
                    self.owner_jobs.insert(caller, &jobs_of_caller);
                }
            }
            self.current_job_id = current_id + 1;
            //update user_info chỗ successful_jobs_and_all_jobs: all_jobs tăng thêm 1
            let mut owner_detail = caller_info.unwrap();
            owner_detail.successful_jobs_and_all_jobs.1 = owner_detail.successful_jobs_and_all_jobs.1 + 1;
            self.personal_account_info.insert(caller, &owner_detail);
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
            let caller = self.env().caller();
            let caller_info = self.personal_account_info.get(caller);
            // kiểm tra đăng kí và role 
            match caller_info.clone() {
                None => return Err(JobError::NotRegistered), //check đăng kí chưa
                Some(user_info) => { //check role đúng chưa
                    if user_info.role != AccountRole::FREELANCER {return Err(JobError::NotFreelancer)}
                },
            }

            let mut job = self.jobs.get(job_id).ok_or(JobError::NotExisted)?;
            //check end_time
            if job.end_time <= self.env().block_timestamp() {return Err(JobError::OutOfDate)};
            match job.status {
                Status::DOING => return Err(JobError::Proccesing),
                Status::REVIEW | Status::UNQUALIFIED => return Err(JobError::Submited),
                Status::CANCELED | Status::FINISH => return Err(JobError::Finish), 
                Status::OPEN | Status::REOPEN => {  
                    //update lại thông tin job
                    job.status = Status::DOING;
                    job.person_obtain = Some(caller);
                    self.jobs.insert(job_id, &job);
                    //update công việc của freelancer
                    match self.freelancer_jobs.contains(caller) {
                        true => {
                            let mut jobs_of_caller = self.freelancer_jobs.get(caller).unwrap();
                            jobs_of_caller.push(job_id);
                            self.freelancer_jobs.insert(caller, &jobs_of_caller);
                        }
                        false => {
                            let mut jobs_of_caller = Vec::new();
                            jobs_of_caller.push(job_id);
                            self.freelancer_jobs.insert(caller, &jobs_of_caller);
                        }
                    }
                    //update user_info chỗ successful_jobs_and_all_jobs: all_jobs tăng thêm 1 
                    let mut freelancer_detail = caller_info.unwrap();
                    freelancer_detail.successful_jobs_and_all_jobs.1 = freelancer_detail.successful_jobs_and_all_jobs.1 + 1;
                    self.personal_account_info.insert(caller, &freelancer_detail);
                }
            }
            Ok(())

        }

        #[ink(message)]
        pub fn submit(&mut self, job_id: JobId, result: String) -> Result<(), JobError>{
            let caller = self.env().caller();
            let caller_info = self.personal_account_info.get(caller);
            // kiểm tra đăng kí và role 
            match caller_info.clone() {
                None => return Err(JobError::NotRegistered), //check đăng kí chưa
                Some(user_info) => { //check role đúng chưa
                    if user_info.role != AccountRole::FREELANCER {return Err(JobError::NotFreelancer)}
                },
            }
            let mut job = self.jobs.get(job_id).ok_or(JobError::NotExisted)?;
            //check end_time
            if job.end_time < self.env().block_timestamp() {return Err(JobError::OutOfDate)};
            //check job đó có phải của mình nhận hay không
            if job.person_obtain.unwrap() != caller {return Err(JobError::NotTakeThisJob)};
            //check job status
            match job.status {
                // Status::OPEN | Status::REOPEN => return Err(JobError::NotTakeThisJob), // không thể xảy ra vì job đã của freelance 
                Status::REVIEW | Status::UNQUALIFIED => return Err(JobError::Submited),
                Status::CANCELED | Status::FINISH => return Err(JobError::Finish), 
                Status::DOING => {  
                    //update lại thông tin job
                    job.result = Some(result);
                    job.status = Status::REVIEW;
                    self.jobs.insert(job_id, &job);
                },
                _ => ()
            }

            Ok(())
        }

        #[ink(message)]
        pub fn reject(&mut self, job_id: JobId) -> Result<(), JobError>{
            let caller = self.env().caller();
            let caller_info = self.personal_account_info.get(caller);
            // kiểm tra đăng kí và role 
            match caller_info.clone() {
                None => return Err(JobError::NotRegistered), //check đăng kí chưa
                Some(user_info) => { //check role đúng chưa
                    if user_info.role == AccountRole::FREELANCER {return Err(JobError::NotJobAssigner)}
                },
            }
            let mut job = self.jobs.get(job_id).ok_or(JobError::NotExisted)?;
            //check end_time
            if job.end_time < self.env().block_timestamp() {return Err(JobError::OutOfDate)};
            //check job đó có phải của mình giao hay không
            if job.person_create.unwrap() != caller {return Err(JobError::NotAssignThisJob)};

            match job.status {
                Status::OPEN | Status::REOPEN => return Err(JobError::NotTaked),
                Status::DOING | Status::UNQUALIFIED => return Err(JobError::Proccesing),
                Status::CANCELED | Status::FINISH => return Err(JobError::Finish), 
                Status::REVIEW => {  
                    //update lại thông tin job để freelancer biết chưa hài lòng
                    job.status = Status::UNQUALIFIED;
                    self.jobs.insert(job_id, &job);
                }
            }
            Ok(())
        }

        #[ink(message)]
        pub fn complaint(&mut self, job_id: JobId, agreement: bool) -> Result<(), JobError>{
            let caller = self.env().caller();
            let caller_info = self.personal_account_info.get(caller);
            // kiểm tra đăng kí và role 
            match caller_info.clone() {
                None => return Err(JobError::NotRegistered), //check đăng kí chưa
                Some(user_info) => { //check role đúng chưa
                    if user_info.role != AccountRole::FREELANCER {return Err(JobError::NotFreelancer)}
                },
            }
            let mut job = self.jobs.get(job_id).ok_or(JobError::NotExisted)?;
            //check end_time
            if job.end_time < self.env().block_timestamp() {return Err(JobError::OutOfDate)};
            //check job đó có phải của mình hay không
            if job.person_obtain.unwrap() != caller {return Err(JobError::NotTakeThisJob)};

            match job.status {
                // Status::OPEN | Status::REOPEN => return Err(JobError::NotTaked), //không xảy ra
                Status::DOING | Status::REVIEW => return Err(JobError::Proccesing),
                Status::CANCELED | Status::FINISH => return Err(JobError::Finish), 
                Status::UNQUALIFIED => {  
                    match agreement {
                        true => {
                            //update lại thông tin job
                            job.result = None;
                            job.status = Status::REOPEN;
                            job.person_obtain = None;
                            self.jobs.insert(job_id, &job);
                            //update lại unccessful_jobs
                            //khởi tạo jobs chưa hoàn thành, đánh giá nằm ở rating: đánh giá ở unccessfull job và rate cho user_info.
                            self.unsuccesful_jobs.insert((job.person_create.unwrap(), caller, job_id), &Some(String::new()));
                        },
                        false => {
                            // chưa có làm

                        }
                    }
                }
                _ => () //status open và reopen
            }
            Ok(())
        }


        #[ink(message, payable)]
        pub fn aproval(&mut self, job_id: JobId) -> Result<(), JobError>{
            let caller = self.env().caller();
            let caller_info = self.personal_account_info.get(caller);
            // kiểm tra đăng kí và role 
            match caller_info.clone() {
                None => return Err(JobError::NotRegistered), //check đăng kí chưa
                Some(user_info) => { //check role đúng chưa
                    if user_info.role == AccountRole::FREELANCER {return Err(JobError::NotJobAssigner)}
                },
            }
            let mut job = self.jobs.get(job_id).ok_or(JobError::NotExisted)?;
            //check end_time
            if job.end_time < self.env().block_timestamp() {return Err(JobError::OutOfDate)};
            //check job đó có phải của mình hay không
            if job.person_create.unwrap() != caller {return Err(JobError::NotAssignThisJob)};

            match job.status {
                Status::OPEN | Status::REOPEN => return Err(JobError::NotTaked),
                Status::DOING | Status::UNQUALIFIED => return Err(JobError::Proccesing),
                Status::CANCELED | Status::FINISH => return Err(JobError::Finish), 
                Status::REVIEW => {  
                    //update lại thông tin job
                    job.status = Status::FINISH;
                    self.jobs.insert(job_id, &job);
                    //update user_info chỗ công việc thành công của owner tăng thêm 1
                    let mut owner_detail = caller_info.unwrap();
                    owner_detail.successful_jobs_and_all_jobs.0 = owner_detail.successful_jobs_and_all_jobs.0 + 1;
                    self.personal_account_info.insert(caller, &owner_detail);
                    //update user_info chỗ công việc hoàn thành của freelancer tăng thêm 1
                    let freelancer = job.person_obtain.unwrap();
                    let mut freelancer_detail = self.personal_account_info.get(freelancer).unwrap();
                    freelancer_detail.successful_jobs_and_all_jobs.0 = freelancer_detail.successful_jobs_and_all_jobs.0 + 1;
                    self.personal_account_info.insert(freelancer, &freelancer_detail);
                    //khởi tạo job thành công, nội dung đánh giá sẽ do raiting làm
                    self.successful_jobs.insert((caller, freelancer, job_id), &Some(String::new()));
                    // chuyển tiền và giữ lại phần trăm phí 
                    let budget = job.budget * (100 - FEE_PERCENTAGE as u128)/100;
                    let _ = self.env().transfer(freelancer, budget);
                }
            }
            Ok(())
        } 
        
        #[ink(message,payable)]
        pub fn cancel(&mut self, job_id: JobId) -> Result<(), JobError> {
            // Khách hàng có thể huỷ job nếu job ở trạng thái OPEN hoặc REOPEN hoặc UNQUALIFIED mà hết thời gian job, nếu job đã được giao thì không thể huỷ, budget của job sẽ được trả lại cho người giao job đó.
            // Retrieve job
            // let mut job = self.jobs.get(job_id).ok_or(JobError::NotExisted)?;
            // // Check caller is job owner ?
            // let caller = self.env().caller();
            // if self.owner_jobs.get(caller).unwrap().contains(&job_id) {
            //         return Err(JobError::NotAssignThisJob)        
            // }
            let caller = self.env().caller();
            let caller_info = self.personal_account_info.get(caller);
            // kiểm tra đăng kí và role 
            match caller_info.clone() {
                None => return Err(JobError::NotRegistered), //check đăng kí chưa
                Some(user_info) => { //check role đúng chưa
                    if user_info.role == AccountRole::FREELANCER {return Err(JobError::NotJobAssigner)}
                },
            }
            let mut job = self.jobs.get(job_id).ok_or(JobError::NotExisted)?;

            //check job đó có phải của mình hay không
            if job.person_create.unwrap() != caller {return Err(JobError::NotAssignThisJob)};
            
            // // Only allow cancel if status is OPEN or REOPEN
            match job.status {
                Status::OPEN | Status::REOPEN => {
                    // Set status to canceled
                    job.status = Status::CANCELED;
                    // Update job in storage
                    self.jobs.insert(job_id, &job);
                    // trả tiền
                    let budget = job.budget * (100 - FEE_PERCENTAGE as u128) / 100; // chuyển tiền và giữ lại phần trăm phí tạo việc 
                    let _ = self.env().transfer(caller, budget);
                    //update unsuccessful_jobs: chú ý là chỉ có chỗ này lưu trữ thông tin công việc đã thất bại. còn nếu công việc đó được reopen thì trong jobs chỉ lưu trạng thái tiếp theo của công việc đó.
                    self.unsuccesful_jobs.insert((caller, job.person_obtain.unwrap(), job_id), &Some(String::new()));
                },
                Status::UNQUALIFIED => {
                    if job.end_time < self.env().block_timestamp() { //nếu đã qua thời gian thì được cancle job
                        // Set status to canceled
                        job.status = Status::CANCELED;
                        // Update job in storage
                        self.jobs.insert(job_id, &job);
                        // trả tiền
                        let budget = job.budget * (100 - FEE_PERCENTAGE as u128) / 100; // chuyển tiền và giữ lại phần trăm phí tạo việc 
                        let _ = self.env().transfer(caller, budget);
                        //update unsuccessful_jobs: chú ý là chỉ có chỗ này lưu trữ thông tin công việc đã thất bại. còn nếu công việc đó được reopen thì trong jobs chỉ lưu trạng thái tiếp theo của công việc đó.
                        self.unsuccesful_jobs.insert((caller, job.person_obtain.unwrap(), job_id), &Some(String::new()));
                    }else { //nếu còn thời gian freelancer có thể gọi complaint nhiều lần, nếu hết thời gian mà ko đồng ý kết quả thì trạng thái job là Unqualified và người tạo phải hủy để rút tiền về, freelancer và người tạo job có thể raiting và "phốt" nhau.
                        return Err(JobError::Proccesing)
                    }
                },
                Status::DOING | Status::REVIEW => return Err(JobError::Proccesing),
                Status::CANCELED | Status::FINISH => return Err(JobError::Finish), // job đã bị hủy hoặc finish
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
