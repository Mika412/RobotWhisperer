use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::broadcast;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn forwarder_survives_broadcast_lag() {
    let (tx, mut rx) = broadcast::channel::<Arc<u64>>(4);

    let publisher = tokio::spawn(async move {
        for i in 0..500u64 {
            let _ = tx.send(Arc::new(i));
            if i % 64 == 0 {
                tokio::task::yield_now().await;
            }
        }
        drop(tx);
    });

    let received = Arc::new(AtomicU64::new(0));
    let lag_seen = Arc::new(AtomicBool::new(false));
    let r = received.clone();
    let l = lag_seen.clone();
    let forwarder = tokio::spawn(async move {
        use tokio::sync::broadcast::error::RecvError;
        loop {
            match rx.recv().await {
                Ok(_frame) => {
                    r.fetch_add(1, Ordering::Relaxed);
                }
                Err(RecvError::Lagged(_)) => {
                    l.store(true, Ordering::Relaxed);
                    continue;
                }
                Err(RecvError::Closed) => break,
            }
        }
    });

    tokio::time::timeout(Duration::from_secs(5), publisher)
        .await
        .expect("publisher must finish in 5s")
        .expect("publisher panicked");
    tokio::time::timeout(Duration::from_secs(5), forwarder)
        .await
        .expect("forwarder must finish in 5s after publisher closes")
        .expect("forwarder panicked");

    assert!(
        lag_seen.load(Ordering::Relaxed),
        "expected at least one Lagged signal with this capacity / publisher burst",
    );
    let count = received.load(Ordering::Relaxed);
    assert!(
        count >= 4,
        "forwarder should keep receiving frames after Lagged, \
         the broadcast cap (4) is the lower bound on tail-end deliveries; got {count}",
    );
}
