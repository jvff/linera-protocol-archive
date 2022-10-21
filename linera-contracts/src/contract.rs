pub struct Contract {
    contract: Contract,
    instance: Instance,
    store: Store,
}

#[async_trait]
impl UserApplication for Contract {
    async fn apply_operation(
        &self,
        context: &OperationContext,
        storage: StorageContext<'_, true>,
        operation: &[u8],
    ) -> Result<RawApplicationResult<Vec<u8>>, Error> {
        todo!();
    }

    async fn apply_effect(
        &self,
        context: &EffectContext,
        storage: StorageContext<'_, true>,
        effect: &[u8],
    ) -> Result<RawApplicationResult<Vec<u8>>, Error> {
        todo!();
    }

    async fn call(
        &self,
        context: &CalleeContext,
        storage: StorageContext<'_, true>,
        name: &str,
        argument: &[u8],
    ) -> Result<(Vec<u8>, RawApplicationResult<Vec<u8>>), Error> {
        todo!();
    }

    async fn query(
        &self,
        context: &QueryContext,
        storage: StorageContext<'_, false>,
        name: &str,
        argument: &[u8],
    ) -> Result<Vec<u8>, Error> {
        todo!();
    }
}
