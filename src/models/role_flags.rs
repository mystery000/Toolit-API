use serde::{Deserialize, Serialize};

bitflags! {
    #[derive(Serialize, Deserialize)]
    pub struct RoleFlags: u32 {
        const NONE                   = 0b0000000000;
        const CRAFTSMAN              = 0b0000000001;
        const OFFICE_CONTENT_ADMIN   = 0b0000000010;
        const OFFICE_BILLING_ADMIN   = 0b0000000100;
        const OFFICE_PERSONNEL_ADMIN = 0b0000001000;
        const GLOBAL_CONTENT_ADMIN   = 0b0000010000;
        const GLOBAL_BILLING_ADMIN   = 0b0000100000;
        const GLOBAL_PERSONNEL_ADMIN = 0b0001000000;
    }
}
