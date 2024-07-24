// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::time::{Duration, Instant};

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use linera_views::{
    batch::Batch,
    common::Context,
    memory::{create_memory_context, MemoryContext},
    reentrant_collection_view::ReentrantCollectionView,
    register_view::RegisterView,
    views::View,
};
use tokio::runtime::Runtime;

fn bench_load_all_entries_already_in_memory(criterion: &mut Criterion) {
    criterion.bench_function(
        "load_all_entries_already_in_memory_with_method",
        |bencher| {
            bencher
                .to_async(Runtime::new().expect("Failed to create Tokio runtime"))
                .iter_custom(|iterations| async move {
                    let mut total_time = Duration::ZERO;

                    for _ in 0..iterations {
                        let view = create_populated_reentrant_collection_view().await;

                        let measurement = Instant::now();
                        black_box(
                            view.load_all_entries()
                                .await
                                .expect("Failed to load entries from `ReentrantCollectionView`")
                                .collect::<Vec<_>>(),
                        );
                        total_time += measurement.elapsed();
                    }

                    total_time
                })
        },
    );

    criterion.bench_function("load_all_entries_already_in_memory_manually", |bencher| {
        bencher
            .to_async(Runtime::new().expect("Failed to create Tokio runtime"))
            .iter_custom(|iterations| async move {
                let mut total_time = Duration::ZERO;

                for _ in 0..iterations {
                    let view = create_populated_reentrant_collection_view().await;

                    let measurement = Instant::now();
                    let indices = view
                        .indices()
                        .await
                        .expect("Failed to load all indices from `ReentrantCollectionView`");
                    black_box(
                        view.try_load_entries(&indices)
                            .await
                            .expect("Failed to load entries from `ReentrantCollectionView`"),
                    );
                    total_time += measurement.elapsed();
                }

                total_time
            })
    });
}

fn bench_load_all_entries_from_storage(criterion: &mut Criterion) {
    criterion.bench_function("load_all_entries_from_storage_with_method", |bencher| {
        bencher
            .to_async(Runtime::new().expect("Failed to create Tokio runtime"))
            .iter_custom(|iterations| async move {
                let mut total_time = Duration::ZERO;

                for _ in 0..iterations {
                    let view = create_and_store_populated_reentrant_collection_view().await;

                    let measurement = Instant::now();
                    black_box(
                        view.load_all_entries()
                            .await
                            .expect("Failed to load entries from `ReentrantCollectionView`")
                            .collect::<Vec<_>>(),
                    );
                    total_time += measurement.elapsed();
                }

                total_time
            })
    });

    criterion.bench_function("load_all_entries_from_storage_manually", |bencher| {
        bencher
            .to_async(Runtime::new().expect("Failed to create Tokio runtime"))
            .iter_custom(|iterations| async move {
                let mut total_time = Duration::ZERO;

                for _ in 0..iterations {
                    let view = create_and_store_populated_reentrant_collection_view().await;

                    let measurement = Instant::now();
                    let indices = view
                        .indices()
                        .await
                        .expect("Failed to load all indices from `ReentrantCollectionView`");
                    black_box(
                        view.try_load_entries(&indices)
                            .await
                            .expect("Failed to load entries from `ReentrantCollectionView`"),
                    );
                    total_time += measurement.elapsed();
                }

                total_time
            })
    });
}

async fn create_populated_reentrant_collection_view(
) -> ReentrantCollectionView<MemoryContext<()>, String, RegisterView<MemoryContext<()>, String>> {
    let context = create_memory_context();
    let mut view: ReentrantCollectionView<_, String, RegisterView<_, String>> =
        ReentrantCollectionView::load(context)
            .await
            .expect("Failed to create `ReentrantCollectionView`");

    let greek_alphabet = [
        ("alpha", "α"),
        ("beta", "β"),
        ("gamma", "γ"),
        ("delta", "δ"),
        ("epsilon", "ε"),
        ("zeta", "ζ"),
        ("eta", "η"),
        ("theta", "θ"),
        ("iota", "ι"),
        ("kappa", "κ"),
        ("lambda", "λ"),
        ("mu", "μ"),
        ("nu", "ν"),
        ("xi", "ξ"),
        ("omicron", "ο"),
        ("pi", "π"),
        ("rho", "ρ"),
        ("sigma", "σ"),
        ("tau", "τ"),
        ("upsilon", "υ"),
        ("phi", "φ"),
        ("chi", "χ"),
        ("psi", "ψ"),
        ("omega", "ω"),
    ];

    for (name, letter) in greek_alphabet {
        view.try_load_entry_mut(name)
            .await
            .expect("Failed to create entry in `ReentrantCollectionView`")
            .set(letter.to_owned());
    }

    view
}

async fn create_and_store_populated_reentrant_collection_view(
) -> ReentrantCollectionView<MemoryContext<()>, String, RegisterView<MemoryContext<()>, String>> {
    let mut view = create_populated_reentrant_collection_view().await;
    let context = view.context().clone();
    let mut batch = Batch::new();
    view.flush(&mut batch)
        .expect("Failed to flush popluated `ReentrantCollectionView`'s contents");
    context
        .write_batch(batch)
        .await
        .expect("Failed to store popluated `ReentrantCollectionView`'s contents");

    ReentrantCollectionView::load(context)
        .await
        .expect("Failed to create second `ReentrantCollectionView`")
}

criterion_group!(
    benches,
    bench_load_all_entries_already_in_memory,
    bench_load_all_entries_from_storage
);
criterion_main!(benches);
