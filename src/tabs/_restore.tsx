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
import { BackupChainInfo } from "@/types";

const RestoreTab = () => {
  const { apps } = useAppStore();
  const { backups, fetchBackup, verifyBackupIntegrity, verifyBackupChain, getBackupChainInfo } = useBackupStore();
  const [selectedBackup, setSelectedBackup] = React.useState<string | null>(
    null
  );
  const [chainInfo, setChainInfo] = React.useState<BackupChainInfo | null>(null);
  const [isVerifying, setIsVerifying] = React.useState(false);

  const fetchBackupCallback = React.useCallback(async () => {
    await fetchBackup();
  }, [fetchBackup]);

  React.useEffect(() => {
    fetchBackupCallback();
  }, [apps, fetchBackupCallback]);

  React.useEffect(() => {
    if (selectedBackup) {
      getBackupChainInfo(selectedBackup).then(setChainInfo);
    } else {
      setChainInfo(null);
    }
  }, [selectedBackup, getBackupChainInfo]);

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

  const handleVerifyIntegrity = async () => {
    if (!selectedBackup) return;
    setIsVerifying(true);
    await verifyBackupIntegrity(selectedBackup);
    // Refresh chain info after verification
    const newInfo = await getBackupChainInfo(selectedBackup);
    setChainInfo(newInfo);
    setIsVerifying(false);
  };

  const handleVerifyChain = async () => {
    if (!selectedBackup) return;
    setIsVerifying(true);
    await verifyBackupChain(selectedBackup);
    setIsVerifying(false);
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle>Restore from Backup</CardTitle>
        <CardDescription>
          Select a backup and restore its configuration. Backups are protected with 
          blockchain-style integrity verification to ensure data hasn't been tampered with.
          Maximum compression is used to optimize storage.
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
        
        {/* Blockchain Integrity Information */}
        {chainInfo && (
          <div className="space-y-2 p-3 border rounded-md bg-background">
            <Label className="text-sm font-semibold">Blockchain Integrity Status</Label>
            <div className="text-xs space-y-1">
              <div className="flex justify-between">
                <span>Integrity Valid:</span>
                <span className={chainInfo.is_integrity_valid ? "text-green-600" : "text-red-600"}>
                  {chainInfo.is_integrity_valid ? "✓ Valid" : "✗ Invalid"}
                </span>
              </div>
              <div className="flex justify-between">
                <span>Chain Hash:</span>
                <span className="font-mono text-xs">{chainInfo.chain_hash.slice(0, 16)}...</span>
              </div>
              {chainInfo.previous_backup_hash && (
                <div className="flex justify-between">
                  <span>Previous Backup:</span>
                  <span className="font-mono text-xs">{chainInfo.previous_backup_hash.slice(0, 16)}...</span>
                </div>
              )}
            </div>
          </div>
        )}
      </CardContent>
      <CardFooter className="flex flex-col space-y-2">
        <div className="flex w-full space-x-2">
          <Button onClick={handleRestoreBackup} disabled={!selectedBackup}>
            Restore Selected Apps
          </Button>
          <Button
            variant="outline"
            onClick={fetchBackupCallback}
          >
            Refresh
          </Button>
        </div>
        
        {/* Blockchain Verification Buttons */}
        {selectedBackup && (
          <div className="flex w-full space-x-2">
            <Button
              variant="secondary"
              size="sm"
              onClick={handleVerifyIntegrity}
              disabled={isVerifying}
            >
              {isVerifying ? "Verifying..." : "Verify Integrity"}
            </Button>
            <Button
              variant="secondary"
              size="sm"
              onClick={handleVerifyChain}
              disabled={isVerifying}
            >
              {isVerifying ? "Verifying..." : "Verify Chain"}
            </Button>
          </div>
        )}
      </CardFooter>
    </Card>
  );
};

export { RestoreTab };
