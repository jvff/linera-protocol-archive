// Export the system interface used by a user contract.
wit_bindgen_host_wasmtime_rust::export!("../linera-sdk/writable_system.wit");

// Export the system interface used by a user service.
wit_bindgen_host_wasmtime_rust::export!("../linera-sdk/queryable_system.wit");

// Import the interface implemented by a user contract.
wit_bindgen_host_wasmtime_rust::import!("../linera-sdk/contract.wit");

// Import the interface implemented by a user service.
wit_bindgen_host_wasmtime_rust::import!("../linera-sdk/service.wit");

use self::{
    contract::{Contract, ContractData},
    queryable_system::{QueryableSystem, QueryableSystemTables},
    service::{Service, ServiceData},
    writable_system::{WritableSystem, WritableSystemTables},
};
use anyhow::Result;
use wasmtime::Linker;
use wit_bindgen_host_wasmtime_rust::Le;

pub fn configure_linker(linker: &mut Linker<State>) -> Result<()> {
    linker.allow_shadowing(true);

    writable_system::add_to_linker(linker, State::writable_api)?;
    queryable_system::add_to_linker(linker, State::queryable_api)?;

    Contract::add_to_linker(linker, State::contract_data)?;
    Service::add_to_linker(linker, State::service_data)?;

    Ok(())
}

#[derive(Default)]
pub struct State {
    api: Api,
    contract_data: ContractData,
    service_data: ServiceData,
    queryable_tables: QueryableSystemTables<Api>,
    writable_tables: WritableSystemTables<Api>,
}

impl State {
    pub fn contract_data(&mut self) -> &mut ContractData {
        &mut self.contract_data
    }

    pub fn service_data(&mut self) -> &mut ServiceData {
        &mut self.service_data
    }

    pub fn writable_api(&mut self) -> (&mut Api, &mut WritableSystemTables<Api>) {
        (&mut self.api, &mut self.writable_tables)
    }

    pub fn queryable_api(&mut self) -> (&mut Api, &mut QueryableSystemTables<Api>) {
        (&mut self.api, &mut self.queryable_tables)
    }
}

#[derive(Default)]
pub struct Api;

impl QueryableSystem for Api {
    type Load = ();

    fn chain_id(&mut self) -> queryable_system::HashValue {
        todo!();
    }

    fn application_id(&mut self) -> queryable_system::ApplicationId {
        todo!();
    }

    fn read_system_balance(&mut self) -> queryable_system::SystemBalance {
        todo!();
    }

    fn load_new(&mut self) {}

    fn load_poll(&mut self, _: &Self::Load) -> queryable_system::PollLoad {
        todo!();
    }
}

impl WritableSystem for Api {
    type Load = ();
    type LoadAndLock = ();
    type TryCallApplication = ();
    type TryCallSession = ();

    fn chain_id(&mut self) -> writable_system::HashValue {
        todo!();
    }

    fn application_id(&mut self) -> writable_system::ApplicationId {
        todo!();
    }

    fn read_system_balance(&mut self) -> writable_system::SystemBalance {
        todo!();
    }

    fn load_new(&mut self) {}

    fn load_poll(&mut self, _: &Self::Load) -> writable_system::PollLoad {
        todo!();
    }

    fn load_and_lock_new(&mut self) {}

    fn load_and_lock_poll(&mut self, _: &Self::Load) -> writable_system::PollLoad {
        todo!();
    }

    fn try_call_application_new(
        &mut self,
        _: bool,
        _: writable_system::ApplicationId,
        _: &[u8],
        _: &[Le<writable_system::SessionId>],
    ) {
    }

    fn try_call_application_poll(&mut self, _: &Self::Load) -> writable_system::PollCallResult {
        todo!();
    }

    fn try_call_session_new(
        &mut self,
        _: bool,
        _: writable_system::SessionId,
        _: &[u8],
        _: &[Le<writable_system::SessionId>],
    ) {
    }

    fn try_call_session_poll(&mut self, _: &Self::Load) -> writable_system::PollCallResult {
        todo!();
    }

    fn store_and_unlock(&mut self, _: &[u8]) -> bool {
        todo!();
    }
}
