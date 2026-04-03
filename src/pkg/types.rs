use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Clone, Debug, Default)]
pub struct Manifest {
    pub module_path: String,
    pub rr_version: Option<String>,
    pub description: Option<String>,
    pub license: Option<String>,
    pub homepage: Option<String>,
    pub requires: BTreeMap<String, String>,
    pub replaces: BTreeMap<String, String>,
}

#[derive(Clone, Debug)]
pub struct InstalledModule {
    pub path: String,
    pub version: String,
    pub commit: String,
    pub sum: String,
    pub direct: bool,
}

#[derive(Clone, Debug)]
pub struct InstallReport {
    pub project_root: PathBuf,
    pub direct_module: InstalledModule,
    pub all_modules: Vec<InstalledModule>,
}

#[derive(Clone, Debug)]
pub struct VerifyMismatch {
    pub path: String,
    pub source_root: PathBuf,
    pub expected_sum: String,
    pub actual_sum: String,
}

#[derive(Clone, Debug)]
pub struct VerifyReport {
    pub checked: usize,
    pub mismatches: Vec<VerifyMismatch>,
}

#[derive(Clone, Debug)]
pub struct OutdatedDependency {
    pub path: String,
    pub current_version: String,
    pub latest_version: Option<String>,
    pub status: String,
}

#[derive(Clone, Debug)]
pub struct PublishReport {
    pub archive_path: PathBuf,
    pub included_files: Vec<String>,
    pub dry_run: bool,
    pub tag: Option<String>,
    pub tag_pushed: bool,
}

#[derive(Clone, Debug, Default)]
pub struct PublishOptions {
    pub dry_run: bool,
    pub allow_dirty: bool,
    pub push_tag: bool,
    pub remote: Option<String>,
    pub registry: Option<PathBuf>,
}

#[derive(Clone, Debug)]
pub struct RegistrySearchResult {
    pub path: String,
    pub latest_version: Option<String>,
    pub description: Option<String>,
    pub license: Option<String>,
    pub deprecated: Option<String>,
    pub release_count: usize,
    pub yanked_count: usize,
    pub pending_count: usize,
}

#[derive(Clone, Debug)]
pub struct RegistryInfo {
    pub path: String,
    pub description: Option<String>,
    pub license: Option<String>,
    pub homepage: Option<String>,
    pub deprecated: Option<String>,
    pub channels: BTreeMap<String, String>,
    pub releases: Vec<RegistryReleaseInfo>,
}

#[derive(Clone, Debug)]
pub struct RegistryReleaseInfo {
    pub version: String,
    pub archive_rel: String,
    pub archive_sum: String,
    pub file_count: usize,
    pub yanked: bool,
    pub approved: bool,
    pub signed: bool,
    pub signer: Option<String>,
    pub signature_scheme: Option<String>,
}

#[derive(Clone, Debug)]
pub struct RegistryVerifyIssue {
    pub path: String,
    pub version: String,
    pub archive_path: PathBuf,
    pub message: String,
}

#[derive(Clone, Debug)]
pub struct RegistryVerifyReport {
    pub checked_modules: usize,
    pub checked_releases: usize,
    pub issues: Vec<RegistryVerifyIssue>,
}

#[derive(Clone, Debug)]
pub struct RegistryKeygenReport {
    pub public_key_hex: String,
    pub secret_key_hex: String,
    pub identity: Option<String>,
    pub written_files: Vec<PathBuf>,
}

#[derive(Clone, Debug)]
pub struct RegistryPolicyLintReport {
    pub path: PathBuf,
    pub exists: bool,
    pub require_signed: bool,
    pub require_approval: bool,
    pub trusted_count: usize,
    pub revoked_count: usize,
    pub allowed_signer_count: usize,
    pub auto_approve_signer_count: usize,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct RegistryQueueItem {
    pub path: String,
    pub version: String,
    pub yanked: bool,
    pub signed: bool,
    pub signer: Option<String>,
}

#[derive(Clone, Debug)]
pub struct RegistryAuditEntry {
    pub timestamp_secs: u64,
    pub action: String,
    pub detail: String,
}

#[derive(Clone, Debug)]
pub struct RegistryReportModule {
    pub path: String,
    pub latest_version: Option<String>,
    pub channel_count: usize,
    pub release_count: usize,
    pub approved_count: usize,
    pub pending_count: usize,
    pub yanked_count: usize,
    pub signed_count: usize,
    pub deprecated: bool,
}

#[derive(Clone, Debug)]
pub struct RegistryReport {
    pub module_count: usize,
    pub channel_count: usize,
    pub release_count: usize,
    pub approved_count: usize,
    pub pending_count: usize,
    pub yanked_count: usize,
    pub signed_count: usize,
    pub deprecated_module_count: usize,
    pub modules: Vec<RegistryReportModule>,
}

#[derive(Clone, Debug)]
pub struct RegistryDiffReport {
    pub module_path: String,
    pub from_version: String,
    pub to_version: String,
    pub from_approved: bool,
    pub to_approved: bool,
    pub from_yanked: bool,
    pub to_yanked: bool,
    pub from_signed: bool,
    pub to_signed: bool,
    pub from_signer: Option<String>,
    pub to_signer: Option<String>,
    pub added_files: Vec<String>,
    pub removed_files: Vec<String>,
    pub changed_files: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct RegistryRiskFactor {
    pub key: String,
    pub points: u32,
    pub detail: String,
}

#[derive(Clone, Debug)]
pub struct RegistryRiskReport {
    pub module_path: String,
    pub version: String,
    pub baseline_version: Option<String>,
    pub score: u32,
    pub level: String,
    pub factors: Vec<RegistryRiskFactor>,
}

#[derive(Clone, Debug)]
pub struct RegistryPolicyShowReport {
    pub path: PathBuf,
    pub exists: bool,
    pub content: String,
}

#[derive(Clone, Debug)]
pub struct RegistryOnboardReport {
    pub keygen: RegistryKeygenReport,
    pub policy_path: PathBuf,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct RegistryAuditFilter<'a> {
    pub action: Option<&'a str>,
    pub module: Option<&'a str>,
    pub contains: Option<&'a str>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Workspace {
    pub root: PathBuf,
    pub uses: Vec<PathBuf>,
}

#[derive(Clone, Debug)]
pub(crate) struct ModuleRequest {
    pub module_path: String,
    pub source: ModuleSource,
    pub subdir: PathBuf,
    pub requested: RequestedVersion,
}

#[derive(Clone, Debug)]
pub(crate) enum ModuleSource {
    Git { repo_url: String },
    Registry { registry_root: PathBuf },
}

#[derive(Clone, Debug)]
pub(crate) enum RequestedVersion {
    Latest,
    Exact(String),
    Channel(String),
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedVersion {
    pub version: String,
    pub checkout_ref: Option<String>,
}

pub(crate) struct InstallState {
    pub package_home: PathBuf,
    pub installed: BTreeMap<String, InstalledModule>,
}

#[derive(Clone, Debug)]
pub(crate) struct SelectedModule {
    pub version: String,
    pub direct: bool,
}

pub(crate) struct LoadedModule {
    pub installed: InstalledModule,
    pub manifest: Manifest,
    pub root: PathBuf,
}

#[derive(Clone, Debug)]
pub(crate) struct RegistryEntry {
    pub version: String,
    pub archive_rel: String,
    pub archive_sum: String,
    pub file_count: usize,
    pub yanked: bool,
    pub approved: bool,
    pub archive_sig: Option<String>,
    pub signer: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct RegistryIndex {
    pub module_path: String,
    pub description: Option<String>,
    pub license: Option<String>,
    pub homepage: Option<String>,
    pub deprecated: Option<String>,
    pub channels: BTreeMap<String, String>,
    pub releases: Vec<RegistryEntry>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct RegistryTrustPolicy {
    pub require_signed: bool,
    pub require_approval: bool,
    pub trusted_ed25519_keys: Vec<String>,
    pub revoked_ed25519_keys: Vec<String>,
    pub allowed_signers: Vec<String>,
    pub auto_approve_signers: Vec<String>,
}
