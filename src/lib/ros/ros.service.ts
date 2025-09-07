import { browser } from "$app/environment";
import type { ActionInfo, ServiceInfo, TopicInfo } from "./types";
// import { NativeTauriProvider } from "./nativeTauriProvider";
import { WSFoxgloveProvider } from "./wsFoxgloveProvider";
import type { ROSProvider } from "./provider";
import { topics, services, actions, connectionInfo, connectionStatus } from "$lib/stores/rosStore";

export function createRosService() {
  // let topics = $state<TopicInfo[]>([]);
  // let services = $state<ServiceInfo[]>([]);
  // let actions = $state<ActionInfo[]>([]);
  // let connectionStatus = $state<"connected" | "disconnected">("disconnected");
  // let connectionInfo = $state<string | null>(null);

  let provider: ROSProvider | null = null;

  async function connect(url?: string) {
    // if (!browser) return;

    // // In a Tauri environment, use the native provider
    // if (window.__TAURI__) {
    //   provider = new NativeTauriProvider();
    // } else {
    // }
    provider = new WSFoxgloveProvider();

    // provider.setEvents({
    //   onStatus: (status) => {
    //     connectionStatus = status.connected ? "connected" : "disconnected";
    //     connectionInfo = status.info ?? null;
    //   },
    //   onTopics: (topicList) => {
    //     topics = topicList;
    //   },
    //   onServices: (serviceList) => {
    //     services = serviceList;
    //   },
    //   onActions: (actionList) => {
    //     actions = actionList;
    //   },
    // });

    try {
      await provider.connect({ url });
    } catch (error) {
      console.error("Failed to connect to ROS:", error);
      connectionStatus = "disconnected";
    }
  }

  async function disconnect() {
    if (provider) {
      await provider.disconnect();
      provider = null;
    }
    connectionStatus = "disconnected";
    connectionInfo = null;
    topics = [];
    services = [];
    actions = [];
  }

  return {
    connect,
    disconnect,
    get topics() {
      return topics;
    },
    get services() {
      return services;
    },
    get actions() {
      return actions;
    },
    get connectionStatus() {
      return connectionStatus;
    },
    get connectionInfo() {
      return connectionInfo;
    },
  };
}

export type RosService = ReturnType<typeof createRosService>;
