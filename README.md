<h1 align="center">🦀 Rscovery</h1>

<p align="center">
  <strong>Recuperação de arquivos por <em>magic bytes</em>, feita em Rust + Tauri.</strong><br/>
  Escaneia discos bloco a bloco e recupera JPEG, PNG, PDF, ZIP e textos — mesmo sem sistema de arquivos.
</p>

<p align="center">
  <a href="https://github.com/Pedro21062014/rscovery/releases/latest">
    <img src="https://img.shields.io/github/v/release/Pedro21062014/rscovery?label=release&logo=github&color=blue" alt="Última release"/>
  </a>
  <a href="https://github.com/Pedro21062014/rscovery/releases">
    <img src="https://img.shields.io/github/downloads/Pedro21062014/rscovery/total?label=downloads&logo=github" alt="Downloads"/>
  </a>
  <a href="https://github.com/Pedro21062014/rscovery/actions/workflows/release.yml">
    <img src="https://img.shields.io/github/actions/workflow/status/Pedro21062014/rscovery/release.yml?label=build&logo=githubactions&logoColor=white" alt="Build"/>
  </a>
  <img src="https://img.shields.io/github/languages/top/Pedro21062014/rscovery?logo=rust&label=Rust" alt="Linguagem principal"/>
  <img src="https://img.shields.io/badge/Tauri-2-24C5DB?logo=tauri&logoColor=white" alt="Tauri 2"/>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Linux-FCC624?logo=linux&logoColor=black" alt="Linux"/>
  <img src="https://img.shields.io/badge/macOS-000000?logo=apple&logoColor=white" alt="macOS"/>
  <img src="https://img.shields.io/badge/Windows-0078D6?logo=windows&logoColor=white" alt="Windows"/>
</p>

---

> ℹ️ **Este projeto é um fork de [YuriRDev/rscovery](https://github.com/YuriRDev/rscovery).**
> Todos os créditos do projeto original vão para os autores originais.
> Este fork adiciona: CI/CD com builds automáticos, releases multiplataforma
> (Linux `.deb`/`.AppImage`/`.rpm`, macOS `.dmg`, Windows `.exe`/`.msi`),
> tratamento de permissões e correções de compilação em Windows/macOS.

---

## ✨ Funcionalidades

| Recurso | Descrição |
|---|---|
| 🔍 **Scan de blocos** | Lê o disco inteiro em blocos de 32 MB e mostra um mapa visual das áreas com dados |
| 🖼️ **Recuperar imagens** | JPEG e PNG (pré-visualização direto no app) |
| 📄 **Recuperar documentos** | PDF e ZIP salvos em disco |
| ✏️ **Recuperar textos** | Busca trechos de texto usando wordlist/blacklist personalizáveis |
| 💽 **Multiplataforma** | Linux, macOS e Windows |

## 📥 Downloads

Baixe a versão mais recente na [página de Releases](https://github.com/Pedro21062014/rscovery/releases/latest):

| Sistema | Arquivo |
|---|---|
| 🐧 Linux (Debian/Ubuntu) | `rscovery_*_amd64.deb` |
| 🐧 Linux (qualquer distro) | `rscovery_*_amd64.AppImage` |
| 🐧 Linux (Fedora/RHEL) | `rscovery-*.x86_64.rpm` |
| 🍎 macOS (Intel e Apple Silicon) | `rscovery_*_universal.dmg` |
| 🪟 Windows (instalador) | `rscovery_*_x64-setup.exe` |
| 🪟 Windows (MSI) | `rscovery_*_x64_en-US.msi` |

## ⚠️ Permissões (importante!)

O Rscovery lê o disco **diretamente (raw disk)** e por isso precisa ser
executado com permissões elevadas, senão o scan não inicia:

```bash
# 🐧 Linux — AppImage
chmod +x rscovery_*_amd64.AppImage
sudo ./rscovery_*_amd64.AppImage

# 🐧 Linux — instalado via .deb
sudo rscovery
```

```text
🪟 Windows → clique com o botão direito no app → "Executar como administrador"
🍎 macOS   → sudo /Applications/rscovery.app/Contents/MacOS/rscovery
```

> O app avisa na tela inicial quando está rodando sem permissões elevadas.

## 🛠️ Rodando o projeto localmente

### Pré-requisitos

- **Git**, **Node.js/NPM** e **Yarn**
- **Rust** (instale com `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`)
- **Linux:** bibliotecas do Tauri 2:

```bash
sudo apt update
sudo apt install -y build-essential git curl wget file pkg-config \
  libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev
```

> 💡 No Ubuntu 22.04+ o pacote é `libwebkit2gtk-4.1-dev` (o Tauri 2 usa a versão 4.1).

### Passos

```bash
git clone https://github.com/Pedro21062014/rscovery
cd rscovery
yarn install        # dependências do frontend
yarn tauri dev      # modo desenvolvimento
```

Para gerar os instaladores localmente:

```bash
yarn tauri build
```

Os artefatos ficam em `src-tauri/target/release/bundle/`.

> 🔧 Se ocorrerem erros de permissão com `cargo`/`npm`:
> `sudo chown -R $USER:$USER ~/.cargo ~/.npm`

## ⚙️ CI/CD — Builds e Releases automáticos

Toda tag `v*` dispara o workflow
[`release.yml`](.github/workflows/release.yml), que compila o app em
paralelo nas 3 plataformas e publica a release automaticamente:

| Runner | O que gera |
|---|---|
| `ubuntu-22.04` | `.deb`, `.rpm` e `.AppImage` |
| `macos-latest` | `.dmg` universal (Intel + Apple Silicon) |
| `windows-latest` | `.exe` (NSIS) e `.msi` |

Para publicar uma nova versão:

```bash
# 1. atualize a versão em src-tauri/tauri.conf.json e src-tauri/Cargo.toml
# 2. crie e envie a tag:
git tag v0.2.0
git push origin v0.2.0
```

## 📚 Como funciona

1. **Listagem de discos** — o backend (Rust, crate `sysinfo`) lista os discos
   e resolve o dispositivo físico (`/dev/sda1`, `\\.\C:`, `/dev/rdisk*`).
2. **Scan de blocos** — o disco é lido em blocos de 32 MB; blocos não vazios
   são marcados no mapa visual.
3. **Extração por magic bytes** — cada tipo de arquivo é identificado pela
   assinatura binária (ex.: JPEG começa com `FF D8` e termina com `FF D9`)
   e reconstruído a partir do stream bruto.

---

<p align="center">
  Feito com 🦀 Rust + ⚛️ React · Fork de
  <a href="https://github.com/YuriRDev/rscovery">YuriRDev/rscovery</a>
</p>
