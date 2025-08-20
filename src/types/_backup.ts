interface BackupInfo {
  name: string;
  created_at: string;
}

interface BackupChainInfo {
  name: string;
  backup_hash: string;
  chain_hash: string;
  previous_backup_hash?: string;
  is_integrity_valid: boolean;
}

export type { BackupInfo, BackupChainInfo };
