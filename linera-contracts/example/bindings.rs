#[allow(clippy::all)]
mod contract {
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
  #[export_name = "example: func() -> variant { pending, ready(u32) }"]
  unsafe extern "C" fn __wit_bindgen_contract_example() -> i32{
    let result0 = <super::Contract as Contract>::example();
    let ptr1 = __CONTRACT_RET_AREA.0.as_mut_ptr() as i32;
    match result0 {
      Poll::Pending=> {
        {
          *((ptr1 + 0) as *mut u8) = (0i32) as u8;
          
        }
      }
      Poll::Ready(e) => {
        *((ptr1 + 0) as *mut u8) = (1i32) as u8;
        *((ptr1 + 4) as *mut i32) = wit_bindgen_guest_rust::rt::as_i32(e);
        
      },
    };
    ptr1
  }
  
  #[repr(align(4))]
  struct __ContractRetArea([u8; 8]);
  static mut __CONTRACT_RET_AREA: __ContractRetArea = __ContractRetArea([0; 8]);
  pub trait Contract {
    fn example() -> Poll;
  }
}
