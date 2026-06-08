use rw_transport::Transport;
use rw_transport_foxglove_ws::{FoxgloveConfig, FoxgloveTransport};
use std::time::Duration;
use tokio::time::timeout;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn probe_ros1_fox() {
    if std::env::var("RW_PROBE").as_deref() != Ok("1") {
        eprintln!("skipping (set RW_PROBE=1)");
        return;
    }
    let mut cfg = FoxgloveConfig::new("ws://127.0.0.1:8765");
    cfg.connect_timeout = Duration::from_secs(5);
    let transport = FoxgloveTransport::new(cfg);
    transport.connect().await.expect("connect");
    eprintln!("connected ok, waiting for discovery...");
    tokio::time::sleep(Duration::from_secs(3)).await;
    let disc = transport.discovery().borrow().clone();
    eprintln!("topics: {}", disc.topics.len());
    assert!(!disc.topics.is_empty(), "no topics advertised");

    let mut subs: Vec<(String, _)> = Vec::new();
    for t in &disc.topics {
        match transport.subscribe_topic(&t.name).await {
            Ok(s) => subs.push((t.name.clone(), s)),
            Err(err) => eprintln!("subscribe {}: {}", t.name, err),
        }
    }
    eprintln!("subscribed to {} topics", subs.len());
    let frame_fut = async {
        let mut futures = futures::stream::FuturesUnordered::new();
        for (name, mut sub) in subs {
            futures.push(async move {
                let f = sub.frames.recv().await;
                (name, f)
            });
        }
        use futures::StreamExt;
        futures.next().await
    };
    let (name, frame) = timeout(Duration::from_secs(30), frame_fut)
        .await
        .expect("any frame within 30s")
        .expect("at least one subscription");
    let frame = frame.expect("frame channel open");
    eprintln!("got frame on {name} (schema {})", frame.schema.name);
}
