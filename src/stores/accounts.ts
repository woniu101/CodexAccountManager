import { defineStore } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import type { DashboardState, LoginProgress, ManagedAccount } from "../types/account";

let loginTimer: number | undefined;

export const useAccountStore = defineStore("accounts", {
  state: () => ({
    accounts: [] as ManagedAccount[],
    codexRunning: false,
    refreshedAt: undefined as number | undefined,
    loading: false,
    switchingId: undefined as string | undefined,
    login: { status: "idle" } as LoginProgress,
    error: undefined as string | undefined,
  }),
  getters: {
    current: (state) => state.accounts.find((account) => account.isActive) ?? state.accounts[0],
  },
  actions: {
    applyDashboard(dashboard: DashboardState) {
      this.accounts = dashboard.accounts;
      this.codexRunning = dashboard.codexRunning;
      this.refreshedAt = dashboard.refreshedAt;
    },
    async load() {
      this.loading = true;
      this.error = undefined;
      try {
        this.applyDashboard(await invoke<DashboardState>("load_dashboard"));
      } catch (error) {
        this.error = String(error);
      } finally {
        this.loading = false;
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
      this.error = undefined;
      try {
        this.login = await invoke<LoginProgress>("start_add_account");
        if (this.login.status === "waiting") {
          await openUrl(this.login.authUrl);
          window.clearInterval(loginTimer);
          loginTimer = window.setInterval(() => void this.pollLogin(), 1200);
        }
      } catch (error) {
        this.login = { status: "failed", message: String(error) };
      }
    },
    async pollLogin() {
      try {
        this.login = await invoke<LoginProgress>("poll_add_account");
        if (this.login.status === "completed") {
          window.clearInterval(loginTimer);
          await this.load();
        } else if (this.login.status === "duplicate") {
          window.clearInterval(loginTimer);
        } else if (this.login.status === "failed") {
          window.clearInterval(loginTimer);
        }
      } catch (error) {
        window.clearInterval(loginTimer);
        this.login = { status: "failed", message: String(error) };
      }
    },
    async cancelLogin() {
      window.clearInterval(loginTimer);
      await invoke("cancel_add_account");
      this.login = { status: "idle" };
    },
    async confirmDuplicate(overwrite: boolean) {
      try {
        this.login = await invoke<LoginProgress>("confirm_add_account", { overwrite });
        if (this.login.status === "completed") await this.load();
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
  },
});
