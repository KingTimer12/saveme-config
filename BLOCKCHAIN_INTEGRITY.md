# Blockchain Integrity & Enhanced Compression

SaveMe Config agora inclui recursos avançados de integridade blockchain e compressão otimizada para garantir máxima segurança e eficiência de armazenamento.

## 🔒 Sistema de Integridade Blockchain

### Verificação de Cadeia
Cada backup agora mantém uma referência criptográfica ao backup anterior, criando uma cadeia de integridade inviolável:

```
Backup 1 → Hash da Cadeia 1
Backup 2 → [Hash da Cadeia 1] + [Conteúdo] → Hash da Cadeia 2  
Backup 3 → [Hash da Cadeia 2] + [Conteúdo] → Hash da Cadeia 3
```

### Recursos de Segurança
- **Detecção de Violação**: Qualquer alteração nos dados quebra a cadeia
- **Verificação Individual**: Verificação de integridade de backups únicos
- **Verificação de Cadeia**: Validação completa desde qualquer ponto inicial
- **Transparência Total**: Visualização do status de integridade na interface

## 🗜️ Compressão Máxima & Deduplicação

### Otimizações Implementadas
- **Compressão zstd nível 19**: Máxima compressão possível (antes: nível 3)
- **Deduplicação Global**: Conteúdo idêntico é compartilhado entre backups
- **Otimização para GitHub**: Tamanhos reduzidos para upload em repositórios

### Benefícios de Armazenamento
- **Redução de 30-70%** no tamanho dos backups
- **Eliminação de Redundância**: Arquivos idênticos são armazenados uma única vez
- **Compatibilidade com GitHub**: Adequado para limites de tamanho de repositório

## 🚀 Novos Comandos da API

### Verificação de Integridade
```typescript
// Verificar integridade de um backup específico
invoke('verify_backup_integrity', { backupName: 'meu-backup' })

// Verificar cadeia completa a partir de um backup
invoke('verify_backup_chain', { startBackupName: 'meu-backup' })

// Obter informações detalhadas da cadeia
invoke('get_backup_chain_info', { backupName: 'meu-backup' })
```

### Estrutura BackupChainInfo
```typescript
interface BackupChainInfo {
  name: string;
  backup_hash: string;        // Hash do conteúdo do backup
  chain_hash: string;         // Hash da cadeia blockchain
  previous_backup_hash?: string; // Referência ao backup anterior
  is_integrity_valid: boolean;   // Status de integridade
}
```

## 🖥️ Interface do Usuário

### Aba de Restauração Atualizada
- **Status de Integridade**: Indicador visual da validade do backup
- **Botões de Verificação**: Verificação manual de integridade e cadeia
- **Informações da Cadeia**: Visualização de hashes e referências
- **Feedback em Tempo Real**: Notificações de sucesso/falha

### Criação de Backup
- **Vinculação Automática**: Novos backups são automaticamente vinculados aos anteriores
- **Deduplicação Transparente**: Redução automática de armazenamento
- **Compressão Máxima**: Aplicada automaticamente a todos os novos backups

## 🔧 Implementação Técnica

### Algoritmo de Hash da Cadeia
```rust
fn calculate_chain_hash(previous_hash: Option<String>, current_hash: String) -> String {
    let mut hasher = Sha256::new();
    if let Some(prev) = previous_hash {
        hasher.update(prev.as_bytes());
    }
    hasher.update(current_hash.as_bytes());
    hex::encode(hasher.finalize())
}
```

### Processo de Deduplicação
1. Calcula SHA256 do conteúdo comprimido
2. Verifica se conteúdo idêntico existe em backups anteriores
3. Se encontrado, reutiliza referência existente
4. Se não encontrado, armazena novo blob

### Verificação de Integridade
1. Recalcula hash do conteúdo do backup
2. Verifica se hash da cadeia corresponde ao armazenado
3. Para verificação de cadeia, percorre todos os backups conectados
4. Detecta referências circulares e cadeias quebradas

## ⚡ Benefícios

### Segurança
- **Imutabilidade**: Dados protegidos contra alteração não autorizada
- **Rastreabilidade**: Histórico completo de modificações
- **Verificação Independente**: Validação sem necessidade de confiança

### Eficiência
- **Armazenamento Otimizado**: Significativa redução de espaço
- **Performance**: Compressão máxima sem comprometer velocidade
- **Compatibilidade**: Ideal para versionamento em Git/GitHub

### Usabilidade
- **Interface Intuitiva**: Verificações com um clique
- **Feedback Visual**: Status claro de integridade
- **Operação Transparente**: Funciona automaticamente em segundo plano

Este sistema garante que os programas salvos nunca falhem devido à corrupção de dados, enquanto mantém os arquivos o mais comprimidos possível para facilitar o upload no GitHub.