    use super::*;

    #[test]
    fn test_compare_semver() {
        assert_eq!(compare_semver("1.0.0", "1.0.0"), Ordering::Equal);
        assert_eq!(compare_semver("1.0.1", "1.0.0"), Ordering::Greater);
        assert_eq!(compare_semver("1.0.0", "1.1.0"), Ordering::Less);
        assert_eq!(compare_semver("v2.0.0", "1.9.9"), Ordering::Greater);
        assert_eq!(compare_semver("1.0.0", "v1.0.0"), Ordering::Equal);
    }

    #[test]
    fn test_compare_semver_prerelease() {
        assert_eq!(compare_semver("1.1.0", "1.1.0-beta.1"), Ordering::Greater);
        assert_eq!(compare_semver("1.1.0-beta.1", "1.1.0"), Ordering::Less);
        assert_eq!(
            compare_semver("1.1.0-beta.2", "1.1.0-beta.1"),
            Ordering::Greater
        );
        assert_eq!(
            compare_semver("1.1.0-beta.10", "1.1.0-beta.2"),
            Ordering::Greater
        );
        assert_eq!(
            compare_semver("1.1.0-rc.1", "1.1.0-beta.9"),
            Ordering::Greater
        );
        assert_eq!(
            compare_semver("1.1.0-beta.1+build.42", "1.1.0-beta.1"),
            Ordering::Equal
        );
    }

    #[test]
    fn test_sha256_nist_vectors() {
        assert_eq!(
            compute_sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            compute_sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            compute_sha256_hex(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
    }

    #[test]
    fn test_parse_version_manifest_windows_self_update() {
        let valid_sha = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let manifest = serde_json::json!({
            "version": "1.2.0",
            "published_at": "2026-11-11",
            "notes": "Self update test",
            "download_url": "https://github.com/mitrafun-creator/CecDesk/releases",
            "windows_package_url": "https://github.com/mitrafun-creator/CecDesk/releases/download/v1.2.0/CecDesk_Windows_x64.zip",
            "windows_sha256": valid_sha.to_uppercase()
        })
        .to_string();

        let win_info = parse_version_manifest(&manifest, "1.1.0-beta.1", true).unwrap();
        assert!(win_info.has_update);
        assert!(win_info.can_self_update);
        assert_eq!(win_info.package_kind.as_deref(), Some("zip"));
        assert_eq!(win_info.package_sha256.as_deref(), Some(valid_sha));

        let linux_info = parse_version_manifest(&manifest, "1.1.0-beta.1", false).unwrap();
        assert!(linux_info.has_update);
        assert!(!linux_info.can_self_update);

        let same_ver_info = parse_version_manifest(&manifest, "1.2.0", true).unwrap();
        assert!(!same_ver_info.has_update);
        assert!(!same_ver_info.can_self_update);

        let bad_host = serde_json::json!({
            "version": "1.2.0",
            "windows": {
                "url": "https://evil.example.com/update.exe",
                "sha256": valid_sha
            }
        })
        .to_string();
        let bad_info = parse_version_manifest(&bad_host, "1.1.0-beta.1", true).unwrap();
        assert!(!bad_info.can_self_update);
        assert!(bad_info.package_url.is_none());

        let missing_sha = serde_json::json!({
            "version": "1.2.0",
            "windows_package_url": "https://github.com/mitrafun-creator/CecDesk/raw/main/CecDesk.exe"
        })
        .to_string();
        let no_sha_info = parse_version_manifest(&missing_sha, "1.1.0-beta.1", true).unwrap();
        assert!(!no_sha_info.can_self_update);
    }

    #[test]
    fn test_extract_safe_update_zip_and_zip_slip_blocked() {
        let root = std::env::temp_dir().join(format!("hdmi_cec_zip_upd_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();

        let mut fake_exe = vec![b'M', b'Z'];
        fake_exe.resize(1_050_000, 0x41);
        let fw_bytes = b"UF2\nPAYLOAD";
        let prof_bytes = br#"{"id":"lg_test"}"#;

        let zip_bytes = crate::telemetry::build_zip_archive(&[
            ("CecDesk_Win/CecDesk.exe", &fake_exe),
            ("CecDesk_Win/bin/firmware/rp2040_zero_firmware.uf2", fw_bytes),
            ("CecDesk_Win/tv_profiles/lg_test.json", prof_bytes),
            ("CecDesk_Win/extra/ignored.txt", b"ignore"),
        ])
        .unwrap();

        let zip_path = root.join("ok.zip");
        fs::write(&zip_path, &zip_bytes).unwrap();
        let digest = compute_file_sha256_hex(&zip_path).unwrap();
        assert_eq!(digest, compute_sha256_hex(&zip_bytes));

        let stage = root.join("stage");
        let extracted = extract_safe_update_zip(&zip_path, &stage, "CecDesk.exe").unwrap();
        assert_eq!(extracted.len(), 3);
        assert!(verify_windows_pe_binary(&stage.join("CecDesk.exe")).is_ok());
        assert_eq!(
            fs::read(stage.join("bin/firmware/rp2040_zero_firmware.uf2")).unwrap(),
            fw_bytes
        );
        assert_eq!(
            fs::read(stage.join("tv_profiles/lg_test.json")).unwrap(),
            prof_bytes
        );
        assert!(!stage.join("extra/ignored.txt").exists());

        let evil_zip = crate::telemetry::build_zip_archive(&[
            ("../../evil.exe", &fake_exe),
        ])
        .unwrap();
        let evil_path = root.join("evil.zip");
        fs::write(&evil_path, &evil_zip).unwrap();
        assert!(extract_safe_update_zip(&evil_path, &root.join("stage_evil"), "CecDesk.exe").is_err());

        let not_pe = root.join("not_pe.exe");
        fs::write(&not_pe, vec![0x00u8; 1_050_000]).unwrap();
        assert!(verify_windows_pe_binary(&not_pe).is_err());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_repo_manifest_and_windows_zip_bundle_compatibility() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..");
        let version_txt = fs::read_to_string(repo_root.join("version.json"))
            .expect("version.json must exist in repo root");
        let parsed = parse_version_manifest(&version_txt, "1.1.0-beta.0", true)
            .expect("version.json must parse cleanly");
        assert!(parsed.has_update);
        assert!(parsed.can_self_update);
        assert_eq!(parsed.package_kind.as_deref(), Some("zip"));

        let sha = parsed.package_sha256.expect("windows_sha256 must be present");
        assert!(
            sha.chars().any(|c| c != '0'),
            "windows_sha256 in version.json must not remain all-zeros"
        );

        let zip_path = repo_root.join("CecDesk_Windows_x64.zip");
        if zip_path.is_file() {
            let actual_sha = compute_file_sha256_hex(&zip_path).unwrap();
            assert_eq!(
                actual_sha, sha,
                "version.json windows_sha256 must match CecDesk_Windows_x64.zip"
            );

            let stage = std::env::temp_dir()
                .join(format!("cecdesk_real_zip_verify_{}", std::process::id()));
            let _ = fs::remove_dir_all(&stage);
            let extracted = extract_safe_update_zip(&zip_path, &stage, "CecDesk.exe")
                .expect("CecDesk_Windows_x64.zip must unpack via extract_safe_update_zip");

            assert!(verify_windows_pe_binary(&stage.join("CecDesk.exe")).is_ok());
            assert!(stage.join("WebView2Loader.dll").is_file());
            assert!(stage.join("README.md").is_file());
            assert!(stage.join("LICENSE").is_file());
            assert!(stage.join("THIRD_PARTY_NOTICES.md").is_file());
            assert!(stage.join("bin/AudioTool.exe").is_file());
            assert!(stage.join("bin/firmware/rp2040_zero_firmware.uf2").is_file());
            assert!(stage.join("bin/firmware/esp32_s3_firmware.bin").is_file());
            assert!(stage.join("tv_profiles/universal_hdmi_cec.json").is_file());
            assert!(extracted.len() >= 20);

            let _ = fs::remove_dir_all(&stage);
        }
    }
