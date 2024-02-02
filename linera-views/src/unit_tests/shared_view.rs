// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Tests the behavior of [`SharedView`].

use futures::{stream::FuturesUnordered, FutureExt, StreamExt};
use linera_views::{
    memory::create_memory_context,
    register_view::RegisterView,
    shared_view::SharedView,
    views::{View, ViewError},
};
use linera_views_derive::RootView;
use std::time::Duration;
use tokio::time::sleep;

/// Test if a [`View`] can be shared among multiple readers.
#[tokio::test(start_paused = true)]
async fn test_multiple_readers() -> Result<(), ViewError> {
    let context = create_memory_context();

    let dummy_value = 82;
    let mut dummy_view = SimpleView::load(context).await?;
    dummy_view.byte.set(dummy_value);

    let mut shared_view = SharedView::new(dummy_view);

    let tasks = FuturesUnordered::new();

    for _ in 0..100 {
        let reference = shared_view
            .inner()
            .now_or_never()
            .expect("Read-only reference should be immediately available")?;

        let task = tokio::spawn(async move {
            sleep(Duration::from_millis(10)).await;
            *reference.byte.get()
        });

        tasks.push(task);
    }

    tasks
        .for_each_concurrent(100, |read_value| async {
            assert_eq!(read_value.unwrap(), dummy_value);
        })
        .await;

    Ok(())
}

/// A simple view used to test sharing views.
#[derive(RootView)]
struct SimpleView<C> {
    byte: RegisterView<C, u8>,
}
