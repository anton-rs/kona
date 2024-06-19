//! Stage Utilities

// use alloc::vec;
use alloc::vec::Vec;
use alloc_stdlib::heap_alloc::*;
// use alloc_no_stdlib::*;
use anyhow::Result;
use brotli::*;
use crate::FJORD_MAX_SPAN_BATCH_BYTES;

/// Decompresses the given bytes data using the Brotli decompressor implemented
/// in the [`brotli`](https://crates.io/crates/brotli) crate.
pub fn decompress_brotli(data: &[u8]) -> Result<Vec<u8>> {
    // let mut u8_buffer = define_allocator_memory_pool!(4096, u8, [0; 32 * 1024 * 1024], heap);
    // let mut u32_buffer = define_allocator_memory_pool!(4096, u32, [0; 1024 * 1024], heap);
    // let mut hc_buffer = define_allocator_memory_pool!(4096, HuffmanCode, [0; 4 * 1024 * 1024], heap);
    //
    // let heap_u8_allocator = HeapPrealloc::<u8>::new_allocator(4096, &mut u8_buffer, bzero);
    // let heap_u32_allocator = HeapPrealloc::<u32>::new_allocator(4096, &mut u32_buffer, bzero);
    // let heap_hc_allocator = HeapPrealloc::<HuffmanCode>::new_allocator(4096, &mut hc_buffer, bzero);
    
    let heap_u8_alloc = HeapAlloc::<u8>::new(0);
    let heap_u32_alloc = HeapAlloc::<u32>::new(0);
    let heap_hc_alloc = HeapAlloc::<HuffmanCode>::new(Default::default());

    let mut brotli_state = BrotliState::new(heap_u8_alloc, heap_u32_alloc, heap_hc_alloc);

    // declare_stack_allocator_struct!(StackAllocatedFreelist4, 4, stack);
    //
    // let mut u8_buffer = define_allocator_memory_pool!(4, u8, [0; 32 * 1024 * 1024], stack);
    // let mut stack_u8_alloc = StackAllocatedFreelist4::<u8>::new_allocator(&mut u8_buffer, bzero);
    //
    // let mut u32_buffer = define_allocator_memory_pool!(4, u32, [0; 1024 * 1024], stack);
    // let mut stack_u32_alloc = StackAllocatedFreelist4::<u32>::new_allocator(&mut u32_buffer,
    // bzero);
    //
    // let mut hc_buffer = define_allocator_memory_pool!(4, HuffmanCode, [0; 4 * 1024 * 1024],
    // stack); let mut stack_hc_alloc =
    // StackAllocatedFreelist4::<HuffmanCode>::new_allocator(&mut hc_buffer, bzero);


    // let mut brotli_state = BrotliState::new(
    //     StandardAlloc::default(),
    //     StandardAlloc::default(),
    //     StandardAlloc::default(),
    // );

    // Setup the decompressor inputs and outputs
    let mut output = Vec::with_capacity(data.len());
    // let input = &data[..];
    let mut available_in = data.len();
    let mut input_offset = 0;
    let mut available_out = FJORD_MAX_SPAN_BATCH_BYTES as usize;
    let mut output_offset = 0;
    let mut written = 0;

    // Decompress the data stream until success or failure
    loop {
        tracing::trace!(target: "brotli", "decompressing brotli stream...");
        match brotli::BrotliDecompressStream(
            &mut available_in,
            &mut input_offset,
            data,
            &mut available_out,
            &mut output_offset,
            &mut output,
            &mut written,
            &mut brotli_state,
        ) {
            brotli::BrotliResult::ResultSuccess => break,
            brotli::BrotliResult::ResultFailure => {
                tracing::warn!(target: "brotli", "Brotli decompression failed");
                break;
            }
            _ => tracing::debug!(target: "batch-reader", "decompressing brotli data"),
        }
    }
    tracing::trace!(target: "brotli", "Written: {}", written);
    tracing::trace!(target: "brotli", "Output: {:?}", output);
    tracing::trace!(target: "brotli", "Output offset: {}", output_offset);
    Ok(output)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_decompress_brotli() {
        use tracing::Level;
        let subscriber = tracing_subscriber::fmt().with_max_level(Level::TRACE).finish();
        tracing::subscriber::set_global_default(subscriber).unwrap();
        let expected = alloy_primitives::hex::decode("75ed184249e9bc19675e").unwrap();
        let compressed = alloy_primitives::hex::decode("018b048075ed184249e9bc19675e").unwrap();
        let decompressed = decompress_brotli(&compressed).unwrap();
        assert_eq!(decompressed, expected);
    }
}
