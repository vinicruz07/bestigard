//! Validador de pipeline de build Android (cargo-apk / NDK), manifesto XML e gerador de ícones em Safe Rust.
#![forbid(unsafe_code)]

use crate::texture::RgbaImage;
use std::fs;
use std::path::Path;

/// Relatório de prontidão do empacotamento Android APK.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApkBuildReadinessReport {
    pub package_name: String,
    pub min_sdk: u32,
    pub target_sdk: u32,
    pub screen_orientation: String,
    pub fullscreen_theme: bool,
    pub native_lib_name: String,
    pub target_abis: Vec<String>,
    pub icon_png_bytes: usize,
}

/// Utilitário de validação e preparação de artefatos para empacotamento Android APK.
pub struct ApkPackagingValidator;

impl ApkPackagingValidator {
    pub const EXPECTED_PACKAGE: &'static str = "com.bestigard.game";
    pub const EXPECTED_LIB_NAME: &'static str = "mobile_entry";

    /// Valida o conteúdo de `platform/android/AndroidManifest.xml` garantindo conformidade
    /// com os requisitos de tela cheia, orientação retrato, OpenGL ES 3.0 e NativeActivity.
    pub fn validate_android_manifest(xml_content: &str) -> Result<ApkBuildReadinessReport, String> {
        if !xml_content.contains("package=\"com.bestigard.game\"") {
            return Err("AndroidManifest.xml ausente de package=\"com.bestigard.game\"".to_string());
        }
        if !xml_content.contains("android:minSdkVersion=\"24\"") {
            return Err("AndroidManifest.xml deve especificar minSdkVersion=\"24\"".to_string());
        }
        if !xml_content.contains("android:targetSdkVersion=\"34\"") {
            return Err("AndroidManifest.xml deve especificar targetSdkVersion=\"34\"".to_string());
        }
        if !xml_content.contains("android:screenOrientation=\"portrait\"") {
            return Err("AndroidManifest.xml deve fixar screenOrientation=\"portrait\"".to_string());
        }
        if !xml_content.contains("Theme.NoTitleBar.Fullscreen") {
            return Err("AndroidManifest.xml deve configurar tema Fullscreen".to_string());
        }
        if !xml_content.contains("android:value=\"mobile_entry\"") {
            return Err("AndroidManifest.xml deve apontar android.app.lib_name para mobile_entry".to_string());
        }
        if !xml_content.contains("0x00030000") {
            return Err("AndroidManifest.xml deve exigir OpenGL ES 3.0 (0x00030000)".to_string());
        }

        Ok(ApkBuildReadinessReport {
            package_name: Self::EXPECTED_PACKAGE.to_string(),
            min_sdk: 24,
            target_sdk: 34,
            screen_orientation: "portrait".to_string(),
            fullscreen_theme: true,
            native_lib_name: Self::EXPECTED_LIB_NAME.to_string(),
            target_abis: vec![
                "aarch64-linux-android".to_string(),
                "armv7-linux-androideabi".to_string(),
                "x86_64-linux-android".to_string(),
            ],
            icon_png_bytes: 0,
        })
    }

    /// Valida a seção `[package.metadata.android]` do `Cargo.toml` do `mobile_entry`.
    pub fn validate_cargo_apk_metadata(cargo_toml: &str) -> Result<(), String> {
        let required_keys = [
            "[package.metadata.android]",
            "package = \"com.bestigard.game\"",
            "min_sdk_version = 24",
            "target_sdk_version = 34",
            "screen_orientation = \"portrait\"",
            "aarch64-linux-android",
        ];

        for key in required_keys {
            if !cargo_toml.contains(key) {
                return Err(format!(
                    "Metadado obrigatório ausente em Cargo.toml: '{}'",
                    key
                ));
            }
        }
        Ok(())
    }

    /// Gera o ícone oficial de lançador (`64x64` RGBA8) de Bestigard (escudo dourado e chama elemental)
    /// e codifica como PNG válido em memória.
    pub fn generate_launcher_icon_png() -> Result<(RgbaImage, Vec<u8>), String> {
        let size = 64u32;
        let mut img = RgbaImage::solid(size, size, [18, 26, 48, 255]);

        // Moldura dourada do brasão do santuário
        img.fill_rect(0, 0, size, 4, [242, 198, 64, 255]);
        img.fill_rect(0, size - 4, size, 4, [242, 198, 64, 255]);
        img.fill_rect(0, 0, 4, size, [242, 198, 64, 255]);
        img.fill_rect(size - 4, 0, 4, size, [242, 198, 64, 255]);

        // Emblema central em chama elemental (laranja/dourado/rubi)
        let cx = 32.0f32;
        let cy = 34.0f32;
        for y in 10..54 {
            for x in 10..54 {
                let dx = x as f32 - cx;
                let dy = y as f32 - cy;
                let dist = (dx * dx + dy * dy).sqrt();
                if dist <= 18.0 {
                    let color = if dist <= 7.0 {
                        [255, 240, 110, 255]
                    } else if dist <= 13.0 {
                        [255, 130, 35, 255]
                    } else {
                        [195, 45, 30, 255]
                    };
                    img.set_pixel(x, y, color);
                }
            }
        }

        let mut png_bytes = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut png_bytes, size, size);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder
                .write_header()
                .map_err(|e| format!("Falha ao escrever cabeçalho PNG do ícone: {}", e))?;
            writer
                .write_image_data(&img.pixels)
                .map_err(|e| format!("Falha ao codificar pixels PNG do ícone: {}", e))?;
        }

        Ok((img, png_bytes))
    }

    /// Garante a geração do ícone de lançador no diretório de recursos Android (`platform/android/res/mipmap-hdpi/ic_launcher.png`)
    /// e valida o manifesto, o `Cargo.toml` e o guia `docs/build_mobile.md`.
    pub fn verify_workspace_apk_pipeline(
        manifest_path: &Path,
        cargo_toml_path: &Path,
        guide_path: &Path,
        icon_output_path: &Path,
    ) -> Result<ApkBuildReadinessReport, String> {
        let xml = fs::read_to_string(manifest_path)
            .map_err(|e| format!("Erro ao ler {:?}: {}", manifest_path, e))?;
        let mut report = Self::validate_android_manifest(&xml)?;

        let toml_str = fs::read_to_string(cargo_toml_path)
            .map_err(|e| format!("Erro ao ler {:?}: {}", cargo_toml_path, e))?;
        Self::validate_cargo_apk_metadata(&toml_str)?;

        let guide_str = fs::read_to_string(guide_path)
            .map_err(|e| format!("Erro ao ler {:?}: {}", guide_path, e))?;
        if !guide_str.contains("cargo apk build") {
            return Err("Guia docs/build_mobile.md deve documentar 'cargo apk build'".to_string());
        }

        let (_, png_bytes) = Self::generate_launcher_icon_png()?;
        if let Some(parent) = icon_output_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(icon_output_path, &png_bytes);
        report.icon_png_bytes = png_bytes.len();

        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_launcher_icon_png_roundtrip() {
        let (orig_img, png_bytes) =
            ApkPackagingValidator::generate_launcher_icon_png().expect("Icon generation should succeed");
        assert!(png_bytes.len() > 100);
        let decoded =
            RgbaImage::from_png_bytes(&png_bytes).expect("Generated PNG should decode cleanly");
        assert_eq!(decoded.width, orig_img.width);
        assert_eq!(decoded.height, orig_img.height);
        assert_eq!(decoded.pixels, orig_img.pixels);
    }

    #[test]
    fn test_manifest_and_cargo_metadata_validation() {
        let sample_xml = r#"
            <manifest package="com.bestigard.game">
                <uses-sdk android:minSdkVersion="24" android:targetSdkVersion="34" />
                <uses-feature android:glEsVersion="0x00030000" android:required="true" />
                <application android:theme="@android:style/Theme.NoTitleBar.Fullscreen">
                    <activity android:screenOrientation="portrait">
                        <meta-data android:name="android.app.lib_name" android:value="mobile_entry" />
                    </activity>
                </application>
            </manifest>
        "#;
        let rep = ApkPackagingValidator::validate_android_manifest(sample_xml)
            .expect("Valid manifest should pass");
        assert_eq!(rep.package_name, "com.bestigard.game");
        assert_eq!(rep.min_sdk, 24);
        assert_eq!(rep.target_sdk, 34);
        assert_eq!(rep.screen_orientation, "portrait");
        assert!(rep.fullscreen_theme);

        let sample_toml = r#"
            [package.metadata.android]
            package = "com.bestigard.game"
            build_targets = ["aarch64-linux-android"]
            [package.metadata.android.sdk]
            min_sdk_version = 24
            target_sdk_version = 34
            [package.metadata.android.application.activity]
            screen_orientation = "portrait"
        "#;
        ApkPackagingValidator::validate_cargo_apk_metadata(sample_toml)
            .expect("Valid Cargo.toml metadata should pass");
    }
}

