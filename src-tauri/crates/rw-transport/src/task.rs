#[cfg(not(target_family = "wasm"))]
pub use native::{spawn_detached, spawn_task, SpawnedTask};
#[cfg(target_family = "wasm")]
pub use wasm::{spawn_detached, spawn_task, SpawnedTask};

#[cfg(not(target_family = "wasm"))]
mod native {
    use tokio::task::JoinHandle;

    pub type SpawnedTask = JoinHandle<()>;

    pub fn spawn_task<F>(future: F) -> SpawnedTask
    where
        F: std::future::Future<Output = ()> + Send + 'static,
    {
        tokio::spawn(future)
    }

    pub fn spawn_detached<F>(future: F)
    where
        F: std::future::Future<Output = ()> + Send + 'static,
    {
        tokio::spawn(future);
    }
}

#[cfg(target_family = "wasm")]
mod wasm {
    #[derive(Debug)]
    pub struct SpawnedTask {
        cancel: Option<tokio::sync::oneshot::Sender<()>>,
    }

    impl SpawnedTask {
        pub fn abort(&mut self) {
            if let Some(sender) = self.cancel.take() {
                let _ = sender.send(());
            }
        }
    }

    impl Drop for SpawnedTask {
        fn drop(&mut self) {
            self.abort();
        }
    }

    pub fn spawn_task<F>(future: F) -> SpawnedTask
    where
        F: std::future::Future<Output = ()> + 'static,
    {
        let (cancel, cancel_rx) = tokio::sync::oneshot::channel::<()>();
        wasm_bindgen_futures::spawn_local(async move {
            tokio::select! {
                _ = future => {}
                _ = cancel_rx => {}
            }
        });
        SpawnedTask {
            cancel: Some(cancel),
        }
    }

    pub fn spawn_detached<F>(future: F)
    where
        F: std::future::Future<Output = ()> + 'static,
    {
        wasm_bindgen_futures::spawn_local(future);
    }
}
