import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Label } from "@/components/ui/label";
import { useAppStore } from "@/store";
import { useBackupStore } from "@/store/_backup";
import { invoke } from "@tauri-apps/api/core";
import React from "react";
import { toast } from "sonner";

const RestoreTab = () => {
  const { apps } = useAppStore();
  const { backups, fetchBackup } = useBackupStore();
  const [selectedBackup, setSelectedBackup] = React.useState<string | null>(
    null
  );

  const fetchBackupCallback = React.useCallback(async () => {
    await fetchBackup();
  }, [fetchBackup]);

  React.useEffect(() => {
    fetchBackupCallback();
  }, [apps, fetchBackupCallback]);

  const handleRestoreBackup = async () => {
    const toastId = toast.loading("Restoring backup...");

    try {
      const result = await invoke<string>("restore_config", {
        backupName: selectedBackup,
        appIds: apps.map((app) => app.id),
      });
      toast.success(result, {
        id: toastId,
        description: "Backup restored successfully!",
      });
    } catch (e) {
      toast.error("Failed to restore backup.", { id: toastId });
      console.error(e);
    }
  };

  return (
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
            <p className="text-sm text-muted-foreground">No backups found.</p>
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
        <Button onClick={handleRestoreBackup} disabled={!selectedBackup}>
          Restore Selected Apps
        </Button>
        <Button
          variant="outline"
          onClick={fetchBackupCallback}
          className="ml-2"
        >
          Refresh
        </Button>
      </CardFooter>
    </Card>
  );
};

export { RestoreTab };
