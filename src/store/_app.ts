import { AppInfo } from "@/types";
import { create } from "zustand";
import { combine } from "zustand/middleware";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";

const useAppStore = create(combine({ apps: [] as AppInfo[] }, (set) => ({
  fetchApps: async () => {
    try {
      const appList = await invoke<AppInfo[]>("list_applications");
      set({ apps: appList });
    } catch (e) {
      toast.error("Failed to fetch applications.")
      console.error(e);
    }
  }
})));

export { useAppStore }