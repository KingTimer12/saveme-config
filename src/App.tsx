import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

import { Button } from "./components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from "./components/ui/card";
import { Checkbox } from "./components/ui/checkbox";
import { Input } from "./components/ui/input";
import { Label } from "./components/ui/label";
import { toast } from "sonner";

// Define the types for the data from the backend
interface AppInfo {
  id: string;
  name: string;
  is_installed: boolean;
}

interface BackupInfo {
  name: string;
  created_at: string;
}

function App() {
  // State for applications
  const [apps, setApps] = useState<AppInfo[]>([]);
  const [selectedApps, setSelectedApps] = useState<Set<string>>(new Set());
  const [backupName, setBackupName] = useState("");

  // State for backups
  const [backups, setBackups] = useState<BackupInfo[]>([]);
  const [selectedBackup, setSelectedBackup] = useState<string | null>(null);

  // Loading and error states
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);

  // Fetch initial data on component mount
  useEffect(() => {
    fetchApps();
    fetchBackups();
  }, []);

  const fetchApps = async () => {
    try {
      const appList = await invoke<AppInfo[]>("list_applications");
      setApps(appList);
    } catch (e) {
      setError("Failed to fetch applications.");
      console.error(e);
    }
  };

  const fetchBackups = async () => {
    try {
      const backupList = await invoke<BackupInfo[]>("list_backups");
      setBackups(backupList);
    } catch (e) {
      setError("Failed to fetch backups.");
      console.error(e);
    }
  };

  const handleAppSelection = (appId: string) => {
    setSelectedApps((prev) => {
      const newSet = new Set(prev);
      if (newSet.has(appId)) {
        newSet.delete(appId);
      } else {
        newSet.add(appId);
      }
      return newSet;
    });
  };

  const handleSaveBackup = async () => {
    if (!backupName) {
      setError("Please enter a name for the backup.");
      return;
    }
    if (selectedApps.size === 0) {
      setError("Please select at least one application to back up.");
      return;
    }
    const toastId = toast.loading("Saving backup...");
    setLoading(true);
    setError(null);
    setMessage(null);

    try {
      const result = await invoke<string>("save_config", {
        name: backupName,
        appIds: Array.from(selectedApps),
      });
      setMessage(result);
      setBackupName("");
      setSelectedApps(new Set());
      fetchBackups(); // Refresh the backup list
      toast.success("Backup saved successfully!", { id: toastId });
    } catch (e) {
      toast.error("Failed to save backup!", { id: toastId });
      setError(e as string);
    } finally {
      setLoading(false);
    }
  };

  const handleRestoreBackup = async () => {
    if (!selectedBackup) {
      setError("Please select a backup to restore.");
      return;
    }
    if (selectedApps.size === 0) {
      setError("Please select at least one application to restore.");
      return;
    }
    const toastId = toast.loading("Restoring backup...");
    setLoading(true);
    setError(null);
    setMessage(null);

    try {
      const result = await invoke<string>("restore_config", {
        backupName: selectedBackup,
        appIds: Array.from(selectedApps),
      });
      setMessage(result);
      toast.success("Backup restored successfully!", { id: toastId });
    } catch (e) {
      toast.error("Failed to restore backup.", { id: toastId });
      setError(e as string);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="container mx-auto p-4 font-sans bg-background text-foreground">
      <header className="text-center mb-8">
        <h1 className="text-4xl font-bold">SaveMe Config</h1>
        <p className="text-muted-foreground">
          Your personal configuration manager.
        </p>
      </header>

      {error && (
        <div
          className="p-4 mb-4 text-sm text-destructive-foreground bg-destructive rounded-md"
          role="alert"
        >
          {error}
        </div>
      )}
      {message && (
        <div
          className="p-4 mb-4 text-sm text-primary-foreground bg-primary rounded-md"
          role="alert"
        >
          {message}
        </div>
      )}

      <main className="grid grid-cols-1 md:grid-cols-2 gap-8">
        {/* Backup Section */}
        <Card>
          <CardHeader>
            <CardTitle>Create a Backup</CardTitle>
            <CardDescription>
              Select the applications you want to back up.
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="space-y-2">
              {apps.map((app) => (
                <div key={app.id} className="flex items-center space-x-2">
                  <Checkbox
                    id={`app-${app.id}`}
                    disabled={!app.is_installed}
                    checked={selectedApps.has(app.id)}
                    onChange={() => handleAppSelection(app.id)}
                  />
                  <Label
                    htmlFor={`app-${app.id}`}
                    className={!app.is_installed ? "text-muted-foreground" : ""}
                  >
                    {app.name} {!app.is_installed && "(Not Installed)"}
                  </Label>
                </div>
              ))}
            </div>
            <div className="space-y-2">
              <Label htmlFor="backup-name">Backup Name</Label>
              <Input
                id="backup-name"
                placeholder="e.g., My-Laptop-Setup"
                value={backupName}
                onChange={(e) => setBackupName(e.target.value)}
              />
            </div>
          </CardContent>
          <CardFooter>
            <Button onClick={handleSaveBackup} disabled={loading}>
              {loading ? "Saving..." : "Save Backup"}
            </Button>
          </CardFooter>
        </Card>

        {/* Restore Section */}
        <Card>
          <CardHeader>
            <CardTitle>Restore from Backup</CardTitle>
            <CardDescription>
              Select a backup and restore its configuration. This may install
              missing applications.
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="space-y-2">
              <Label>Available Backups</Label>
              {backups.length === 0 && (
                <p className="text-sm text-muted-foreground">
                  No backups found.
                </p>
              )}
              <div className="space-y-2">
                {backups.map((backup) => (
                  <div
                    key={backup.name}
                    className={`p-2 rounded-md cursor-pointer ${
                      selectedBackup === backup.name
                        ? "bg-primary text-primary-foreground"
                        : "bg-muted"
                    }`}
                    onClick={() => setSelectedBackup(backup.name)}
                  >
                    <p className="font-semibold">{backup.name}</p>
                    <p className="text-xs">
                      {new Date(backup.created_at).toLocaleString()}
                    </p>
                  </div>
                ))}
              </div>
            </div>
          </CardContent>
          <CardFooter>
            <Button
              onClick={handleRestoreBackup}
              disabled={loading || !selectedBackup}
            >
              {loading ? "Restoring..." : "Restore Selected Apps"}
            </Button>
            <Button variant="outline" onClick={fetchBackups} className="ml-2">
              Refresh
            </Button>
          </CardFooter>
        </Card>
      </main>
    </div>
  );
}

export default App;
