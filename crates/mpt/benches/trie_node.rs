//! Contains benchmarks for the [TrieNode].

use alloy_trie::Nibbles;
use criterion::{criterion_group, criterion_main, Criterion};
use kona_mpt::{NoopTrieDBFetcher, NoopTrieDBHinter, TrieNode};
use pprof::criterion::{Output, PProfProfiler};
use rand::{rngs::StdRng, seq::SliceRandom, Rng, SeedableRng};

fn trie(c: &mut Criterion) {
    let mut g = c.benchmark_group("execution");
    g.sample_size(10);

    // Use pseudo-randomness for reproducibility
    let mut rng = StdRng::seed_from_u64(42);

    g.bench_function("Insertion - 4096 nodes", |b| {
        let keys =
            (0..2usize.pow(12)).map(|_| Nibbles::unpack(rng.gen::<[u8; 32]>())).collect::<Vec<_>>();

        b.iter(|| {
            let mut trie = TrieNode::Empty;
            for key in &keys {
                trie.insert(key, key.to_vec().into(), &NoopTrieDBFetcher).unwrap();
            }
        });
    });

    g.bench_function("Insertion - 65,536 nodes", |b| {
        let keys =
            (0..2usize.pow(16)).map(|_| Nibbles::unpack(rng.gen::<[u8; 32]>())).collect::<Vec<_>>();

        b.iter(|| {
            let mut trie = TrieNode::Empty;
            for key in &keys {
                trie.insert(key, key.to_vec().into(), &NoopTrieDBFetcher).unwrap();
            }
        });
    });

    g.bench_function("Delete 16 nodes - 4096 nodes", |b| {
        let keys =
            (0..2usize.pow(12)).map(|_| Nibbles::unpack(rng.gen::<[u8; 32]>())).collect::<Vec<_>>();
        let mut trie = TrieNode::Empty;

        let rng = &mut rand::thread_rng();
        let keys_to_delete = keys.choose_multiple(rng, 16).cloned().collect::<Vec<_>>();

        for key in &keys {
            trie.insert(key, key.to_vec().into(), &NoopTrieDBFetcher).unwrap();
        }

        b.iter(|| {
            let trie = &mut trie.clone();
            for key in &keys_to_delete {
                trie.delete(key, &NoopTrieDBFetcher, &NoopTrieDBHinter).unwrap();
            }
        });
    });

    g.bench_function("Delete 16 nodes - 65,536 nodes", |b| {
        let keys =
            (0..2usize.pow(16)).map(|_| Nibbles::unpack(rng.gen::<[u8; 32]>())).collect::<Vec<_>>();
        let mut trie = TrieNode::Empty;
        for key in &keys {
            trie.insert(key, key.to_vec().into(), &NoopTrieDBFetcher).unwrap();
        }

        let rng = &mut rand::thread_rng();
        let keys_to_delete = keys.choose_multiple(rng, 16).cloned().collect::<Vec<_>>();

        b.iter(|| {
            let trie = &mut trie.clone();
            for key in &keys_to_delete {
                trie.delete(key, &NoopTrieDBFetcher, &NoopTrieDBHinter).unwrap();
            }
        });
    });

    g.bench_function("Open 1024 nodes - 4096 nodes", |b| {
        let keys =
            (0..2usize.pow(12)).map(|_| Nibbles::unpack(rng.gen::<[u8; 32]>())).collect::<Vec<_>>();
        let mut trie = TrieNode::Empty;
        for key in &keys {
            trie.insert(key, key.to_vec().into(), &NoopTrieDBFetcher).unwrap();
        }

        let rng = &mut rand::thread_rng();
        let keys_to_retrieve = keys.choose_multiple(rng, 1024).cloned().collect::<Vec<_>>();

        b.iter(|| {
            for key in &keys_to_retrieve {
                trie.open(key, &NoopTrieDBFetcher).unwrap();
            }
        });
    });

    g.bench_function("Open 1024 nodes - 65,536 nodes", |b| {
        let keys =
            (0..2usize.pow(16)).map(|_| Nibbles::unpack(rng.gen::<[u8; 32]>())).collect::<Vec<_>>();
        let mut trie = TrieNode::Empty;
        for key in &keys {
            trie.insert(key, key.to_vec().into(), &NoopTrieDBFetcher).unwrap();
        }

        let rng = &mut rand::thread_rng();
        let keys_to_retrieve = keys.choose_multiple(rng, 1024).cloned().collect::<Vec<_>>();

        b.iter(|| {
            for key in &keys_to_retrieve {
                trie.open(key, &NoopTrieDBFetcher).unwrap();
            }
        });
    });

    g.bench_function("Compute root, fully open trie - 4096 nodes", |b| {
        let keys =
            (0..2usize.pow(12)).map(|_| Nibbles::unpack(rng.gen::<[u8; 32]>())).collect::<Vec<_>>();
        let mut trie = TrieNode::Empty;
        for key in &keys {
            trie.insert(key, key.to_vec().into(), &NoopTrieDBFetcher).unwrap();
        }

        b.iter(|| {
            let trie = &mut trie.clone();
            trie.blind();
        });
    });

    g.bench_function("Compute root, fully open trie - 65,536 nodes", |b| {
        let keys =
            (0..2usize.pow(16)).map(|_| Nibbles::unpack(rng.gen::<[u8; 32]>())).collect::<Vec<_>>();
        let mut trie = TrieNode::Empty;
        for key in &keys {
            trie.insert(key, key.to_vec().into(), &NoopTrieDBFetcher).unwrap();
        }

        b.iter(|| {
            let trie = &mut trie.clone();
            trie.blind();
        });
    });
}

criterion_group! {
    name = trie_benches;
    config = Criterion::default().with_profiler(PProfProfiler::new(100, Output::Flamegraph(None)));
    targets = trie
}
criterion_main!(trie_benches);
