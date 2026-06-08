use std::net::SocketAddr;
use std::time::Duration;

use tokio::net::TcpStream;
use tokio::time::timeout;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bridge {
    Ros2Foxglove,
    Ros2Rosbridge,
    Ros1Foxglove,
    Ros1Rosbridge,
}

impl Bridge {
    pub const ALL: [Bridge; 4] = [
        Bridge::Ros2Foxglove,
        Bridge::Ros2Rosbridge,
        Bridge::Ros1Foxglove,
        Bridge::Ros1Rosbridge,
    ];

    pub fn host_port(self) -> u16 {
        match self {
            Bridge::Ros2Foxglove => 9091,
            Bridge::Ros2Rosbridge => 9089,
            Bridge::Ros1Foxglove => 9092,
            Bridge::Ros1Rosbridge => 9090,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Bridge::Ros2Foxglove => "ros2-foxglove",
            Bridge::Ros2Rosbridge => "ros2-rosbridge",
            Bridge::Ros1Foxglove => "ros1-foxglove",
            Bridge::Ros1Rosbridge => "ros1-rosbridge",
        }
    }

    pub fn socket_addr(self) -> SocketAddr {
        SocketAddr::from(([127, 0, 0, 1], self.host_port()))
    }
}

pub async fn is_reachable(bridge: Bridge) -> bool {
    timeout(
        Duration::from_secs(2),
        TcpStream::connect(bridge.socket_addr()),
    )
    .await
    .map(|r| r.is_ok())
    .unwrap_or(false)
}

#[macro_export]
macro_rules! require_integration_env {
    () => {
        if std::env::var("RW_INTEGRATION").ok().as_deref() != Some("1") {
            eprintln!(
                "skipping integration test ({}) because RW_INTEGRATION!=1",
                module_path!()
            );
            return;
        }
    };
}
