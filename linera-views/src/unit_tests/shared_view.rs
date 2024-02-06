// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Tests the behavior of [`SharedView`].

use async_trait::async_trait;
use futures::{stream::FuturesUnordered, FutureExt, StreamExt};
use linera_views::{
    memory::{create_memory_context, MemoryContext},
    register_view::RegisterView,
    shared_view::SharedView,
    views::{RootView, View, ViewError},
};
use std::{fmt::Debug, marker::PhantomData, mem, time::Duration};
use test_case::test_case;
use tokio::time::sleep;

/// Test if a [`View`] can be shared among multiple readers.
#[test_case(PhantomData::<ShareRegisterView<_>>; "with RegisterView")]
#[tokio::test(start_paused = true)]
async fn test_multiple_readers<V>(_view_type: PhantomData<V>) -> Result<(), ViewError>
where
    V: ShareViewTest,
{
    let context = create_memory_context();

    let mut view = V::load(context).await?;
    let staged_value = view.stage_changes().await?;

    let mut shared_view = SharedView::new(view);

    let tasks = FuturesUnordered::new();

    for _ in 0..100 {
        let reference = shared_view
            .inner()
            .now_or_never()
            .expect("Read-only reference should be immediately available")?;

        let task = tokio::spawn(async move {
            sleep(Duration::from_millis(10)).await;
            reference.read().await
        });

        tasks.push(task);
    }

    tasks
        .for_each_concurrent(100, |result| async {
            let read_value = result
                .expect("Read task should not panic")
                .expect("Reading through read-only view reference should not fail");
            assert_eq!(read_value, staged_value);
        })
        .await;

    Ok(())
}

/// Test if a [`View`] is shared with at most one writer.
#[test_case(PhantomData::<ShareRegisterView<_>>; "with RegisterView")]
#[tokio::test(start_paused = true)]
async fn test_if_second_writer_waits_for_first_writer<V>(
    _view_type: PhantomData<V>,
) -> Result<(), ViewError>
where
    V: ShareViewTest,
{
    let context = create_memory_context();
    let dummy_view = V::load(context).await?;
    let mut shared_view = SharedView::new(dummy_view);

    let writer_reference = shared_view
        .inner_mut()
        .now_or_never()
        .expect("First read-write reference should be immediately available");

    assert!(
        shared_view.inner_mut().now_or_never().is_none(),
        "Second read-write reference should wait for first writer to finish"
    );

    mem::drop(writer_reference);

    let _second_writer_reference = shared_view.inner_mut().now_or_never().expect(
        "Second read-write reference should be immediately available after the first writer \
        finishes",
    );

    Ok(())
}

/// Test if a [`View`] stops sharing with new readers when it is shared with one writer.
#[tokio::test(start_paused = true)]
async fn test_writer_blocks_new_readers() -> Result<(), ViewError> {
    let context = create_memory_context();
    let dummy_view = SimpleView::load(context).await?;
    let mut shared_view = SharedView::new(dummy_view);

    let _first_reader_reference = shared_view
        .inner()
        .now_or_never()
        .expect("Initial read-only references should be immediately available");
    let _second_reader_reference = shared_view
        .inner()
        .now_or_never()
        .expect("Initial read-only references should be immediately available");

    let writer_reference = shared_view
        .inner_mut()
        .now_or_never()
        .expect("First read-write reference should be immediately available");

    assert!(
        shared_view.inner().now_or_never().is_none(),
        "Read-only references should wait for writer to finish"
    );

    mem::drop(writer_reference);

    let _third_reader_reference = shared_view.inner().now_or_never().expect(
        "Third read-only reference should be immediately available after the writer finishes",
    );

    Ok(())
}

/// Test if writer waits for readers to finish before saving.
#[tokio::test(start_paused = true)]
async fn test_writer_waits_for_readers() -> Result<(), ViewError> {
    let context = create_memory_context();
    let dummy_view = SimpleView::load(context).await?;
    let mut shared_view = SharedView::new(dummy_view);

    let reader_delays = [100, 300, 250, 200, 150, 400, 200]
        .into_iter()
        .map(Duration::from_millis);

    let reader_tasks = FuturesUnordered::new();

    for delay in reader_delays {
        let reader_reference = shared_view.inner().await?;

        reader_tasks.push(tokio::spawn(async move {
            let _reader_reference = reader_reference;
            sleep(delay).await;
        }));
    }

    let mut writer_reference = shared_view.inner_mut().await?;
    writer_reference.save().await?;

    let readers_collector =
        reader_tasks.for_each(|task_result| async move { assert!(task_result.is_ok()) });

    assert_eq!(
        readers_collector.now_or_never(),
        Some(()),
        "Reader tasks should have finished before the writer saved, so collecting the task \
        results should finish immediately"
    );

    Ok(())
}

/// A [`View`] to be used in the [`SharedView`] tests.
#[async_trait]
trait ShareViewTest: RootView<MemoryContext<()>> + Send + 'static {
    /// Representation of the view's state.
    type State: Debug + Eq + Send;

    /// Performs some changes to the view, staging them, and returning a representation of the
    /// view's state.
    async fn stage_changes(&mut self) -> Result<Self::State, ViewError>;

    /// Reads the view's current state.
    async fn read(&self) -> Result<Self::State, ViewError>;
}

/// Wrapper to test sharing a [`RegisterView`].
#[derive(RootView)]
struct ShareRegisterView<C> {
    byte: RegisterView<C, u8>,
}

#[async_trait]
impl ShareViewTest for ShareRegisterView<MemoryContext<()>> {
    type State = u8;

    async fn stage_changes(&mut self) -> Result<Self::State, ViewError> {
        let dummy_value = 82;
        self.byte.set(dummy_value);
        Ok(dummy_value)
    }

    async fn read(&self) -> Result<Self::State, ViewError> {
        Ok(*self.byte.get())
    }
}
