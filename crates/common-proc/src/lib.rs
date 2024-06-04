extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input, ItemFn, LitInt, Result,
};

/// The arguments for the `#[client_entry]` attribute proc macro
struct MacroArgs {
    /// The heap size to allocate
    heap_size: LitInt,
}

impl Parse for MacroArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let heap_size = input.parse()?;
        Ok(MacroArgs { heap_size })
    }
}

#[proc_macro_attribute]
pub fn client_entry(attr: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as MacroArgs);
    let input_fn = parse_macro_input!(input as ItemFn);

    let heap_size = args.heap_size;
    let fn_body = &input_fn.block;
    let fn_name = &input_fn.sig.ident;

    let expanded = quote! {
        use anyhow::Result as AnyhowResult;

        fn #fn_name() -> AnyhowResult<()> {
            #fn_body
        }

        cfg_if::cfg_if! {
            if #[cfg(any(target_arch = "mips", target_arch = "riscv64"))] {
                const HEAP_SIZE: usize = #heap_size;

                #[no_mangle]
                pub extern "C" fn _start() {
                    kona_common::alloc_heap!(HEAP_SIZE);
                    #fn_name().unwrap();
                    kona_common::io::exit(0);
                }

                #[panic_handler]
                fn panic(info: &core::panic::PanicInfo) -> ! {
                    let msg = alloc::format!("Panic: {}", info);
                    kona_common::io::print_err(msg.as_ref());
                    kona_common::io::exit(2)
                }
            }
        }
    };

    TokenStream::from(expanded)
}
