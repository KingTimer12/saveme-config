import { Button } from "./components/ui/button";
import "./styles.css";
import { X } from "lucide-react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { invoke } from "@tauri-apps/api/core";
import { attachConsole } from '@tauri-apps/plugin-log';

function App() {
  const appWindow = getCurrentWindow();
  const onClick = async () => {
    const detach = await attachConsole();
    console.log("abc");
    const path = await invoke<string>("save_config", { name: "snapshot-test" });
    console.log(`Config saved at: ${path}`);
    detach();
  };
  return (
    <div className="grid grid-cols-12 gap-2 w-full  h-full bg-black">
      <header
        data-tauri-drag-region
        className="w-full bg-primary h-10 items-center justify-between px-10 flex col-span-12 z-10"
      >
        <p className="text-primary-foreground pointer-events-auto select-none">
          Save me config!
        </p>
        <Button
          size="icon"
          className="cursor-pointer hover:text-destructive-foreground rounded-full z-20"
          onClick={() => appWindow.close()}
        >
          <X className="size-4" />
        </Button>
      </header>
      <main className="">
        <Button className="cursor-pointer" onClick={onClick}>
          Click me
        </Button>
      </main>
    </div>
  );
}

export default App;
