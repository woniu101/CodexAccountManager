import { defineStore } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import type { DashboardState, LoginProgress, ManagedAccount } from "../types/account";

let loginTimer: number | undefined;
let loginPolling = false;
let loginGeneration = 0;
let activeLoad: Promise<void> | undefined;
let noticeTimer: number | undefined;

export const useAccountStore = defineStore("accounts", {
  state: () => ({
    accounts: [] as ManagedAccount[],
    codexRunning: false,
    refreshedAt: undefined as number | undefined,
    loading: false,
    switchingId: undefined as string | undefined,
    addingAccount: false,
    cancellingLogin: false,
    login: { status: "idle" } as LoginProgress,
    notice: undefined as string | undefined,
    error: undefined as string | undefined,
  }),
  getters: {
    current: (state) => state.accounts.find((account) => account.isActive) ?? state.accounts[0],
  },
  actions: {
    showNotice(message: string) {
      this.notice = message;
      window.clearTimeout(noticeTimer);
      noticeTimer = window.setTimeout(() => { this.notice = undefined; }, 2200);
    },
    applyDashboard(dashboard: DashboardState) {
      this.accounts = dashboard.accounts;
      this.codexRunning = dashboard.codexRunning;
      this.refreshedAt = dashboard.refreshedAt;
    },
    async load(forceAfterCurrent = false) {
      if (activeLoad) {
        await activeLoad;
        if (!forceAfterCurrent) return;
      }
      const request = (async () => {
        this.loading = true;
        this.error = undefined;
        try {
          this.applyDashboard(await invoke<DashboardState>("load_dashboard"));
        } catch (error) {
          this.error = String(error);
        } finally {
          this.loading = false;
        }
      })();
      activeLoad = request;
      try {
        await request;
      } finally {
        if (activeLoad === request) activeLoad = undefined;
      }
    },
    async importCurrent() {
      this.loading = true;
      this.error = undefined;
      try {
        this.applyDashboard(await invoke<DashboardState>("import_current_account"));
      } catch (error) {
        this.error = String(error);
      } finally {
        this.loading = false;
      }
    },
    async startAddAccount() {
      if (this.addingAccount || this.login.status === "waiting") return;
      const generation = ++loginGeneration;
      this.error = undefined;
      this.notice = undefined;
      this.addingAccount = true;
      try {
        this.login = await invoke<LoginProgress>("start_add_account");
        if (this.login.status === "waiting") {
          try {
            await openUrl(this.login.authUrl);
          } catch (error) {
            await invoke("cancel_add_account");
            throw error;
          }
          window.clearInterval(loginTimer);
          loginTimer = window.setInterval(() => void this.pollLogin(generation), 1200);
        }
      } catch (error) {
        this.login = { status: "failed", message: String(error) };
      } finally {
        this.addingAccount = false;
      }
    },
    async pollLogin(generation = loginGeneration) {
      if (generation !== loginGeneration || loginPolling) return;
      loginPolling = true;
      try {
        const progress = await invoke<LoginProgress>("poll_add_account");
        if (generation !== loginGeneration) return;
        this.login = progress;
        if (this.login.status === "completed") {
          window.clearInterval(loginTimer);
          await this.load(true);
          this.login = { status: "idle" };
          this.showNotice("账号已添加，列表和用量已刷新");
        } else if (this.login.status === "duplicate") {
          window.clearInterval(loginTimer);
        } else if (this.login.status === "failed") {
          window.clearInterval(loginTimer);
        }
      } catch (error) {
        if (generation !== loginGeneration) return;
        window.clearInterval(loginTimer);
        this.login = { status: "failed", message: String(error) };
      } finally {
        loginPolling = false;
      }
    },
    async cancelLogin() {
      if (this.cancellingLogin) return;
      loginGeneration += 1;
      window.clearInterval(loginTimer);
      this.cancellingLogin = true;
      try {
        await invoke("cancel_add_account");
        this.login = { status: "idle" };
        this.showNotice("已取消添加账号");
      } catch (error) {
        this.login = { status: "failed", message: `取消授权失败：${String(error)}` };
      } finally {
        this.cancellingLogin = false;
      }
    },
    async confirmDuplicate(overwrite: boolean) {
      try {
        this.login = await invoke<LoginProgress>("confirm_add_account", { overwrite });
        if (this.login.status === "completed") {
          await this.load(true);
          this.login = { status: "idle" };
          this.showNotice("账号凭据已更新，用量已刷新");
        } else if (!overwrite && this.login.status === "idle") {
          this.showNotice("已取消更新重复账号");
        }
      } catch (error) {
        this.login = { status: "failed", message: String(error) };
      }
    },
    async switchTo(account: ManagedAccount) {
      if (account.isActive || this.switchingId) return;
      this.switchingId = account.id;
      this.error = undefined;
      try {
        this.applyDashboard(
          await invoke<DashboardState>("switch_account", { accountId: account.id }),
        );
      } catch (error) {
        this.error = String(error);
      } finally {
        this.switchingId = undefined;
      }
    },
    async removeAccount(account: ManagedAccount) {
      if (account.isActive) {
        this.error = "请先切换到其他账号，再删除当前账号";
        return;
      }
      this.loading = true;
      this.error = undefined;
      try {
        this.applyDashboard(
          await invoke<DashboardState>("remove_account", { accountId: account.id }),
        );
      } catch (error) {
        this.error = String(error);
      } finally {
        this.loading = false;
      }
    },
  },
});
