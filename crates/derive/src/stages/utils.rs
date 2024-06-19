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
    let mut output = brotli_state.alloc_u8.alloc_cell(FJORD_MAX_SPAN_BATCH_BYTES as usize);
    tracing::info!(target: "brotli", "Decompressing Brotli data with output length: {}", output.len());
    let mut available_in = data.len();
    let mut input_offset = 0;
    let mut available_out = output.len();
    let mut output_offset = 0;
    let mut written = 0;

    // Decompress the data stream until success or failure
    loop {
        // tracing::trace!(target: "brotli", "decompressing brotli stream...");
        match brotli::BrotliDecompressStream(
            &mut available_in,
            &mut input_offset,
            data,
            &mut available_out,
            &mut output_offset,
            output.slice_mut(),
            &mut written,
            &mut brotli_state,
        ) {
            brotli::BrotliResult::ResultSuccess => break,
            brotli::BrotliResult::ResultFailure => {
                tracing::warn!(target: "brotli", "Brotli decompression failed");
                break;
            }
            brotli::BrotliResult::NeedsMoreInput => tracing::debug!(target: "brotli", "Brotli needs more input, output: {:?}", output.slice()),
            brotli::BrotliResult::NeedsMoreOutput => {
                tracing::warn!(target: "brotli", "Brotli needs more output");
                break;
            }
        }
    }
    tracing::trace!(target: "brotli", "Written: {}", written);
    tracing::trace!(target: "brotli", "Output: {:?}", output.slice());
    tracing::trace!(target: "brotli", "Output offset: {}", output_offset);
    Ok(output.slice().to_vec())
}

#[cfg(test)]
mod test {
    use super::*;

    // Tuple of (compressed, decompressed) test vectors.
    const TEST_VECTORS: &[(&str, &str)] = &[
        (
            // The channel compressor uses the first byte to store the compression type.
            "018b048075ed184249e9bc19675e",
            "75ed184249e9bc19675e",
        ),
        (
            // The channel compressor uses the first byte to store the compression type.
            "018b098075ed184249e9bc19675e4d1f766213da71b64278",
            "75ed184249e9bc19675e4d1f766213da71b64278",
        ),
    ];

    #[test]
    fn test_decompress_brotli() {
        use tracing::Level;
        let subscriber = tracing_subscriber::fmt().with_max_level(Level::TRACE).finish();
        tracing::subscriber::set_global_default(subscriber).unwrap();

        for (compressed, expected) in TEST_VECTORS {
            let compressed = alloy_primitives::hex::decode(compressed).unwrap();
            let expected = alloy_primitives::hex::decode(expected).unwrap();
            let decompressed = decompress_brotli(&compressed[1..]).unwrap();
            assert_eq!(decompressed, expected);
        }
    }
}
