#[cfg(not(target_arch = "wasm32"))]
use crate::futures::executor::ThreadPool;
use parking_lot::Mutex;
use std::{
    any::Any,
    future::Future,
    sync::mpsc::{self, Receiver, Sender},
};
use uuid::Uuid;

// ========
// Non-WASM
#[cfg(not(target_arch = "wasm32"))]
pub trait AsyncTaskResult: Any + Send + 'static {
    fn into_any(self: Box<Self>) -> Box<dyn Any>;
}

#[cfg(not(target_arch = "wasm32"))]
impl<T> AsyncTaskResult for T
where
    T: Any + Send + 'static,
{
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub trait AsyncTask<R: AsyncTaskResult>: Future<Output = R> + Send + 'static {}

#[cfg(not(target_arch = "wasm32"))]
impl<T, R: AsyncTaskResult> AsyncTask<R> for T where T: Future<Output = R> + Send + 'static {}

// ========
// WASM
#[cfg(target_arch = "wasm32")]
pub trait AsyncTaskResult: Any + 'static {
    fn into_any(self: Box<Self>) -> Box<dyn Any>;
}

#[cfg(target_arch = "wasm32")]
impl<T> AsyncTaskResult for T
where
    T: Any + 'static,
{
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}

#[cfg(target_arch = "wasm32")]
pub trait AsyncTask<R: AsyncTaskResult>: Future<Output = R> + 'static {}

#[cfg(target_arch = "wasm32")]
impl<T, R: AsyncTaskResult> AsyncTask<R> for T where T: Future<Output = R> + 'static {}

// ========
// Common
impl dyn AsyncTaskResult {
    pub fn downcast<T: AsyncTaskResult>(self: Box<Self>) -> Result<Box<T>, Box<dyn Any>> {
        self.into_any().downcast()
    }
}

pub struct TaskResult {
    pub id: Uuid,
    pub payload: Box<dyn AsyncTaskResult>,
}

pub struct TaskPool {
    #[cfg(not(target_arch = "wasm32"))]
    thread_pool: ThreadPool,
    sender: Sender<TaskResult>,
    receiver: Mutex<Receiver<TaskResult>>,
}

impl Default for TaskPool {
    fn default() -> Self {
        Self::new()
    }
}

impl TaskPool {
    #[inline]
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel();
        Self {
            #[cfg(not(target_arch = "wasm32"))]
            thread_pool: ThreadPool::new().unwrap(),
            sender,
            receiver: Mutex::new(receiver),
        }
    }

    #[inline]
    #[cfg(target_arch = "wasm32")]
    pub fn spawn_task<F>(&self, future: F)
    where
        F: Future<Output = ()> + 'static,
    {
        crate::wasm_bindgen_futures::spawn_local(future);
    }

    #[inline]
    #[cfg(not(target_arch = "wasm32"))]
    pub fn spawn_task<F>(&self, future: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        self.thread_pool.spawn_ok(future);
    }

    #[inline]
    pub fn spawn_with_result<F, T>(&self, future: F) -> Uuid
    where
        F: AsyncTask<T>,
        T: AsyncTaskResult,
    {
        let id = Uuid::new_v4();
        let sender = self.sender.clone();
        self.spawn_task(async move {
            let result = future.await;
            sender
                .send(TaskResult {
                    id,
                    payload: Box::new(result),
                })
                .unwrap();
        });
        id
    }

    #[inline]
    pub fn next_task_result(&self) -> Option<TaskResult> {
        self.receiver.lock().try_recv().ok()
    }
}
