import { BackupInfo, BackupChainInfo } from "@/types";
import { create } from "zustand";
import { combine } from "zustand/middleware";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";

const useBackupStore = create(combine({ backups: [] as BackupInfo[] }, (set) => ({
  fetchBackup: async () => {
    try {
      const appList = await invoke<BackupInfo[]>("list_backups");
      set({ backups: appList });
    } catch (e) {
      toast.error("Failed to fetch backups.")
      console.error(e);
    }
  },
  
  verifyBackupIntegrity: async (backupName: string) => {
    try {
      const result = await invoke<string>("verify_backup_integrity", { backupName });
      toast.success(result);
      return true;
    } catch (e) {
      toast.error(`Integrity verification failed: ${e}`);
      console.error(e);
      return false;
    }
  },

  verifyBackupChain: async (startBackupName: string) => {
    try {
      const result = await invoke<string>("verify_backup_chain", { startBackupName });
      toast.success(result);
      return true;
    } catch (e) {
      toast.error(`Chain verification failed: ${e}`);
      console.error(e);
      return false;
    }
  },

  getBackupChainInfo: async (backupName: string) => {
    try {
      const result = await invoke<BackupChainInfo>("get_backup_chain_info", { backupName });
      return result;
    } catch (e) {
      toast.error(`Failed to get chain info: ${e}`);
      console.error(e);
      return null;
    }
  }
})));

export { useBackupStore }