import { defineStore } from "pinia";
import {
  accountInfo,
  configPath,
  defaultConfig,
  defaultServerStatus,
  generateImage,
  loadConfig,
  loginBrowser,
  logout,
  saveConfig,
  serverStatus,
  startServer,
  stopServer,
  type AccountInfo,
  type AppConfig,
  type ImageCommandRequest,
  type ServerStatus,
} from "../api/tauri";
import { applyTheme } from "../theme";

interface AppStoreState {
  account: AccountInfo;
  config: AppConfig;
  server: ServerStatus;
  configPath: string;
  logs: string[];
  lastGeneratedImage: string | null;
  error: string | null;
}

export const useAppStore = defineStore("app", {
  state: (): AppStoreState => {
    const config = defaultConfig();
    return {
      account: { logged_in: false, email: null, account_id: null, expires_at: null },
      config,
      server: defaultServerStatus(config),
      configPath: "",
      logs: [],
      lastGeneratedImage: null,
      error: null,
    };
  },
  actions: {
    async load() {
      try {
        this.config = await loadConfig();
        this.server = await serverStatus();
        this.account = await accountInfo();
        this.configPath = await configPath();
      } catch (error) {
        this.error = String(error);
      } finally {
        applyTheme(this.config.ui.theme);
      }
    },
    async save() {
      this.config = await saveConfig(this.config);
      applyTheme(this.config.ui.theme);
    },
    async start() {
      this.server = await startServer();
    },
    async stop() {
      this.server = await stopServer();
    },
    async login() {
      this.account = await loginBrowser();
    },
    async logout() {
      this.account = await logout();
    },
    async generateCurrentImage(prompt: string) {
      const request: ImageCommandRequest = {
        prompt,
        model: this.config.image.default_model,
        size: this.config.image.size,
        quality: this.config.image.quality,
        background: this.config.image.background,
        output_format: this.config.image.output_format,
        output_compression: this.config.image.output_compression ?? undefined,
      };
      try {
        const response = await generateImage(request);
        this.lastGeneratedImage = response.b64_json;
        this.error = null;
      } catch (error) {
        this.error = String(error);
      }
    },
  },
});
