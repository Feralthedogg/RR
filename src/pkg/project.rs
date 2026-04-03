use super::env::{current_project_root_hint, find_outermost_manifest_root, load_workspace_from};
use super::git::clean_git_error;
use super::util::{
    create_synthetic_package_entry, is_major_version_segment, module_path_to_rel_path,
    write_lockfile,
};
use super::*;

pub fn is_package_import(import_path: &str) -> bool {
    let trimmed = import_path.trim();
    !(trimmed.starts_with("./")
        || trimmed.starts_with("../")
        || trimmed.starts_with('/')
        || trimmed.ends_with(".rr"))
}

pub fn resolve_import_path(importer_path: &Path, import_path: &str) -> RR<PathBuf> {
    if is_package_import(import_path) {
        resolve_package_import(importer_path, import_path)
    } else {
        let importer_dir = importer_path.parent().unwrap_or(Path::new("."));
        Ok(normalize_path(&importer_dir.join(import_path)))
    }
}

pub fn install_dependency_in_project(
    project_root: &Path,
    spec: &str,
) -> Result<InstallReport, String> {
    let mut manifest = Manifest::load_from_dir(project_root)?;
    let direct_request = parse_module_request(spec).map_err(clean_git_error)?;
    let mut probe_state = InstallState {
        package_home: package_home(),
        installed: BTreeMap::new(),
    };
    let resolved_direct = install_single_remote_module(&direct_request, true, &mut probe_state)
        .map_err(clean_git_error)?;
    manifest.requires.insert(
        resolved_direct.path.clone(),
        resolved_direct.version.clone(),
    );

    let all_modules =
        resolve_manifest_dependencies(project_root, &manifest).map_err(clean_git_error)?;
    manifest.write_to_dir(project_root)?;
    write_lockfile(project_root, all_modules.iter())?;

    Ok(InstallReport {
        project_root: project_root.to_path_buf(),
        direct_module: resolved_direct,
        all_modules,
    })
}

pub fn remove_dependency_from_project(
    project_root: &Path,
    module_path: &str,
) -> Result<(bool, bool, usize), String> {
    let mut manifest = Manifest::load_from_dir(project_root)?;
    let removed_require = manifest.requires.remove(module_path).is_some();
    let removed_replace = manifest.replaces.remove(module_path).is_some();
    if !removed_require && !removed_replace {
        return Err(format!(
            "dependency '{}' was not found in rr.mod",
            module_path
        ));
    }
    let all_modules =
        resolve_manifest_dependencies(project_root, &manifest).map_err(clean_git_error)?;
    manifest.write_to_dir(project_root)?;
    write_lockfile(project_root, all_modules.iter())?;
    Ok((removed_require, removed_replace, all_modules.len()))
}

pub fn tidy_project(project_root: &Path) -> Result<(usize, usize, usize), String> {
    let mut manifest = Manifest::load_from_dir(project_root)?;
    let old_requires = manifest.requires.clone();
    let used_imports = collect_project_package_imports(project_root)?;
    let mut new_requires = BTreeMap::new();
    let workspace = load_workspace_from(project_root).map_err(clean_git_error)?;

    for import_path in used_imports {
        if import_path == manifest.module_path
            || import_path
                .strip_prefix(manifest.module_path.as_str())
                .is_some_and(|rest| rest.starts_with('/'))
        {
            continue;
        }

        if workspace_member_for_import(workspace.as_ref(), &import_path)
            .map_err(clean_git_error)?
            .is_some()
        {
            continue;
        }

        if let Some((replace_path, _)) = best_matching_replace(&manifest, &import_path) {
            let version = old_requires
                .get(replace_path)
                .cloned()
                .unwrap_or_else(|| "v0.0.0".to_string());
            new_requires.insert(replace_path.clone(), version);
            continue;
        }

        if let Some((dep_path, version)) = best_matching_requirement(&manifest, &import_path) {
            new_requires.insert(dep_path.clone(), version.clone());
            continue;
        }

        let inferred = infer_module_path_for_import(&import_path);
        let request =
            parse_module_request(&format!("{inferred}@latest")).map_err(clean_git_error)?;
        let mut probe_state = InstallState {
            package_home: package_home(),
            installed: BTreeMap::new(),
        };
        let resolved = install_single_remote_module(&request, true, &mut probe_state)
            .map_err(clean_git_error)?;
        new_requires.insert(resolved.path, resolved.version);
    }

    let removed = old_requires
        .keys()
        .filter(|path| !new_requires.contains_key(*path))
        .count();
    let added = new_requires
        .keys()
        .filter(|path| !old_requires.contains_key(*path))
        .count();
    manifest.requires = new_requires;

    let all_modules =
        resolve_manifest_dependencies(project_root, &manifest).map_err(clean_git_error)?;
    manifest.write_to_dir(project_root)?;
    write_lockfile(project_root, all_modules.iter())?;
    Ok((added, removed, all_modules.len()))
}

pub fn vendor_project_dependencies(project_root: &Path) -> Result<usize, String> {
    let manifest = Manifest::load_from_dir(project_root)?;
    let entries = load_lockfile(project_root)?;
    if entries.is_empty() {
        return Err("rr.lock is empty; install dependencies before vendoring".to_string());
    }

    let vendor_root = project_root.join("vendor");
    if vendor_root.exists() {
        fs::remove_dir_all(&vendor_root)
            .map_err(|e| format!("failed to clear '{}': {}", vendor_root.display(), e))?;
    }
    fs::create_dir_all(&vendor_root)
        .map_err(|e| format!("failed to create '{}': {}", vendor_root.display(), e))?;

    let mut modules_txt = String::new();
    for entry in &entries {
        let source_root = if let Some(target) = manifest.replaces.get(&entry.path) {
            normalize_replace_target(project_root, target)
        } else {
            module_cache_dir(&package_home(), &entry.path, &entry.version)
        };
        if !source_root.is_dir() {
            return Err(format!(
                "failed to vendor '{}': source directory '{}' not found",
                entry.path,
                source_root.display()
            ));
        }
        let dest = vendor_root.join(module_path_to_rel_path(&entry.path));
        copy_dir_recursive(&source_root, &dest)?;
        modules_txt.push_str(&format!(
            "{} {}{}\n",
            entry.path,
            entry.version,
            if entry.direct { " direct" } else { "" }
        ));
    }

    fs::write(vendor_root.join("modules.txt"), modules_txt)
        .map_err(|e| format!("failed to write vendor/modules.txt: {}", e))?;
    Ok(entries.len())
}

pub fn graph_project_dependencies(project_root: &Path) -> Result<Vec<(String, String)>, String> {
    let manifest = Manifest::load_from_dir(project_root)?;
    let (manifest, locked, graph) = load_project_graph(project_root, &manifest)?;
    let locked_map = lock_map(&locked);
    let mut edges = Vec::new();

    let mut direct_requires: Vec<&String> = manifest.requires.keys().collect();
    direct_requires.sort();
    for dep in direct_requires {
        edges.push((
            manifest.module_path.clone(),
            display_locked_node(dep, &locked_map),
        ));
    }

    let mut modules: Vec<_> = graph.into_iter().collect();
    modules.sort_by(|a, b| a.0.cmp(&b.0));
    for (module, mut deps) in modules {
        deps.sort();
        for dep in deps {
            edges.push((
                display_locked_node(&module, &locked_map),
                display_locked_node(&dep, &locked_map),
            ));
        }
    }

    Ok(edges)
}

pub fn why_project_dependency(project_root: &Path, target: &str) -> Result<Vec<String>, String> {
    let manifest = Manifest::load_from_dir(project_root)?;
    let (manifest, locked, graph) = load_project_graph(project_root, &manifest)?;
    let locked_map = lock_map(&locked);
    let target_module = infer_module_path_for_import(target);
    let root = manifest.module_path.clone();
    let mut preds = BTreeMap::<String, Option<String>>::new();
    let mut queue = std::collections::VecDeque::new();

    preds.insert(root.clone(), None);
    queue.push_back(root.clone());

    while let Some(node) = queue.pop_front() {
        let mut deps = Vec::new();
        if node == root {
            deps.extend(manifest.requires.keys().cloned());
        } else if let Some(children) = graph.get(&node) {
            deps.extend(children.clone());
        }
        deps.sort();
        for dep in deps {
            if preds.contains_key(&dep) {
                continue;
            }
            preds.insert(dep.clone(), Some(node.clone()));
            queue.push_back(dep);
        }
    }

    if !preds.contains_key(&target_module) {
        return Ok(Vec::new());
    }

    let mut chain = Vec::new();
    let mut cursor = Some(target_module.clone());
    while let Some(node) = cursor {
        chain.push(if node == root {
            node.clone()
        } else {
            display_locked_node(&node, &locked_map)
        });
        cursor = preds.get(&node).cloned().flatten();
    }
    chain.reverse();
    Ok(chain)
}

pub fn verify_project_dependencies(project_root: &Path) -> Result<VerifyReport, String> {
    let manifest = Manifest::load_from_dir(project_root)?;
    let entries = load_lockfile(project_root)?;
    let mut mismatches = Vec::new();
    for entry in &entries {
        let source_root = source_root_for_locked_module(project_root, &manifest, entry);
        let actual_sum = directory_checksum(&source_root)?;
        if actual_sum != entry.sum {
            mismatches.push(VerifyMismatch {
                path: entry.path.clone(),
                source_root,
                expected_sum: entry.sum.clone(),
                actual_sum,
            });
        }
    }
    Ok(VerifyReport {
        checked: entries.len(),
        mismatches,
    })
}

pub fn outdated_direct_dependencies(
    project_root: &Path,
) -> Result<Vec<OutdatedDependency>, String> {
    let manifest = Manifest::load_from_dir(project_root)?;
    let entries = load_lockfile(project_root)?;
    let locked = lock_map(&entries);
    let mut deps = Vec::new();
    for path in manifest.requires.keys() {
        let Some(entry) = locked.get(path) else {
            continue;
        };
        let request = parse_module_request(&format!("{}@latest", entry.path))?;
        let latest = latest_available_version(&request)?;
        let status = match &latest {
            Some(version) if compare_versions(version, &entry.version) == Ordering::Greater => {
                "outdated"
            }
            Some(version) if version == &entry.version => "current",
            Some(_) => "ahead",
            None => "unknown",
        };
        deps.push(OutdatedDependency {
            path: entry.path.clone(),
            current_version: entry.version.clone(),
            latest_version: latest,
            status: status.to_string(),
        });
    }
    deps.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(deps)
}

pub fn update_project_dependencies(
    project_root: &Path,
    only_module: Option<&str>,
) -> Result<Vec<InstalledModule>, String> {
    let mut manifest = Manifest::load_from_dir(project_root)?;
    let keys: Vec<String> = manifest.requires.keys().cloned().collect();
    let mut changed = false;
    for path in keys {
        if only_module.is_some_and(|target| target != path) {
            continue;
        }
        let request = parse_module_request(&format!("{path}@latest")).map_err(clean_git_error)?;
        let mut probe_state = InstallState {
            package_home: package_home(),
            installed: BTreeMap::new(),
        };
        let resolved = install_single_remote_module(&request, true, &mut probe_state)
            .map_err(clean_git_error)?;
        if manifest.requires.get(&path) != Some(&resolved.version) {
            manifest.requires.insert(path.clone(), resolved.version);
            changed = true;
        }
    }

    let all_modules =
        resolve_manifest_dependencies(project_root, &manifest).map_err(clean_git_error)?;
    if changed {
        manifest.write_to_dir(project_root)?;
    }
    write_lockfile(project_root, all_modules.iter())?;
    Ok(all_modules)
}

fn resolve_package_import(importer_path: &Path, import_path: &str) -> RR<PathBuf> {
    let Some(module_root) =
        current_project_root_hint().or_else(|| find_outermost_manifest_root(importer_path))
    else {
        return Err(RRException::new(
            "RR.ParseError",
            RRCode::E0001,
            Stage::Parse,
            format!(
                "package import '{}' requires an rr.mod manifest at the project root",
                import_path
            ),
        )
        .help("run RR init to create an rr.mod manifest, or switch to a relative file import"));
    };

    let manifest = Manifest::load_from_dir(&module_root).map_err(|message| {
        RRException::new("RR.ParseError", RRCode::E0001, Stage::Parse, message)
    })?;

    let (package_dir, checksum_target) = if import_path == manifest.module_path {
        (module_root.join("src"), None)
    } else if let Some(suffix) = import_path
        .strip_prefix(manifest.module_path.as_str())
        .and_then(|rest| rest.strip_prefix('/'))
    {
        (module_root.join("src").join(suffix), None)
    } else if let Some((member_root, member_module)) = workspace_member_for_import(
        load_workspace_from(&module_root)
            .map_err(|message| {
                RRException::new("RR.ParseError", RRCode::E0001, Stage::Parse, message)
            })?
            .as_ref(),
        import_path,
    )
    .map_err(|message| RRException::new("RR.ParseError", RRCode::E0001, Stage::Parse, message))?
    {
        let package_dir = if import_path == member_module {
            member_root.join("src")
        } else {
            let suffix = import_path
                .strip_prefix(member_module.as_str())
                .and_then(|rest| rest.strip_prefix('/'))
                .unwrap_or_default();
            member_root.join("src").join(suffix)
        };
        (package_dir, None)
    } else if let Some((replace_path, target)) = best_matching_replace(&manifest, import_path) {
        let base = normalize_replace_target(&module_root, target);
        let package_dir = if import_path == replace_path {
            base.join("src")
        } else {
            let suffix = import_path
                .strip_prefix(replace_path.as_str())
                .and_then(|rest| rest.strip_prefix('/'))
                .unwrap_or_default();
            base.join("src").join(suffix)
        };
        let locked = find_matching_locked_module(&module_root, import_path);
        (
            package_dir,
            locked.map(|entry| (entry.path, entry.sum, base)),
        )
    } else if let Some(vendor_dir) = find_vendor_package_dir(&module_root, import_path) {
        let locked = find_matching_locked_module(&module_root, import_path);
        let verify_root = if let Some(ref entry) = locked {
            module_root
                .join("vendor")
                .join(module_path_to_rel_path(&entry.path))
        } else {
            vendor_dir.clone()
        };
        (
            vendor_dir,
            locked.map(|entry| (entry.path, entry.sum, verify_root)),
        )
    } else if let Some(locked) = find_matching_locked_module(&module_root, import_path) {
        let dep_root = module_cache_dir(&package_home(), &locked.path, &locked.version);
        let package_dir = if import_path == locked.path {
            dep_root.join("src")
        } else {
            let suffix = import_path
                .strip_prefix(locked.path.as_str())
                .and_then(|rest| rest.strip_prefix('/'))
                .unwrap_or_default();
            dep_root.join("src").join(suffix)
        };
        let verify_root = dep_root.clone();
        (package_dir, Some((locked.path, locked.sum, verify_root)))
    } else {
        let Some((dep_path, version)) = best_matching_requirement(&manifest, import_path) else {
            return Err(RRException::new(
                "RR.ParseError",
                RRCode::E0001,
                Stage::Parse,
                format!("package import not found: {}", import_path),
            )
            .note(format!("project module: {}", manifest.module_path))
            .help(format!(
                "run `RR install {}@latest` or add it to rr.mod",
                import_path
            )));
        };
        let dep_root = module_cache_dir(&package_home(), dep_path, version);
        let package_dir = if import_path == dep_path {
            dep_root.join("src")
        } else {
            let suffix = import_path
                .strip_prefix(dep_path.as_str())
                .and_then(|rest| rest.strip_prefix('/'))
                .unwrap_or_default();
            dep_root.join("src").join(suffix)
        };
        (package_dir, None)
    };

    if let Some((module_path, expected_sum, verify_root)) = checksum_target {
        verify_module_checksum(&module_root, &module_path, &verify_root, &expected_sum)?;
    }

    resolve_package_entry(&package_dir, import_path)
}

fn best_matching_requirement<'a>(
    manifest: &'a Manifest,
    import_path: &str,
) -> Option<(&'a String, &'a String)> {
    manifest
        .requires
        .iter()
        .filter(|(path, _)| {
            import_path == path.as_str()
                || import_path
                    .strip_prefix(path.as_str())
                    .is_some_and(|rest| rest.starts_with('/'))
        })
        .max_by_key(|(path, _)| path.len())
}

fn best_matching_replace<'a>(
    manifest: &'a Manifest,
    import_path: &str,
) -> Option<(&'a String, &'a String)> {
    manifest
        .replaces
        .iter()
        .filter(|(path, _)| {
            import_path == path.as_str()
                || import_path
                    .strip_prefix(path.as_str())
                    .is_some_and(|rest| rest.starts_with('/'))
        })
        .max_by_key(|(path, _)| path.len())
}

fn workspace_member_for_import(
    workspace: Option<&Workspace>,
    import_path: &str,
) -> Result<Option<(PathBuf, String)>, String> {
    let Some(workspace) = workspace else {
        return Ok(None);
    };

    let mut best: Option<(PathBuf, String)> = None;
    for member_root in &workspace.uses {
        let manifest = match Manifest::load_from_dir(member_root) {
            Ok(manifest) => manifest,
            Err(err) => {
                return Err(format!(
                    "failed to load workspace member '{}': {}",
                    member_root.display(),
                    err
                ));
            }
        };
        let module_path = manifest.module_path;
        if import_path == module_path
            || import_path
                .strip_prefix(module_path.as_str())
                .is_some_and(|rest| rest.starts_with('/'))
        {
            let candidate = (member_root.clone(), module_path.clone());
            match &best {
                Some((_, current)) if current.len() >= module_path.len() => {}
                _ => best = Some(candidate),
            }
        }
    }
    Ok(best)
}

fn find_matching_locked_module(project_root: &Path, import_path: &str) -> Option<InstalledModule> {
    let entries = load_lockfile(project_root).ok()?;
    entries
        .into_iter()
        .filter(|entry| {
            import_path == entry.path
                || import_path
                    .strip_prefix(entry.path.as_str())
                    .is_some_and(|rest| rest.starts_with('/'))
        })
        .max_by_key(|entry| entry.path.len())
}

fn find_vendor_package_dir(project_root: &Path, import_path: &str) -> Option<PathBuf> {
    let prefixes = import_path_prefixes(import_path);
    for prefix in prefixes {
        let module_dir = project_root
            .join("vendor")
            .join(module_path_to_rel_path(prefix));
        if !module_dir.is_dir() {
            continue;
        }
        let suffix = if import_path == prefix {
            String::new()
        } else {
            import_path
                .strip_prefix(prefix)
                .and_then(|rest| rest.strip_prefix('/'))
                .unwrap_or_default()
                .to_string()
        };
        let package_dir = if suffix.is_empty() {
            module_dir.join("src")
        } else {
            module_dir.join("src").join(suffix)
        };
        if package_dir.is_dir() || package_dir.join("lib.rr").is_file() {
            return Some(package_dir);
        }
    }
    None
}

fn import_path_prefixes(import_path: &str) -> Vec<&str> {
    let mut prefixes = Vec::new();
    let mut end = import_path.len();
    loop {
        prefixes.push(&import_path[..end]);
        let Some(pos) = import_path[..end].rfind('/') else {
            break;
        };
        end = pos;
    }
    prefixes
}

pub(super) fn normalize_replace_target(project_root: &Path, target: &str) -> PathBuf {
    let target_path = PathBuf::from(target);
    if target_path.is_absolute() {
        normalize_path(&target_path)
    } else {
        normalize_path(&project_root.join(target_path))
    }
}

fn resolve_package_entry(package_dir: &Path, import_path: &str) -> RR<PathBuf> {
    if package_dir.join("lib.rr").is_file() {
        return Ok(normalize_path(&package_dir.join("lib.rr")));
    }

    let files = collect_package_rr_files(package_dir).map_err(|message| {
        RRException::new("RR.ParseError", RRCode::E0001, Stage::Parse, message)
    })?;
    if files.is_empty() {
        return Err(RRException::new(
            "RR.ParseError",
            RRCode::E0001,
            Stage::Parse,
            format!("package import not found: {}", import_path),
        )
        .note(format!(
            "looked for package directory: {}",
            package_dir.display()
        ))
        .help("add lib.rr or at least one .rr file to that package directory"));
    }

    create_synthetic_package_entry(package_dir, import_path, &files)
        .map_err(|message| RRException::new("RR.ParseError", RRCode::E0001, Stage::Parse, message))
}

fn collect_package_rr_files(package_dir: &Path) -> Result<Vec<PathBuf>, String> {
    if !package_dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut files = Vec::new();
    for entry in fs::read_dir(package_dir)
        .map_err(|e| format!("failed to read '{}': {}", package_dir.display(), e))?
    {
        let entry = entry.map_err(|e| format!("failed to read directory entry: {}", e))?;
        let path = entry.path();
        if !path.is_file() || path.extension().and_then(|ext| ext.to_str()) != Some("rr") {
            continue;
        }
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");
        if name == "main.rr" {
            continue;
        }
        files.push(normalize_path(&path));
    }
    files.sort();
    Ok(files)
}

fn load_lockfile(project_root: &Path) -> Result<Vec<InstalledModule>, String> {
    let lock_path = project_root.join("rr.lock");
    let content = fs::read_to_string(&lock_path)
        .map_err(|e| format!("failed to read '{}': {}", lock_path.display(), e))?;
    let mut modules = Vec::new();
    let mut current: Option<InstalledModule> = None;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed == "version = 1" {
            continue;
        }
        if trimmed == "[[module]]" {
            if let Some(module) = current.take() {
                modules.push(module);
            }
            current = Some(InstalledModule {
                path: String::new(),
                version: String::new(),
                commit: String::new(),
                sum: String::new(),
                direct: false,
            });
            continue;
        }

        let Some(module) = current.as_mut() else {
            continue;
        };
        if let Some((key, value)) = trimmed.split_once('=') {
            let key = key.trim();
            let value = value.trim();
            match key {
                "path" => module.path = parse_toml_string(value)?,
                "version" => module.version = parse_toml_string(value)?,
                "commit" => module.commit = parse_toml_string(value)?,
                "sum" => module.sum = parse_toml_string(value)?,
                "direct" => module.direct = value == "true",
                _ => {}
            }
        }
    }

    if let Some(module) = current.take() {
        modules.push(module);
    }

    Ok(modules
        .into_iter()
        .filter(|module| !module.path.is_empty())
        .collect())
}

type ProjectGraphLoad = (
    Manifest,
    Vec<InstalledModule>,
    BTreeMap<String, Vec<String>>,
);

fn load_project_graph(
    project_root: &Path,
    manifest: &Manifest,
) -> Result<ProjectGraphLoad, String> {
    let locked = load_lockfile(project_root)?;
    let mut graph = BTreeMap::<String, Vec<String>>::new();

    for entry in &locked {
        let source_root = source_root_for_locked_module(project_root, manifest, entry);
        let dep_manifest = Manifest::load_from_dir(&source_root)?;
        let mut deps: Vec<String> = dep_manifest.requires.keys().cloned().collect();
        deps.sort();
        graph.insert(entry.path.clone(), deps);
    }

    Ok((manifest.clone(), locked, graph))
}

fn lock_map(entries: &[InstalledModule]) -> BTreeMap<String, InstalledModule> {
    let mut out = BTreeMap::new();
    for entry in entries {
        out.insert(entry.path.clone(), entry.clone());
    }
    out
}

fn display_locked_node(
    module_path: &str,
    locked_map: &BTreeMap<String, InstalledModule>,
) -> String {
    if let Some(entry) = locked_map.get(module_path) {
        format!("{}@{}", entry.path, entry.version)
    } else {
        module_path.to_string()
    }
}

fn source_root_for_locked_module(
    project_root: &Path,
    manifest: &Manifest,
    entry: &InstalledModule,
) -> PathBuf {
    if let Some(target) = manifest.replaces.get(&entry.path) {
        return normalize_replace_target(project_root, target);
    }
    let vendored = project_root
        .join("vendor")
        .join(module_path_to_rel_path(&entry.path));
    if vendored.is_dir() {
        return vendored;
    }
    module_cache_dir(&package_home(), &entry.path, &entry.version)
}

fn verify_module_checksum(
    project_root: &Path,
    module_path: &str,
    source_root: &Path,
    expected_sum: &str,
) -> RR<()> {
    let actual_sum = directory_checksum(source_root).map_err(|message| {
        RRException::new("RR.ParseError", RRCode::E0001, Stage::Parse, message)
    })?;
    if actual_sum == expected_sum {
        return Ok(());
    }

    Err(RRException::new(
        "RR.ParseError",
        RRCode::E0001,
        Stage::Parse,
        format!("checksum mismatch for module '{}'", module_path),
    )
    .note(format!("project root: {}", project_root.display()))
    .note(format!("source root: {}", source_root.display()))
    .note(format!("expected sum: {}", expected_sum))
    .note(format!("actual sum: {}", actual_sum))
    .help("rerun RR install or RR mod vendor to refresh the locked module contents"))
}

fn collect_project_package_imports(project_root: &Path) -> Result<Vec<String>, String> {
    let mut files = Vec::new();
    collect_project_rr_files(project_root, &mut files)?;
    files.sort();

    let mut imports = BTreeMap::<String, ()>::new();
    for file in files {
        let content = fs::read_to_string(&file)
            .map_err(|e| format!("failed to read '{}': {}", file.display(), e))?;
        for import_path in extract_plain_imports(&content) {
            if is_package_import(&import_path) {
                imports.insert(import_path, ());
            }
        }
    }
    Ok(imports.into_keys().collect())
}

fn collect_project_rr_files(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in
        fs::read_dir(dir).map_err(|e| format!("failed to read '{}': {}", dir.display(), e))?
    {
        let entry = entry.map_err(|e| format!("failed to read directory entry: {}", e))?;
        let path = entry.path();
        let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if path.is_dir() {
            if matches!(name, "Build" | "target" | ".git" | "vendor") {
                continue;
            }
            collect_project_rr_files(&path, out)?;
        } else if path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("rr") {
            out.push(path);
        }
    }
    Ok(())
}

fn extract_plain_imports(content: &str) -> Vec<String> {
    let mut imports = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with("import ") || trimmed.starts_with("import r ") {
            continue;
        }
        let rest = trimmed["import ".len()..].trim();
        if let Some(stripped) = rest.strip_prefix('"')
            && let Some(end) = stripped.find('"')
        {
            imports.push(stripped[..end].to_string());
        }
    }
    imports
}

fn infer_module_path_for_import(import_path: &str) -> String {
    let parts: Vec<&str> = import_path
        .split('/')
        .filter(|part| !part.is_empty())
        .collect();
    if parts.len() < 3 {
        return import_path.to_string();
    }
    if parts.first() == Some(&"github.com") {
        if parts.len() >= 4 && is_major_version_segment(parts[3]) {
            return parts[..4].join("/");
        }
        return parts[..3].join("/");
    }
    import_path.to_string()
}
