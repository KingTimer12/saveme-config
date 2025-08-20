# SaveMe Config

Uma aplicação desktop moderna para backup e restauração de configurações de aplicações, construída com Tauri, React e TypeScript. O projeto implementa um sistema de blockchain para garantir a integridade dos dados e oferece deduplicação inteligente para otimizar o armazenamento.

## 🚀 Funcionalidades

### Core Features

- **Backup Automático**: Detecta e faz backup das configurações de aplicações instaladas
- **Restauração Inteligente**: Restaura configurações e instala aplicações automaticamente quando necessário
- **Verificação de Integridade**: Sistema de blockchain para garantir que os backups não foram corrompidos
- **Deduplicação**: Evita armazenar dados duplicados, economizando espaço em disco
- **Compressão Avançada**: Usa zstd com máxima compressão para otimizar o armazenamento
- **Cross-Platform**: Suporte para Windows, macOS e Linux

### Tecnologias Avançadas

- **Blob Blockchain**: Cada arquivo é armazenado como um blob em uma cadeia criptográfica
- **SHA256 Hashing**: Verificação de integridade e deduplicação baseada em hash
- **TAR Archives**: Empacotamento eficiente de arquivos de configuração
- **Zstd Compression**: Compressão de alta performance com nível máximo

## 🛠️ Tecnologias Utilizadas

### Frontend

- **React 18**: Biblioteca para interfaces de usuário
- **TypeScript**: Superset tipado do JavaScript
- **Vite**: Ferramenta de build e desenvolvimento rápida

### Backend

- **Tauri**: Framework para aplicações desktop com Rust
- **Rust**: Linguagem de programação systems-level
- **Serde**: Serialização/deserialização de dados
- **Tokio**: Runtime assíncrono para Rust

### Bibliotecas Principais

- `base64`: Codificação/decodificação de dados
- `sha2`: Hashing criptográfico
- `zstd`: Compressão de dados
- `tar`: Manipulação de arquivos TAR
- `walkdir`: Navegação de diretórios
- `chrono`: Manipulação de datas e horários
- `anyhow`: Tratamento de erros

## 📋 Pré-requisitos

### Sistema

- **Node.js**: versão 18 ou superior
- **Rust**: versão 1.70 ou superior (instalado via [rustup](https://rustup.rs/))
- **Git**: para controle de versão

### Plataforma Específica

#### Windows

- Microsoft Visual Studio C++ Build Tools
- WebView2 Runtime

#### macOS

- Xcode Command Line Tools
- macOS 10.15 ou superior

#### Linux

- `libwebkit2gtk-4.0-dev`
- `build-essential`
- `curl`
- `wget`
- `libssl-dev`
- `libgtk-3-dev`
- `libayatana-appindicator3-dev`
- `librsvg2-dev`

## 🚀 Instalação e Execução

### 1. Clone o Repositório

```bash
git clone https://github.com/seu-usuario/saveme-config.git
cd saveme-config
```

### 2. Instale as Dependências

```bash
# Instalar dependências da aplicação
npm install
```

### 3. Configuração do Ambiente

```bash
# Verificar se o ambiente Tauri está configurado
npm run tauri info
```

### 4. Executar em Modo de Desenvolvimento

```bash
# Inicia o frontend e backend em modo dev
npm run tauri dev
```

### 5. Build para Produção

```bash
# Gera executável otimizado
npm run tauri build
```

## 📖 Como Usar

### 1. Backup de Configurações

1. Abra a aplicação
2. Selecione as aplicações cujas configurações deseja fazer backup
3. Digite um nome para o backup
4. Clique em "Salvar Configurações"

### 2. Restauração

1. Vá para a aba "Backups"
2. Selecione o backup desejado
3. Escolha quais aplicações restaurar
4. Clique em "Restaurar"

### 3. Verificação de Integridade

1. Selecione um backup
2. Clique em "Verificar Integridade"
3. O sistema validará a cadeia de blobs e reportará o status

## 🏗️ Arquitetura do Projeto

```
saveme-config/
├── src/                    # Frontend React/TypeScript
├── src-tauri/             # Backend Rust
│   ├── src/
│   │   ├── apps.rs        # Detecção de aplicações
│   │   ├── installer.rs   # Instalação automática
│   │   └── storage/       # Sistema de armazenamento
│   │       ├── blobs.rs   # Gerenciamento de blobs
│   │       ├── manifest.rs # Manifesto de backups
│   │       └── entry.rs   # Entradas de arquivos
├── public/                # Recursos estáticos
└── dist/                  # Build de produção
```

### Sistema de Blockchain de Blobs

O projeto implementa um sistema único de blockchain para armazenamento de dados:

1. **Blob Creation**: Cada arquivo é convertido em um blob com hash SHA256
2. **Chain Linking**: Cada blob referencia o blob anterior (blockchain)
3. **Integrity Verification**: Verificação criptográfica da cadeia completa
4. **Deduplication**: Blobs idênticos são reutilizados entre backups

## 🤝 Contribuição

### Como Contribuir

1. **Fork** o repositório
2. **Clone** seu fork localmente
3. **Crie** uma branch para sua feature (`git checkout -b feature/nova-funcionalidade`)
4. **Commit** suas mudanças (`git commit -m 'Adiciona nova funcionalidade'`)
5. **Push** para a branch (`git push origin feature/nova-funcionalidade`)
6. **Abra** um Pull Request

### Diretrizes de Contribuição

#### Código

- Siga as convenções do Rust (use `cargo fmt` e `cargo clippy`)
- Mantenha cobertura de testes adequada
- Documente funções públicas
- Use commits semânticos

#### Testes

```bash
# Testes do backend Rust
cargo test

# Testes do frontend (quando implementados)
npm test
```

#### Linting

```bash
# Rust
cargo clippy -- -D warnings
cargo fmt

# TypeScript/React
npm run lint
npm run type-check
```

### Tipos de Contribuição

- 🐛 **Bug Reports**: Reporte bugs com detalhes e passos para reproduzir
- 💡 **Feature Requests**: Sugira novas funcionalidades
- 📝 **Documentação**: Melhore a documentação do projeto
- 🔧 **Código**: Implemente features ou correções
- 🧪 **Testes**: Adicione ou melhore testes existentes

## 🐛 Resolução de Problemas

### Problemas Comuns

#### Erro de Compilação do Tauri

```bash
# Limpar cache e reinstalar
cargo clean
npm run tauri build
```

#### WebView2 não encontrado (Windows)

- Baixe e instale o [WebView2 Runtime](https://developer.microsoft.com/en-us/microsoft-edge/webview2/)

#### Permissões no Linux

```bash
# Dar permissões de execução
chmod +x target/release/bundle/appimage/saveme-config_*.AppImage
```

### Logs e Debug

```bash
# Executar com logs detalhados
RUST_LOG=debug npm run tauri dev
```

## 📄 Licença

Este projeto está licenciado sob a [MIT License](LICENSE).

## 🔗 Links Úteis

- [Documentação do Tauri](https://tauri.app/)
- [Documentação do React](https://reactjs.org/)
- [Documentação do Rust](https://doc.rust-lang.org/)
- [Vite Documentation](https://vitejs.dev/)

## 📞 Suporte

- **Issues**: [GitHub Issues](https://github.com/seu-usuario/saveme-config/issues)
- **Discussions**: [GitHub Discussions](https://github.com/seu-usuario/saveme-config/discussions)

---

Desenvolvido com ❤️ usando Tauri, React e Rust.
