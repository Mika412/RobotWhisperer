use std::time::Duration;

#[derive(Debug)]
pub struct Elapsed;

#[cfg(not(target_family = "wasm"))]
pub async fn sleep(duration: Duration) {
    tokio::time::sleep(duration).await;
}

#[cfg(not(target_family = "wasm"))]
pub async fn timeout<F>(duration: Duration, future: F) -> Result<F::Output, Elapsed>
where
    F: std::future::Future,
{
    tokio::time::timeout(duration, future)
        .await
        .map_err(|_| Elapsed)
}

#[cfg(target_family = "wasm")]
pub async fn sleep(duration: Duration) {
    let millis = duration.as_millis().min(i32::MAX as u128) as u32;
    gloo_timers::future::TimeoutFuture::new(millis).await;
}

#[cfg(target_family = "wasm")]
pub async fn timeout<F>(duration: Duration, future: F) -> Result<F::Output, Elapsed>
where
    F: std::future::Future,
{
    let millis = duration.as_millis().min(i32::MAX as u128) as u32;
    let timer = gloo_timers::future::TimeoutFuture::new(millis);
    futures_util::pin_mut!(future);
    futures_util::pin_mut!(timer);
    match futures_util::future::select(future, timer).await {
        futures_util::future::Either::Left((output, _)) => Ok(output),
        futures_util::future::Either::Right((_, _)) => Err(Elapsed),
    }
}
