use rw_validation::{is_reachable, require_integration_env, Bridge};

#[tokio::test]
async fn all_four_bridges_are_reachable() {
    require_integration_env!();

    let mut failures = Vec::new();
    for bridge in Bridge::ALL {
        let ok = is_reachable(bridge).await;
        eprintln!(
            "  {:>6}  {:<16}  port={}",
            if ok { "OK" } else { "FAIL" },
            bridge.label(),
            bridge.host_port()
        );
        if !ok {
            failures.push(bridge);
        }
    }

    assert!(
        failures.is_empty(),
        "{} bridges unreachable: {:?}. Run ./scripts/whisperer.sh bridges-all and re-probe.",
        failures.len(),
        failures.iter().map(|b| b.label()).collect::<Vec<_>>()
    );
}
