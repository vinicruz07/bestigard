# Guia de Compilação e Publicação Mobile (Android APK / NDK) — Bestigard

Este documento descreve o pipeline completo para compilar, empacotar, assinar e instalar o **Bestigard — Santuário das Bestas** como um pacote Android nativo (`.apk`) utilizando `cargo-apk`, Android SDK/NDK e a arquitetura 100% Safe Rust do workspace.

---

## 1. Arquitetura Mobile do Projeto

O projeto está dividido em 3 crates dentro do workspace Cargo:

- **`crates/game_core`**: Regras de negócio puras, genética, habitats, fazendas, incubação, combate elemental, bestiário e serialização JSON com progresso offline (`#![forbid(unsafe_code)]`).
- **`crates/engine`**: Abstração multiplataforma gráfica (`miniquad` OpenGL ES 2/3), `SpriteBatch` em draw call único, fonte bitmap embutida, `AssetLoader` com atlas procedural de fallback (`512x512` RGBA8), `AudioOutputDriver` com sintetizador ADSR procedural e `PersistenceManager` com caminhos nativos Android (`/data/data/com.bestigard.game/files/savegame.json`).
- **`crates/mobile_entry`**: Ponto de entrada unificado (Desktop e Android `NativeActivity`), configurado com metadados `[package.metadata.android]` em `crates/mobile_entry/Cargo.toml` e manifesto em `platform/android/AndroidManifest.xml`.

---

## 2. Pré-requisitos do Ambiente

### 2.1 Targets Rust para Android
Adicione os targets de compilação cruzada para arquiteturas ARM64 (dispositivos modernos), ARMv7 (dispositivos 32-bit legados) e x86_64 (emuladores Android Studio):

```bash
rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
```

### 2.2 Android SDK & NDK
1. Instale o **Android SDK Platform 34** e **Android SDK Build-Tools 34.0.0+**.
2. Instale o **Android NDK (r25c ou superior, recomendado r26d)** via SDK Manager ou linha de comando (`sdkmanager "ndk;26.3.11579264" "platforms;android-34" "build-tools;34.0.0"`).
3. Configure as variáveis de ambiente no sistema:

**PowerShell (Windows):**
```powershell
$env:ANDROID_HOME = "C:\Users\<usuario>\AppData\Local\Android\Sdk"
$env:ANDROID_NDK_ROOT = "$env:ANDROID_HOME\ndk\26.3.11579264"
$env:JAVA_HOME = "C:\Program Files\Android\Android Studio\jbr"
```

**Bash (Linux / macOS):**
```bash
export ANDROID_HOME="$HOME/Android/Sdk"
export ANDROID_NDK_ROOT="$ANDROID_HOME/ndk/26.3.11579264"
```

### 2.3 Utilitário `cargo-apk`
Instale o empacotador `cargo-apk`:

```bash
cargo install cargo-apk
```

---

## 3. Manifesto e Configurações de Empacotamento

- **Manifesto Oficial**: `platform/android/AndroidManifest.xml`
  - **Pacote (`package`)**: `com.bestigard.game`
  - **Orientação**: Fixada em modo retrato (`android:screenOrientation="portrait"`, resolução lógica `1080x1920`).
  - **Tela Cheia**: `@android:style/Theme.NoTitleBar.Fullscreen`.
  - **Aceleração Gráfica**: OpenGL ES 3.0 (`0x00030000`) e suporte obrigatório a `multitouch`.
  - **Biblioteca Nativa**: `android.app.lib_name = "mobile_entry"`.
- **Metadados Cargo (`crates/mobile_entry/Cargo.toml`)**:
  - Define `[package.metadata.android]` com `min_sdk_version = 24`, `target_sdk_version = 34`, orientação retrato e empacotamento automático da pasta `assets/`.
  - Mesmo se a pasta `assets/` não contiver PNGs externos, o `AssetLoader::build_sanctuary_atlas_bundle` constrói em memória o atlas `512x512` completo com sprites procedurais + fonte ASCII embutida, garantindo inicialização 100% livre de falhas no APK.

---

## 4. Compilação e Geração do APK

### 4.1 Build de Desenvolvimento (Debug APK)
Para gerar um APK assinado com chave de debug para testes rápidos:

```bash
cargo apk build -p mobile_entry
```

O APK será gerado em:
`target/debug/apk/mobile_entry.apk`

### 4.2 Build Otimizado de Produção (Release APK)
O perfil `[profile.release]` no `Cargo.toml` raiz já está configurado com `opt-level = 3`, `lto = true`, `codegen-units = 1`, `panic = "abort"` e `strip = true` para binário mínimo e máxima performance em dispositivos móveis:

```bash
cargo apk build -p mobile_entry --release
```

O APK otimizado será gerado em:
`target/release/apk/mobile_entry.apk`

---

## 5. Assinatura para Distribuição (Release Keystore)

1. Gere uma chave de assinatura (caso ainda não possua):
```bash
keytool -genkey -v -keystore bestigard-release.jks -keyalg RSA -keysize 2048 -validity 10000 -alias bestigard
```

2. Alinhe e assine o APK com `zipalign` e `apksigner` (presentes em `$ANDROID_HOME/build-tools/34.0.0/`):
```bash
zipalign -v -p 4 target/release/apk/mobile_entry.apk target/release/apk/bestigard-aligned.apk
apksigner sign --ks bestigard-release.jks --out target/release/apk/bestigard-release-signed.apk target/release/apk/bestigard-aligned.apk
apksigner verify target/release/apk/bestigard-release-signed.apk
```

---

## 6. Instalação e Depuração em Dispositivo Físico (ADB)

1. Ative a **Depuração USB** nas Opções do Desenvolvedor do aparelho Android e conecte-o via USB.
2. Instale e execute diretamente:
```bash
cargo apk run -p mobile_entry --release
```
Ou instale manualmente via `adb`:
```bash
adb install -r target/release/apk/mobile_entry.apk
```
3. Para acompanhar os logs do ciclo de vida (`on_pause`, auto-save atômico, progresso offline):
```bash
adb logcat -s RustStdoutStderr:D Bestigard:D
```

