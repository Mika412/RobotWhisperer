use std::collections::BTreeMap;
use std::time::Duration;

use rw_canonical::CanonicalValue;
use rw_transport::Transport;
use rw_transport_foxglove_ws::{FoxgloveConfig, FoxgloveTransport};
use tokio::time::{timeout, Instant};

fn env_or(key: &str, fallback: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| fallback.to_string())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn foxglove_action_streams_feedback_and_result() {
    if std::env::var("RW_INTEGRATION").ok().as_deref() != Some("1") {
        eprintln!("skipping live test: RW_INTEGRATION=1 not set");
        return;
    }
    let url = env_or("RW_FOXGLOVE_URL", "ws://127.0.0.1:8765");
    let action = env_or("RW_FG_ACTION", "/fibonacci");
    let goal_field = env_or("RW_FG_GOAL_FIELD", "order");
    let goal_value: i64 = env_or("RW_FG_GOAL_VALUE", "5").parse().unwrap_or(5);

    let transport = FoxgloveTransport::new(FoxgloveConfig::new(url));
    transport.connect().await.expect("foxglove connect");

    let mut discovery = transport.discovery();
    let mut tries = 40;
    loop {
        let snapshot = discovery.borrow_and_update().clone();
        if snapshot.actions.iter().any(|entry| entry.name == action) {
            break;
        }
        if tries == 0 {
            panic!("action {action} never appeared in discovery; is the action server running?");
        }
        tries -= 1;
        let _ = timeout(Duration::from_millis(500), discovery.changed()).await;
    }

    let goal = CanonicalValue::Struct(BTreeMap::from([(
        goal_field,
        CanonicalValue::Int(goal_value),
    )]));

    let mut stream = transport
        .send_action_goal(&action, goal)
        .await
        .expect("send_action_goal accepted");

    let mut feedback_seen = 0u32;
    let deadline = Instant::now() + Duration::from_secs(30);
    let result = loop {
        tokio::select! {
            Some(feedback) = stream.feedback.recv() => {
                feedback_seen += 1;
                eprintln!("feedback #{feedback_seen}: {feedback:?}");
            }
            outcome = &mut stream.result => {
                break outcome.expect("result channel stayed open");
            }
            _ = tokio::time::sleep(Duration::from_secs(1)) => {
                if Instant::now() > deadline {
                    panic!("action did not finish in 30s; feedback_seen={feedback_seen}");
                }
            }
        }
    };

    let value = result.expect("action goal returned ok");
    eprintln!("action result: {value:?}");
    assert!(
        !matches!(value, CanonicalValue::Null),
        "expected a non-null action result"
    );

    transport.disconnect().await.ok();
}
