#[allow(clippy::all)]
pub mod contract {
  #[allow(unused_imports)]
  use wit_bindgen_host_wasmtime_rust::{wasmtime, anyhow};
  #[derive(Clone, Copy)]
  pub enum Poll{
    Pending,
    Ready(u32),
  }
  impl core::fmt::Debug for Poll {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
      match self {
        Poll::Pending => {
          f.debug_tuple("Poll::Pending").finish()
        }
        Poll::Ready(e) => {
          f.debug_tuple("Poll::Ready").field(e).finish()
        }
      }
    }
  }
  
  /// Auxiliary data associated with the wasm exports.
  ///
  /// This is required to be stored within the data of a
  /// `Store<T>` itself so lifting/lowering state can be managed
  /// when translating between the host and wasm.
  #[derive(Default)]
  pub struct ContractData {
  }
  pub struct Contract<T> {
    get_state: Box<dyn Fn(&mut T) -> &mut ContractData + Send + Sync>,
    example: wasmtime::TypedFunc<(), (i32,)>,
    memory: wasmtime::Memory,
  }
  impl<T> Contract<T> {
    #[allow(unused_variables)]
    
    /// Adds any intrinsics, if necessary for this exported wasm
    /// functionality to the `linker` provided.
    ///
    /// The `get_state` closure is required to access the
    /// auxiliary data necessary for these wasm exports from
    /// the general store's state.
    pub fn add_to_linker(
    linker: &mut wasmtime::Linker<T>,
    get_state: impl Fn(&mut T) -> &mut ContractData + Send + Sync + Copy + 'static,
    ) -> anyhow::Result<()> {
      Ok(())
    }
    
    /// Instantiates the provided `module` using the specified
    /// parameters, wrapping up the result in a structure that
    /// translates between wasm and the host.
    ///
    /// The `linker` provided will have intrinsics added to it
    /// automatically, so it's not necessary to call
    /// `add_to_linker` beforehand. This function will
    /// instantiate the `module` otherwise using `linker`, and
    /// both an instance of this structure and the underlying
    /// `wasmtime::Instance` will be returned.
    ///
    /// The `get_state` parameter is used to access the
    /// auxiliary state necessary for these wasm exports from
    /// the general store state `T`.
    pub fn instantiate(
    mut store: impl wasmtime::AsContextMut<Data = T>,
    module: &wasmtime::Module,
    linker: &mut wasmtime::Linker<T>,
    get_state: impl Fn(&mut T) -> &mut ContractData + Send + Sync + Copy + 'static,
    ) -> anyhow::Result<(Self, wasmtime::Instance)> {
      Self::add_to_linker(linker, get_state)?;
      let instance = linker.instantiate(&mut store, module)?;
      Ok((Self::new(store, &instance,get_state)?, instance))
    }
    
    /// Low-level creation wrapper for wrapping up the exports
    /// of the `instance` provided in this structure of wasm
    /// exports.
    ///
    /// This function will extract exports from the `instance`
    /// defined within `store` and wrap them all up in the
    /// returned structure which can be used to interact with
    /// the wasm module.
    pub fn new(
    mut store: impl wasmtime::AsContextMut<Data = T>,
    instance: &wasmtime::Instance,
    get_state: impl Fn(&mut T) -> &mut ContractData + Send + Sync + Copy + 'static,
    ) -> anyhow::Result<Self> {
      let mut store = store.as_context_mut();
      let example= instance.get_typed_func::<(), (i32,), _>(&mut store, "example: func() -> variant { pending, ready(u32) }")?;
      let memory= instance
      .get_memory(&mut store, "memory")
      .ok_or_else(|| {
        anyhow::anyhow!("`memory` export not a memory")
      })?
      ;
      Ok(Contract{
        example,
        memory,
        get_state: Box::new(get_state),
        
      })
    }
    pub fn example(&self, mut caller: impl wasmtime::AsContextMut<Data = T>,)-> Result<Poll, wasmtime::Trap> {
      let memory = &self.memory;
      let (result0_0,) = self.example.call(&mut caller, ())?;
      let load1 = memory.data_mut(&mut caller).load::<u8>(result0_0 + 0)?;
      Ok(match i32::from(load1) {
        0 => Poll::Pending,
        1 => Poll::Ready({
          let load2 = memory.data_mut(&mut caller).load::<i32>(result0_0 + 4)?;
          load2 as u32
        }),
        _ => return Err(invalid_variant("Poll")),
      })
    }
  }
  use wit_bindgen_host_wasmtime_rust::rt::RawMem;
  use wit_bindgen_host_wasmtime_rust::rt::invalid_variant;
}
