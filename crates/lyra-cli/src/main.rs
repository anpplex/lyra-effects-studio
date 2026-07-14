use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use clap::{Parser, Subcommand};
use lyra_pack::{PackArchiver, PackValidator, canonical_json};
use lyra_registry::{LicenseAudit, RegistryCatalog, RegistrySigner, RegistryVerifier};
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::ExitCode;

const EXIT_USAGE: u8 = 64;
const EXIT_DATA: u8 = 65;
const EXIT_SOFTWARE: u8 = 70;

#[derive(Debug, Parser)]
#[command(name = "lyra-effects", disable_version_flag = true, color = clap::ColorChoice::Never)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Validate {
        pack_directory: PathBuf,
    },
    Pack {
        pack_directory: PathBuf,
        output: PathBuf,
    },
    Registry {
        #[command(subcommand)]
        operation: RegistryOperation,
    },
    LicenseAudit {
        registry_directory: PathBuf,
    },
}

#[derive(Debug, Subcommand)]
enum RegistryOperation {
    Build {
        catalog: PathBuf,
        output_directory: PathBuf,
        private_key_file: PathBuf,
    },
    Verify {
        catalog: PathBuf,
        signature_file: PathBuf,
        public_key_file: PathBuf,
    },
    VerifySite {
        registry_site_directory: PathBuf,
    },
    SignChecksum {
        lowercase_sha256: String,
        private_key_file: PathBuf,
    },
}

#[derive(Debug)]
struct Outcome {
    command: String,
    code: u8,
    data: Option<Value>,
    message: Option<String>,
}

impl Outcome {
    fn success(command: &str, data: Value) -> Self {
        Self {
            command: command.into(),
            code: 0,
            data: Some(data),
            message: None,
        }
    }

    fn failure(command: &str, code: u8, message: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            code,
            data: None,
            message: Some(message.into()),
        }
    }

    fn with_data(mut self, data: Value) -> Self {
        self.data = Some(data);
        self
    }

    fn response(&self) -> Value {
        let mut response = json!({
            "command": self.command,
            "ok": self.code == 0,
        });
        if let Some(data) = &self.data {
            response["data"] = data.clone();
        }
        if let Some(message) = &self.message {
            response["message"] = Value::String(message.clone());
        }
        response
    }
}

fn main() -> ExitCode {
    let arguments: Vec<_> = std::env::args_os().collect();
    if arguments.len() == 2
        && arguments[1]
            .to_str()
            .is_some_and(|value| matches!(value, "--version" | "-V"))
    {
        println!("lyra-effects {}", env!("CARGO_PKG_VERSION"));
        return ExitCode::SUCCESS;
    }

    let outcome = match Cli::try_parse_from(arguments) {
        Ok(cli) => execute(cli.command)
            .unwrap_or_else(|message| Outcome::failure("internal", EXIT_SOFTWARE, message)),
        Err(error) => Outcome::failure("usage", EXIT_USAGE, error.to_string()),
    };
    let code = outcome.code;
    match canonical_json::to_vec(&outcome.response()) {
        Ok(bytes) => print!("{}", String::from_utf8_lossy(&bytes)),
        Err(error) => {
            println!(
                "{{\"command\":\"internal\",\"message\":\"{}\",\"ok\":false}}",
                error.to_string().replace('"', "\\\"")
            );
            return ExitCode::from(EXIT_SOFTWARE);
        }
    }
    ExitCode::from(code)
}

fn execute(command: Command) -> Result<Outcome, String> {
    match command {
        Command::Validate { pack_directory } => validate(&pack_directory),
        Command::Pack {
            pack_directory,
            output,
        } => pack(&pack_directory, &output),
        Command::Registry { operation } => registry(operation),
        Command::LicenseAudit { registry_directory } => license_audit(&registry_directory),
    }
}

fn validate(pack_directory: &Path) -> Result<Outcome, String> {
    let diagnostics = PackValidator::default()
        .validate(pack_directory)
        .map_err(|error| error.to_string())?;
    let values: Vec<_> = diagnostics
        .iter()
        .map(|item| {
            json!({
                "code": item.code,
                "message": item.message,
                "path": item.path,
                "severity": "error",
            })
        })
        .collect();
    let data = json!({
        "diagnostics": values,
        "errorCount": diagnostics.len(),
    });
    if diagnostics.is_empty() {
        Ok(Outcome::success("validate", data))
    } else {
        Ok(Outcome::failure("validate", EXIT_DATA, "Pack validation failed").with_data(data))
    }
}

fn pack(pack_directory: &Path, output: &Path) -> Result<Outcome, String> {
    let artifact = PackArchiver::default()
        .build(pack_directory, output)
        .map_err(|error| error.to_string())?;
    Ok(Outcome::success(
        "pack",
        json!({
            "path": artifact.path,
            "sha256": artifact.sha256,
            "byteCount": artifact.byte_count,
        }),
    ))
}

fn registry(operation: RegistryOperation) -> Result<Outcome, String> {
    match operation {
        RegistryOperation::Build {
            catalog,
            output_directory,
            private_key_file,
        } => registry_build(&catalog, &output_directory, &private_key_file),
        RegistryOperation::Verify {
            catalog,
            signature_file,
            public_key_file,
        } => registry_verify(&catalog, &signature_file, &public_key_file),
        RegistryOperation::VerifySite {
            registry_site_directory,
        } => registry_verify_site(&registry_site_directory),
        RegistryOperation::SignChecksum {
            lowercase_sha256,
            private_key_file,
        } => registry_sign_checksum(&lowercase_sha256, &private_key_file),
    }
}

fn registry_build(catalog_path: &Path, output: &Path, key_path: &Path) -> Result<Outcome, String> {
    let mut catalog = read_catalog(catalog_path)?;
    validate_catalog(&catalog)?;
    catalog
        .packs
        .sort_by(|left, right| (&left.id, &left.version).cmp(&(&right.id, &right.version)));
    let signer = load_signer(key_path)?;
    let signature = signer
        .sign_catalog(&catalog)
        .map_err(|error| error.to_string())?;
    fs::create_dir_all(output).map_err(|error| format!("{}: {error}", output.display()))?;
    fs::write(
        output.join("registry-v1.json"),
        catalog
            .to_canonical_vec()
            .map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    fs::write(output.join("registry-v1.sig"), format!("{signature}\n"))
        .map_err(|error| error.to_string())?;
    fs::write(
        output.join("public-key.txt"),
        format!("{}\n", signer.public_key_base64()),
    )
    .map_err(|error| error.to_string())?;
    Ok(Outcome::success(
        "registry build",
        json!({"packCount": catalog.packs.len(), "output": output}),
    ))
}

fn registry_verify(
    catalog_path: &Path,
    signature_path: &Path,
    public_key_path: &Path,
) -> Result<Outcome, String> {
    let catalog = read_catalog(catalog_path)?;
    validate_catalog(&catalog)?;
    let signature = read_trimmed(signature_path)?;
    let verifier = RegistryVerifier::from_public_key_base64(&read_trimmed(public_key_path)?)
        .map_err(|error| error.to_string())?;
    let valid = verifier
        .verify_catalog(&catalog, &signature)
        .map_err(|error| error.to_string())?;
    let data = json!({"valid": valid, "packCount": catalog.packs.len()});
    if valid {
        Ok(Outcome::success("registry verify", data))
    } else {
        Ok(Outcome::failure(
            "registry verify",
            EXIT_DATA,
            "Registry signature is invalid",
        )
        .with_data(data))
    }
}

fn registry_verify_site(root: &Path) -> Result<Outcome, String> {
    let catalog = read_catalog(&root.join("registry-v1.json"))?;
    validate_catalog(&catalog)?;
    let signature = read_trimmed(&root.join("registry-v1.sig"))?;
    let verifier =
        RegistryVerifier::from_public_key_base64(&read_trimmed(&root.join("public-key.txt"))?)
            .map_err(|error| error.to_string())?;
    if !verifier
        .verify_catalog(&catalog, &signature)
        .map_err(|error| error.to_string())?
    {
        return Ok(Outcome::failure(
            "registry verify-site",
            EXIT_DATA,
            "Registry signature is invalid",
        ));
    }
    for pack in &catalog.packs {
        let archive = safe_join(root, &pack.download_url)?;
        let bytes =
            fs::read(&archive).map_err(|error| format!("{}: {error}", archive.display()))?;
        if !verifier.verify_pack(&bytes, &pack.sha256, &pack.signature) {
            return Ok(Outcome::failure(
                "registry verify-site",
                EXIT_DATA,
                format!("Pack verification failed: {}@{}", pack.id, pack.version),
            ));
        }
    }
    Ok(Outcome::success(
        "registry verify-site",
        json!({"valid": true, "packCount": catalog.packs.len()}),
    ))
}

fn registry_sign_checksum(checksum: &str, key_path: &Path) -> Result<Outcome, String> {
    if checksum.len() != 64
        || !checksum
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Ok(Outcome::failure(
            "registry sign-checksum",
            EXIT_DATA,
            "Checksum must be 64 lowercase hexadecimal characters",
        ));
    }
    let signer = load_signer(key_path)?;
    Ok(Outcome::success(
        "registry sign-checksum",
        json!({
            "signature": signer.sign_checksum(checksum),
            "publicKey": signer.public_key_base64(),
        }),
    ))
}

fn license_audit(root: &Path) -> Result<Outcome, String> {
    let bytes = fs::read(root.join("license-audit.json")).map_err(|error| error.to_string())?;
    let audit = LicenseAudit::from_slice(&bytes).map_err(|error| error.to_string())?;
    let diagnostics = audit.validate_sources(root);
    let data = json!({
        "includedCount": audit.included.len(),
        "excludedCount": audit.excluded.len(),
        "diagnosticCount": diagnostics.len(),
    });
    if let Some(first) = diagnostics.first() {
        Ok(Outcome::failure(
            "license-audit",
            EXIT_DATA,
            format!("{}: {}", first.code, first.message),
        )
        .with_data(data))
    } else {
        Ok(Outcome::success("license-audit", data))
    }
}

fn read_catalog(path: &Path) -> Result<RegistryCatalog, String> {
    let bytes = fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
    RegistryCatalog::from_slice(&bytes).map_err(|error| error.to_string())
}

fn read_trimmed(path: &Path) -> Result<String, String> {
    fs::read_to_string(path)
        .map(|value| value.trim().into())
        .map_err(|error| format!("{}: {error}", path.display()))
}

fn load_signer(path: &Path) -> Result<RegistrySigner, String> {
    let source = read_trimmed(path)?;
    let bytes = STANDARD.decode(source).map_err(|error| error.to_string())?;
    let length = bytes.len();
    let bytes: [u8; 32] = bytes
        .try_into()
        .map_err(|_| format!("private key must contain 32 bytes, got {length}"))?;
    Ok(RegistrySigner::from_private_key_bytes(bytes))
}

fn validate_catalog(catalog: &RegistryCatalog) -> Result<(), String> {
    let mut versions = BTreeSet::new();
    for pack in &catalog.packs {
        if !versions.insert((pack.id.as_str(), pack.version.to_string())) {
            return Err(format!(
                "duplicate Pack version: {}@{}",
                pack.id, pack.version
            ));
        }
        if pack.sha256.len() != 64
            || !pack
                .sha256
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(format!("invalid Pack checksum: {}", pack.id));
        }
        safe_relative(&pack.download_url)?;
        safe_relative(&pack.manifest_url)?;
    }
    Ok(())
}

fn safe_join(root: &Path, relative: &str) -> Result<PathBuf, String> {
    safe_relative(relative)?;
    Ok(root.join(relative))
}

fn safe_relative(relative: &str) -> Result<(), String> {
    let path = Path::new(relative);
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_) | Component::CurDir))
    {
        Err(format!("unsafe Registry path: {relative}"))
    } else {
        Ok(())
    }
}
