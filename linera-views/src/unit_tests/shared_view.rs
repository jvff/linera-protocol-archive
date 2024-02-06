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
    let staged_state = view.stage_initial_changes().await?;

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
            let read_state = result
                .expect("Read task should not panic")
                .expect("Reading through read-only view reference should not fail");
            assert_eq!(read_state, staged_state);
        })
        .await;

    Ok(())
}

/// Test if readers can't see the writer's staged changes.
#[test_case(PhantomData::<ShareCollectionView<_>>; "with CollectionView")]
#[test_case(PhantomData::<ShareLogView<_>>; "with LogView")]
#[test_case(PhantomData::<ShareMapView<_>>; "with MapView")]
#[test_case(PhantomData::<ShareRegisterView<_>>; "with RegisterView")]
#[tokio::test(start_paused = true)]
async fn test_writer_staged_changes_are_private<V>(
    _view_type: PhantomData<V>,
) -> Result<(), ViewError>
where
    V: ShareViewTest,
{
    let context = create_memory_context();

    let mut view = V::load(context).await?;
    let initial_value = view.stage_initial_changes().await?;

    let mut shared_view = SharedView::new(view);

    let tasks = FuturesUnordered::new();

    for _ in 0..100 {
        let reference = shared_view.inner().await?;

        let task = tokio::spawn(async move {
            sleep(Duration::from_millis(10)).await;
            reference.read().await
        });

        tasks.push(task);
    }

    let mut writer_reference = shared_view
        .inner_mut()
        .now_or_never()
        .expect("Read-write reference should be immediately available")?;
    writer_reference.stage_changes_to_be_discarded().await?;

    tasks
        .for_each_concurrent(100, |result| async {
            let read_value = result
                .expect("Read task should not panic")
                .expect("Reading through read-only view reference should not fail");
            assert_eq!(read_value, initial_value);
        })
        .await;

    Ok(())
}

/// Test if readers can't see the writer's persisted changes.
#[test_case(PhantomData::<ShareCollectionView<_>>; "with CollectionView")]
#[test_case(PhantomData::<ShareLogView<_>>; "with LogView")]
#[test_case(PhantomData::<ShareMapView<_>>; "with MapView")]
#[test_case(PhantomData::<ShareRegisterView<_>>; "with RegisterView")]
#[tokio::test(start_paused = true)]
async fn test_writer_persisted_changes_are_not_visible_to_readers<V>(
    _view_type: PhantomData<V>,
) -> Result<(), ViewError>
where
    V: ShareViewTest,
{
    let context = create_memory_context();

    let mut view = V::load(context).await?;
    let initial_value = view.stage_initial_changes().await?;

    let mut shared_view = SharedView::new(view);

    let tasks = FuturesUnordered::new();

    for _ in 0..100 {
        let reference = shared_view.inner().await?;

        let task = tokio::spawn(async move {
            sleep(Duration::from_millis(10)).await;
            reference.read().await
        });

        tasks.push(task);
    }

    let mut writer_reference = shared_view
        .inner_mut()
        .now_or_never()
        .expect("Read-write reference should be immediately available")?;
    writer_reference.stage_changes_to_be_persisted().await?;
    writer_reference.save().await?;

    tasks
        .for_each_concurrent(100, |result| async {
            let read_value = result
                .expect("Read task should not panic")
                .expect("Reading through read-only view reference should not fail");
            assert_eq!(read_value, initial_value);
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
    let dummy_view = V::load(context).await?;
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

/// Test if writer runs in parallel with readers if it doesn't save.
#[test_case(PhantomData::<ShareCollectionView<_>>; "with CollectionView")]
#[test_case(PhantomData::<ShareLogView<_>>; "with LogView")]
#[test_case(PhantomData::<ShareMapView<_>>; "with MapView")]
#[test_case(PhantomData::<ShareRegisterView<_>>; "with RegisterView")]
#[tokio::test(start_paused = true)]
async fn test_readers_run_in_parallel_with_writer_while_its_not_saving<V>(
    _view_type: PhantomData<V>,
) -> Result<(), ViewError>
where
    V: ShareViewTest,
{
    let context = create_memory_context();
    let view = V::load(context).await?;
    let mut shared_view = SharedView::new(view);

    let reader_delays = [100, 300, 250, 200, 150, 400, 200]
        .into_iter()
        .map(Duration::from_millis);

    let mut reader_tasks = FuturesUnordered::new();

    for delay in reader_delays {
        let reader_reference = shared_view.inner().await?;

        reader_tasks.push(tokio::spawn(async move {
            let _reader_reference = reader_reference;
            sleep(delay).await;
        }));
    }

    let _writer_reference = shared_view.inner_mut().await?;

    assert!(
        reader_tasks.next().now_or_never().is_none(),
        "Reader tasks should still be executing after a writer starts"
    );

    while let Some(task_result) = reader_tasks.next().await {
        assert!(task_result.is_ok());
    }

    Ok(())
}

/// A [`View`] to be used in the [`SharedView`] tests.
#[async_trait]
trait ShareViewTest: RootView<MemoryContext<()>> + Send + 'static {
    /// Representation of the view's state.
    type State: Debug + Eq + Send;

    /// Performs some initial changes to the view, staging them, and returning a representation of
    /// the view's state.
    async fn stage_initial_changes(&mut self) -> Result<Self::State, ViewError>;

    /// Stages some changes to the view that won't be persisted during the test.
    async fn stage_changes_to_be_discarded(&mut self) -> Result<(), ViewError>;

    /// Stages some changes to the view that will be persisted during the test.
    ///
    /// Assumes that the current view state is the initially staged changes. Returns the updated
    /// state.
    async fn stage_changes_to_be_persisted(&mut self) -> Result<Self::State, ViewError>;

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

    async fn stage_initial_changes(&mut self) -> Result<Self::State, ViewError> {
        let dummy_value = 82;
        self.byte.set(dummy_value);
        Ok(dummy_value)
    }

    async fn stage_changes_to_be_discarded(&mut self) -> Result<(), ViewError> {
        self.byte.set(209);
        Ok(())
    }

    async fn stage_changes_to_be_persisted(&mut self) -> Result<Self::State, ViewError> {
        let dummy_value = 15;
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

    async fn stage_initial_changes(&mut self) -> Result<Self::State, ViewError> {
        let dummy_values = [1, 2, 3, 4, 5];

        for value in dummy_values {
            self.log.push(value);
        }

        Ok(dummy_values.to_vec())
    }

    async fn stage_changes_to_be_discarded(&mut self) -> Result<(), ViewError> {
        for value in [10_000, 20_000, 30_000] {
            self.log.push(value);
        }

        Ok(())
    }

    async fn stage_changes_to_be_persisted(&mut self) -> Result<Self::State, ViewError> {
        let initial_state = [1, 2, 3, 4, 5];
        let new_values = [201, 1, 50_050];

        for value in new_values {
            self.log.push(value);
        }

        Ok(initial_state.into_iter().chain(new_values).collect())
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

    async fn stage_initial_changes(&mut self) -> Result<Self::State, ViewError> {
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

    async fn stage_changes_to_be_discarded(&mut self) -> Result<(), ViewError> {
        let new_entries = [(-1_000_000, "foo"), (2_000_000, "bar")]
            .into_iter()
            .map(|(key, value)| (key, value.to_owned()));

        let entries_to_remove = [0, -3];

        for (key, value) in new_entries {
            self.map.insert(&key, value)?;
        }

        for key in entries_to_remove {
            self.map.remove(&key)?;
        }

        Ok(())
    }

    async fn stage_changes_to_be_persisted(&mut self) -> Result<Self::State, ViewError> {
        let new_entries = [(1_234, "first new entry"), (-2_101_010, "second_new_entry")]
            .into_iter()
            .map(|(key, value)| (key, value.to_owned()));

        let entries_to_remove = [-1, 2, 4];

        for (key, value) in new_entries.clone() {
            self.map.insert(&key, value)?;
        }

        for key in entries_to_remove {
            self.map.remove(&key)?;
        }

        let initial_state = [
            (0, "zero"),
            (-1, "minus one"),
            (2, "two"),
            (-3, "minus three"),
            (4, "four"),
            (-5, "minus five"),
        ];

        let new_state = initial_state
            .into_iter()
            .filter(|(key, _)| !entries_to_remove.contains(key))
            .map(|(key, value)| (key, value.to_owned()))
            .chain(new_entries)
            .collect();

        Ok(new_state)
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

    async fn stage_initial_changes(&mut self) -> Result<Self::State, ViewError> {
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

    async fn stage_changes_to_be_discarded(&mut self) -> Result<(), ViewError> {
        let new_entries = [(-1_000_000, "foo"), (2_000_000, "bar")]
            .into_iter()
            .map(|(key, value)| (key, value.to_owned()));

        let entries_to_remove = [0, -3];

        for (key, value) in new_entries {
            self.collection.load_entry_mut(&key).await?.set(value);
        }

        for key in entries_to_remove {
            self.collection.remove_entry(&key)?;
        }

        Ok(())
    }

    async fn stage_changes_to_be_persisted(&mut self) -> Result<Self::State, ViewError> {
        let new_entries = [(1_234, "first new entry"), (-2_101_010, "second_new_entry")]
            .into_iter()
            .map(|(key, value)| (key, value.to_owned()));

        let entries_to_remove = [-1, 2, 4];

        for (key, value) in new_entries.clone() {
            self.collection.load_entry_mut(&key).await?.set(value);
        }

        for key in entries_to_remove {
            self.collection.remove_entry(&key)?;
        }

        let initial_state = [
            (0, "zero"),
            (-1, "minus one"),
            (2, "two"),
            (-3, "minus three"),
            (4, "four"),
            (-5, "minus five"),
        ];

        let new_state = initial_state
            .into_iter()
            .filter(|(key, _)| !entries_to_remove.contains(key))
            .map(|(key, value)| (key, value.to_owned()))
            .chain(new_entries)
            .collect();

        Ok(new_state)
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
