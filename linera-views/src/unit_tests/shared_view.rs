// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Tests the behavior of [`SharedView`].

use async_trait::async_trait;
use futures::{stream::FuturesUnordered, FutureExt, StreamExt};
use linera_views::{
    collection_view::CollectionView,
    log_view::LogView,
    map_view::MapView,
    memory::{create_memory_context, MemoryContext},
    register_view::RegisterView,
    shared_view::SharedView,
    views::{RootView, View, ViewError},
};
use std::{collections::HashMap, fmt::Debug, marker::PhantomData, mem, time::Duration};
use test_case::test_case;
use tokio::time::sleep;

/// Test if a [`View`] can be shared among multiple readers.
#[test_case(PhantomData::<ShareCollectionView<_>>; "with CollectionView")]
#[test_case(PhantomData::<ShareLogView<_>>; "with LogView")]
#[test_case(PhantomData::<ShareMapView<_>>; "with MapView")]
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
#[test_case(PhantomData::<ShareCollectionView<_>>; "with CollectionView")]
#[test_case(PhantomData::<ShareLogView<_>>; "with LogView")]
#[test_case(PhantomData::<ShareMapView<_>>; "with MapView")]
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
#[test_case(PhantomData::<ShareCollectionView<_>>; "with CollectionView")]
#[test_case(PhantomData::<ShareLogView<_>>; "with LogView")]
#[test_case(PhantomData::<ShareMapView<_>>; "with MapView")]
#[test_case(PhantomData::<ShareRegisterView<_>>; "with RegisterView")]
#[tokio::test(start_paused = true)]
async fn test_writer_blocks_new_readers<V>(_view_type: PhantomData<V>) -> Result<(), ViewError>
where
    V: ShareViewTest,
{
    let context = create_memory_context();
    let dummy_view = V::load(context).await?;
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
#[test_case(PhantomData::<ShareCollectionView<_>>; "with CollectionView")]
#[test_case(PhantomData::<ShareLogView<_>>; "with LogView")]
#[test_case(PhantomData::<ShareMapView<_>>; "with MapView")]
#[test_case(PhantomData::<ShareRegisterView<_>>; "with RegisterView")]
#[tokio::test(start_paused = true)]
async fn test_writer_waits_for_readers<V>(_view_type: PhantomData<V>) -> Result<(), ViewError>
where
    V: ShareViewTest,
{
    let context = create_memory_context();
    let dummy_view = ShareRegisterView::load(context).await?;
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

/// Wrapper to test sharing a [`LogView`].
#[derive(RootView)]
struct ShareLogView<C> {
    log: LogView<C, u16>,
}

#[async_trait]
impl ShareViewTest for ShareLogView<MemoryContext<()>> {
    type State = Vec<u16>;

    async fn stage_changes(&mut self) -> Result<Self::State, ViewError> {
        let dummy_values = [1, 2, 3, 4, 5];

        for value in dummy_values {
            self.log.push(value);
        }

        Ok(dummy_values.to_vec())
    }

    async fn read(&self) -> Result<Self::State, ViewError> {
        self.log.read(..).await
    }
}

/// Wrapper to test sharing a [`MapView`].
#[derive(RootView)]
struct ShareMapView<C> {
    map: MapView<C, i32, String>,
}

#[async_trait]
impl ShareViewTest for ShareMapView<MemoryContext<()>> {
    type State = HashMap<i32, String>;

    async fn stage_changes(&mut self) -> Result<Self::State, ViewError> {
        let dummy_values = [
            (0, "zero"),
            (-1, "minus one"),
            (2, "two"),
            (-3, "minus three"),
            (4, "four"),
            (-5, "minus five"),
        ]
        .into_iter()
        .map(|(key, value)| (key, value.to_owned()));

        for (key, value) in dummy_values.clone() {
            self.map.insert(&key, value)?;
        }

        Ok(dummy_values.collect())
    }

    async fn read(&self) -> Result<Self::State, ViewError> {
        let mut state = HashMap::new();
        self.map
            .for_each_index_value(|key, value| {
                state.insert(key, value);
                Ok(())
            })
            .await?;
        Ok(state)
    }
}

/// Wrapper to test sharing a [`CollectionView`].
#[derive(RootView)]
struct ShareCollectionView<C> {
    collection: CollectionView<C, i32, RegisterView<C, String>>,
}

#[async_trait]
impl ShareViewTest for ShareCollectionView<MemoryContext<()>> {
    type State = HashMap<i32, String>;

    async fn stage_changes(&mut self) -> Result<Self::State, ViewError> {
        let dummy_values = [
            (0, "zero"),
            (-1, "minus one"),
            (2, "two"),
            (-3, "minus three"),
            (4, "four"),
            (-5, "minus five"),
        ]
        .into_iter()
        .map(|(key, value)| (key, value.to_owned()));

        for (key, value) in dummy_values.clone() {
            self.collection.load_entry_mut(&key).await?.set(value);
        }

        Ok(dummy_values.collect())
    }

    async fn read(&self) -> Result<Self::State, ViewError> {
        let indices = self.collection.indices().await?;
        let mut state = HashMap::with_capacity(indices.len());

        for index in indices {
            if let Some(value) = self.collection.try_load_entry(&index).await? {
                state.insert(index, value.get().clone());
            }
        }

        Ok(state)
    }
}
