import { invoke } from "@tauri-apps/api/core";

export type ThemeMode = "system" | "light" | "dark";
export type AppLocaleSetting = "system" | "en" | "zh";

export interface AppConfig {
  server: {
    host: string;
    port: number;
    login_callback_port: number;
    allow_external_bind: boolean;
  };
  api: {
    default_model: string;
    expose_reasoning_models: boolean;
    upstream_base_url: string;
  };
  reasoning: {
    effort: string;
    summary: string | null;
    compat: string;
  };
  text: {
    verbosity: string;
  };
  image: {
    default_model: string;
    size: string;
    quality: string;
    background: string;
    output_format: string;
    output_compression: number | null;
  };
  features: {
    fast_mode: boolean;
    enable_web_search: boolean;
    enable_image_api: boolean;
    enable_responses_websocket: boolean;
  };
  ui: {
    locale: AppLocaleSetting;
    theme: ThemeMode;
  };
}

export interface ServerStatus {
  running: boolean;
  host: string;
  port: number;
  url: string;
}

export interface AccountInfo {
  logged_in: boolean;
  email: string | null;
  account_id: string | null;
  expires_at: string | null;
}

export interface ImageCommandRequest {
  prompt: string;
  model?: string;
  size?: string;
  quality?: string;
  background?: string;
  output_format?: string;
  output_compression?: number;
}

export interface ImageCommandResponse {
  b64_json: string;
}

export function defaultConfig(): AppConfig {
  return {
    server: {
      host: "127.0.0.1",
      port: 14550,
      login_callback_port: 1455,
      allow_external_bind: false,
    },
    api: {
      default_model: "gpt-5.5",
      expose_reasoning_models: true,
      upstream_base_url: "https://chatgpt.com/backend-api/codex/",
    },
    reasoning: {
      effort: "medium",
      summary: "auto",
      compat: "hidden",
    },
    text: {
      verbosity: "medium",
    },
    image: {
      default_model: "chatgpt-image-latest",
      size: "auto",
      quality: "auto",
      background: "auto",
      output_format: "png",
      output_compression: null,
    },
    features: {
      fast_mode: false,
      enable_web_search: true,
      enable_image_api: true,
      enable_responses_websocket: true,
    },
    ui: {
      locale: "system",
      theme: "system",
    },
  };
}

export const defaultServerStatus = (config = defaultConfig()): ServerStatus => ({
  running: false,
  host: config.server.host,
  port: config.server.port,
  url: `http://${config.server.host}:${config.server.port}`,
});

export const loadConfig = () => invoke<AppConfig>("load_config");
export const saveConfig = (config: AppConfig) => invoke<AppConfig>("save_config", { config });
export const startServer = () => invoke<ServerStatus>("start_server");
export const stopServer = () => invoke<ServerStatus>("stop_server");
export const serverStatus = () => invoke<ServerStatus>("server_status");
export const accountInfo = () => invoke<AccountInfo>("account_info");
export const loginBrowser = () => invoke<AccountInfo>("login_browser");
export const logout = () => invoke<AccountInfo>("logout");
export const configPath = () => invoke<string>("config_path");
export const generateImage = (request: ImageCommandRequest) =>
  invoke<ImageCommandResponse>("generate_image", { request });
