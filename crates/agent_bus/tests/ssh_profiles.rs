//! Execute native-app metadata checks without desktop dependencies.
#[path = "../../../app/src/agent_communication/setup.rs"]
pub(crate) mod setup;
mod agent_communication {
    pub(crate) use crate::setup;
}
#[path = "../../../app/src/ssh_remote/profiles.rs"]
mod profiles;
