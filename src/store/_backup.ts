import { BackupInfo } from "@/types";
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
  }
})));

export { useBackupStore }