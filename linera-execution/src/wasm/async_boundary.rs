use futures::future::BoxFuture;
use std::{
    any::type_name,
    fmt::{self, Debug, Formatter},
    future::Future,
    marker::PhantomData,
    mem,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};
use tokio::sync::Mutex;

pub struct HostFuture<'future, Output> {
    future: Mutex<BoxFuture<'future, Output>>,
}

impl<Output> Debug for HostFuture<'_, Output> {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter
            .debug_struct(&format!("HostFuture<'_, {}>", type_name::<Output>()))
            .finish_non_exhaustive()
    }
}

impl<'future, Output> HostFuture<'future, Output> {
    pub fn new(future: impl Future<Output = Output> + Send + 'future) -> Self {
        HostFuture {
            future: Mutex::new(Box::pin(future)),
        }
    }

    pub fn poll(&self, context: &mut ContextForwarder) -> Poll<Output> {
        let mut context_reference = context
            .0
            .try_lock()
            .expect("Unexpected concurrent contract call");

        let context = context_reference
            .as_mut()
            .expect("Contract called without an async task context");

        let mut future = self
            .future
            .try_lock()
            .expect("Contract can't call the future concurrently because it's single threaded");

        future.as_mut().poll(context)
    }
}

pub enum GuestFuture<Future, Runtime>
where
    Runtime: super::Runtime,
{
    FailedToCreate,
    Active {
        context_forwarder: ContextForwarder,
        contract: Runtime::Contract,
        store: Runtime::Store,
        future: Future,
    },
}

impl<Future, Runtime> GuestFuture<Future, Runtime>
where
    Runtime: super::Runtime,
{
    pub fn new<Trap>(
        creation_result: Result<Future, Trap>,
        context_forwarder: ContextForwarder,
        contract: Runtime::Contract,
        store: Runtime::Store,
    ) -> Self {
        match creation_result {
            Ok(future) => GuestFuture::Active {
                context_forwarder,
                contract,
                store,
                future,
            },
            Err(trap) => GuestFuture::FailedToCreate,
        }
    }
}

impl<InnerFuture, Runtime> Future for GuestFuture<InnerFuture, Runtime>
where
    InnerFuture: GuestFutureInterface<Runtime> + Unpin,
    Runtime: super::Runtime,
    Runtime::Contract: Unpin,
    Runtime::Store: Unpin,
{
    type Output = Result<InnerFuture::Output, linera_base::error::Error>;

    fn poll(self: Pin<&mut Self>, context: &mut Context) -> Poll<Self::Output> {
        match self.get_mut() {
            GuestFuture::FailedToCreate => {
                Poll::Ready(Err(linera_base::error::Error::UnknownApplication))
            }
            GuestFuture::Active {
                context_forwarder,
                contract,
                store,
                future,
            } => {
                let _context_guard = context_forwarder.forward(context);
                future.poll(contract, store)
            }
        }
    }
}

pub trait GuestFutureInterface<Runtime>
where
    Runtime: super::Runtime,
{
    type Output;

    fn poll(
        &self,
        contract: &Runtime::Contract,
        store: &mut Runtime::Store,
    ) -> Poll<Result<Self::Output, linera_base::error::Error>>;
}

#[derive(Clone, Default)]
pub struct ContextForwarder(Arc<Mutex<Option<&'static mut Context<'static>>>>);

impl ContextForwarder {
    pub fn forward<'context>(
        &mut self,
        context: &'context mut Context,
    ) -> ActiveContextGuard<'context> {
        let mut context_reference = self
            .0
            .try_lock()
            .expect("Unexpected concurrent task context access");

        assert!(
            context_reference.is_none(),
            "`ContextForwarder` accessed by concurrent tasks"
        );

        *context_reference = Some(unsafe { mem::transmute(context) });

        ActiveContextGuard {
            context: self.0.clone(),
            lifetime: PhantomData,
        }
    }
}

pub struct ActiveContextGuard<'context> {
    context: Arc<Mutex<Option<&'static mut Context<'static>>>>,
    lifetime: PhantomData<&'context mut ()>,
}

impl Drop for ActiveContextGuard<'_> {
    fn drop(&mut self) {
        let mut context_reference = self
            .context
            .try_lock()
            .expect("Unexpected concurrent task context access");

        *context_reference = None;
    }
}
