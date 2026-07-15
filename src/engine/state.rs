use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs::{self, File, OpenOptions, TryLockError};
use std::io::{self, ErrorKind, Write};
use std::path::{Path, PathBuf};

#[cfg(windows)]
use std::ffi::c_void;
#[cfg(windows)]
use std::os::windows::{fs::OpenOptionsExt, io::AsRawHandle};

use crate::adaptive_state::AdaptiveState;
use crate::experience::ExperienceStore;
use crate::hardware::HardwareSnapshot;
use crate::kv_cache::{KvFusionCache, MemoryCompactionPolicy, MemoryRetentionPolicy};

use super::NoironEngine;

const FULL_STATE_MANIFEST_HEADER: &str = "noiron-full-state-v1";
const FULL_STATE_MANIFEST_SUFFIX: &str = ".full-state.current";
const LEGACY_FULL_STATE_GENERATION: u64 = 0;

#[derive(Debug, Clone)]
pub(super) struct FullStateBinding {
    memory: String,
    experience: String,
    adaptive: String,
    generation: u64,
}

impl FullStateBinding {
    fn new(paths: &FullStatePaths, generation: u64) -> io::Result<Self> {
        Ok(Self {
            memory: path_identity(&paths.memory)?,
            experience: path_identity(&paths.experience)?,
            adaptive: path_identity(&paths.adaptive)?,
            generation,
        })
    }

    fn matches(&self, paths: &FullStatePaths) -> io::Result<bool> {
        Ok(self.memory == path_identity(&paths.memory)?
            && self.experience == path_identity(&paths.experience)?
            && self.adaptive == path_identity(&paths.adaptive)?)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FullStateSaveStage {
    MemoryStaged,
    ExperienceStaged,
    AdaptiveStaged,
    ManifestStaged,
    CurrentBackedUp,
    ManifestPublished,
}

#[derive(Debug, Clone)]
struct FullStatePaths {
    memory: PathBuf,
    experience: PathBuf,
    adaptive: PathBuf,
    manifest: PathBuf,
    manifest_next: PathBuf,
    manifest_backup: PathBuf,
    manifest_retired: PathBuf,
    writer_lock: PathBuf,
}

#[derive(Debug, Clone)]
struct FullStateGenerationPaths {
    memory: PathBuf,
    experience: PathBuf,
    adaptive: PathBuf,
}

impl FullStatePaths {
    fn new(memory: &Path, experience: &Path, adaptive: &Path) -> io::Result<Self> {
        if memory == experience || memory == adaptive || experience == adaptive {
            return Err(io::Error::new(
                ErrorKind::InvalidInput,
                "memory, experience, and adaptive state paths must be distinct",
            ));
        }
        let manifest = append_path_suffix(adaptive, FULL_STATE_MANIFEST_SUFFIX);
        let manifest_backup = append_path_suffix(&manifest, ".bak");
        let paths = Self {
            memory: memory.to_path_buf(),
            experience: experience.to_path_buf(),
            adaptive: adaptive.to_path_buf(),
            manifest_next: append_path_suffix(&manifest, ".next"),
            manifest_retired: append_path_suffix(&manifest_backup, ".retired"),
            writer_lock: append_path_suffix(&manifest, ".lock"),
            manifest_backup,
            manifest,
        };
        validate_full_state_paths(&paths)?;
        Ok(paths)
    }

    fn generation(&self, generation: u64) -> io::Result<FullStateGenerationPaths> {
        if generation == LEGACY_FULL_STATE_GENERATION {
            return Ok(FullStateGenerationPaths {
                memory: self.memory.clone(),
                experience: self.experience.clone(),
                adaptive: self.adaptive.clone(),
            });
        }
        Ok(FullStateGenerationPaths {
            memory: generation_path(&self.memory, generation)?,
            experience: generation_path(&self.experience, generation)?,
            adaptive: generation_path(&self.adaptive, generation)?,
        })
    }
}

fn validate_full_state_paths(paths: &FullStatePaths) -> io::Result<()> {
    for path in [&paths.memory, &paths.experience, &paths.adaptive] {
        let file_name = path.file_name().ok_or_else(|| {
            io::Error::new(
                ErrorKind::InvalidInput,
                format!("full-state base path has no file name: {}", path.display()),
            )
        })?;
        let file_name = file_name.to_string_lossy().to_ascii_lowercase();
        if file_name.contains(".full-state-") || file_name.contains(".full-state.current") {
            return Err(io::Error::new(
                ErrorKind::InvalidInput,
                format!(
                    "full-state base path uses the reserved snapshot namespace: {}",
                    path.display()
                ),
            ));
        }
    }

    let generation = paths.generation(1)?;
    let groups = [
        (
            "memory",
            full_state_owned_artifacts(&paths.memory, &generation.memory, false),
        ),
        (
            "experience",
            full_state_owned_artifacts(&paths.experience, &generation.experience, false),
        ),
        (
            "adaptive",
            full_state_owned_artifacts(&paths.adaptive, &generation.adaptive, true),
        ),
        (
            "manifest",
            vec![
                paths.manifest.clone(),
                paths.manifest_next.clone(),
                paths.manifest_backup.clone(),
                paths.manifest_retired.clone(),
                paths.writer_lock.clone(),
                append_path_suffix(&paths.manifest_backup, ".next"),
            ],
        ),
    ];
    let mut owners = BTreeMap::new();
    for (owner, artifacts) in groups {
        for artifact in artifacts {
            let identity = path_identity(&artifact)?;
            if let Some(existing) = owners.get(&identity)
                && existing != &owner
            {
                return Err(io::Error::new(
                    ErrorKind::InvalidInput,
                    format!(
                        "full-state path collision between {existing} and {owner}: {}",
                        artifact.display()
                    ),
                ));
            }
            owners.insert(identity, owner);
        }
    }
    Ok(())
}

fn full_state_owned_artifacts(base: &Path, generation: &Path, adaptive: bool) -> Vec<PathBuf> {
    let mut artifacts = disk_kv_artifact_paths(base);
    artifacts.extend(disk_kv_artifact_paths(generation));
    if adaptive {
        for path in [base, generation] {
            for suffix in [".adaptive.next", ".adaptive.bak"] {
                artifacts.extend(disk_kv_artifact_paths(&append_path_suffix(path, suffix)));
            }
        }
    }
    artifacts
}

fn disk_kv_artifact_paths(path: &Path) -> Vec<PathBuf> {
    vec![
        path.to_path_buf(),
        path.with_extension("compact"),
        path.with_extension("compact.bak"),
    ]
}

fn path_identity(path: &Path) -> io::Result<String> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    let absolute = if let Ok(path) = fs::canonicalize(&absolute) {
        path
    } else if let (Some(parent), Some(file_name)) = (absolute.parent(), absolute.file_name()) {
        fs::canonicalize(parent)
            .map(|parent| parent.join(file_name))
            .unwrap_or(absolute)
    } else {
        absolute
    };
    let mut normalized = PathBuf::new();
    for component in absolute.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            _ => normalized.push(component.as_os_str()),
        }
    }
    let identity = normalized.to_string_lossy().into_owned();
    if cfg!(windows) {
        Ok(identity.to_lowercase())
    } else {
        Ok(identity)
    }
}

impl NoironEngine {
    pub fn load_memory(path: impl AsRef<Path>) -> io::Result<Self> {
        Ok(Self::with_cache(KvFusionCache::load_persistent(path)?))
    }

    pub fn load_state(
        memory_path: impl AsRef<Path>,
        experience_path: impl AsRef<Path>,
    ) -> io::Result<Self> {
        let mut engine = Self::load_memory(memory_path)?;
        engine.experience = ExperienceStore::load_from_disk_kv(experience_path)?;
        Ok(engine)
    }

    pub fn load_full_state(
        memory_path: impl AsRef<Path>,
        experience_path: impl AsRef<Path>,
        adaptive_path: impl AsRef<Path>,
    ) -> io::Result<Self> {
        let paths = FullStatePaths::new(
            memory_path.as_ref(),
            experience_path.as_ref(),
            adaptive_path.as_ref(),
        )?;
        load_committed_full_state(&paths)
    }

    pub fn full_state_files_exist(
        memory_path: impl AsRef<Path>,
        experience_path: impl AsRef<Path>,
        adaptive_path: impl AsRef<Path>,
    ) -> io::Result<bool> {
        let paths = FullStatePaths::new(
            memory_path.as_ref(),
            experience_path.as_ref(),
            adaptive_path.as_ref(),
        )?;
        let generation = select_committed_generation(&paths)?;
        if generation == LEGACY_FULL_STATE_GENERATION {
            return Ok(paths.memory.is_file()
                && paths.experience.is_file()
                && paths.adaptive.is_file());
        }
        generation_is_complete(&paths, generation)
    }

    pub fn full_state_read_paths(
        memory_path: impl AsRef<Path>,
        experience_path: impl AsRef<Path>,
        adaptive_path: impl AsRef<Path>,
    ) -> io::Result<(PathBuf, PathBuf, PathBuf)> {
        let paths = FullStatePaths::new(
            memory_path.as_ref(),
            experience_path.as_ref(),
            adaptive_path.as_ref(),
        )?;
        let generation = select_committed_generation(&paths)?;
        let generation_paths = paths.generation(generation)?;
        if generation != LEGACY_FULL_STATE_GENERATION {
            require_generation_files(&generation_paths)?;
        }
        Ok((
            generation_paths.memory,
            generation_paths.experience,
            generation_paths.adaptive,
        ))
    }

    pub fn save_memory(&self, path: impl AsRef<Path>) -> io::Result<()> {
        self.cache.save_persistent(path)
    }

    pub fn save_experience(&self, path: impl AsRef<Path>) -> io::Result<()> {
        self.experience.save_to_disk_kv(path)
    }

    pub fn adaptive_state(&self) -> AdaptiveState {
        AdaptiveState {
            router: self.router.state(),
            hierarchy: self.hierarchy.state(),
            tier_plan: self.last_tier_plan.clone(),
            memory_retention_policy: self.memory_retention_policy,
            memory_compaction_policy: self.memory_compaction_policy.clone(),
            evolution_ledger: self.evolution_ledger,
            genome_runtime: self.genome_runtime_state.clone(),
        }
    }

    pub fn restore_adaptive_state(&mut self, state: AdaptiveState) {
        self.router.restore_state(state.router);
        self.hierarchy.restore_state(state.hierarchy);
        self.last_tier_plan = state.tier_plan;
        self.memory_retention_policy = state.memory_retention_policy;
        self.memory_compaction_policy = state.memory_compaction_policy;
        self.evolution_ledger = state.evolution_ledger;
        self.genome_runtime_state = state.genome_runtime;
    }

    pub fn save_adaptive_state(&self, path: impl AsRef<Path>) -> io::Result<()> {
        self.adaptive_state().save_to_disk_kv(path)
    }

    pub fn save_full_state(
        &mut self,
        memory_path: impl AsRef<Path>,
        experience_path: impl AsRef<Path>,
        adaptive_path: impl AsRef<Path>,
    ) -> io::Result<()> {
        self.save_full_state_with_failure_stage(
            memory_path.as_ref(),
            experience_path.as_ref(),
            adaptive_path.as_ref(),
            None,
        )
    }

    #[cfg(test)]
    pub(super) fn save_full_state_failing_after(
        &mut self,
        memory_path: impl AsRef<Path>,
        experience_path: impl AsRef<Path>,
        adaptive_path: impl AsRef<Path>,
        stage: FullStateSaveStage,
    ) -> io::Result<()> {
        self.save_full_state_with_failure_stage(
            memory_path.as_ref(),
            experience_path.as_ref(),
            adaptive_path.as_ref(),
            Some(stage),
        )
    }

    #[cfg(test)]
    pub(super) fn full_state_manifest_path_for_test(adaptive_path: impl AsRef<Path>) -> PathBuf {
        append_path_suffix(adaptive_path.as_ref(), FULL_STATE_MANIFEST_SUFFIX)
    }

    #[cfg(test)]
    pub(super) fn full_state_generation_path_for_test(
        path: impl AsRef<Path>,
        generation: u64,
    ) -> io::Result<PathBuf> {
        if generation == LEGACY_FULL_STATE_GENERATION {
            Ok(path.as_ref().to_path_buf())
        } else {
            generation_path(path.as_ref(), generation)
        }
    }

    #[cfg(test)]
    pub(super) fn read_full_state_manifest_for_test(
        adaptive_path: impl AsRef<Path>,
    ) -> io::Result<(u64, Option<u64>)> {
        let current_path = append_path_suffix(adaptive_path.as_ref(), FULL_STATE_MANIFEST_SUFFIX);
        let current = read_manifest(&current_path)?.ok_or_else(|| {
            io::Error::new(
                ErrorKind::NotFound,
                format!("full-state manifest is missing: {}", current_path.display()),
            )
        })?;
        let previous = read_manifest(&append_path_suffix(&current_path, ".bak"))?;
        Ok((current, previous))
    }

    fn save_full_state_with_failure_stage(
        &mut self,
        memory_path: &Path,
        experience_path: &Path,
        adaptive_path: &Path,
        failure_stage: Option<FullStateSaveStage>,
    ) -> io::Result<()> {
        let paths = FullStatePaths::new(memory_path, experience_path, adaptive_path)?;
        ensure_parent_directories_durable(&[memory_path, experience_path, adaptive_path])?;
        let _writer_lock = acquire_full_state_writer_lock(&paths)?;
        #[cfg(test)]
        wait_at_full_state_writer_lock_test_barrier()?;
        match save_full_state_generation(self, &paths, failure_stage) {
            Ok(generation) => {
                self.full_state_binding = Some(FullStateBinding::new(&paths, generation)?);
                Ok(())
            }
            Err(save_error) => match load_committed_full_state(&paths) {
                Ok(committed) => {
                    self.restore_committed_state(committed);
                    Err(save_error)
                }
                Err(restore_error) => Err(recovery_error(
                    save_error,
                    "failed to restore the last committed full state",
                    restore_error,
                )),
            },
        }
    }

    fn restore_committed_state(&mut self, committed: Self) {
        self.full_state_binding = committed.full_state_binding.clone();
        let adaptive = committed.adaptive_state();
        self.cache = committed.cache;
        self.experience = committed.experience;
        self.restore_adaptive_state(adaptive);
    }

    pub fn set_hardware_snapshot(&mut self, snapshot: HardwareSnapshot) {
        self.hardware_snapshot = snapshot;
    }

    pub fn set_auto_replay_limit(&mut self, limit: usize) {
        self.auto_replay_limit = limit;
    }

    pub fn set_memory_retention_policy(&mut self, policy: MemoryRetentionPolicy) {
        self.memory_retention_policy = policy;
    }

    pub fn set_memory_compaction_policy(&mut self, policy: MemoryCompactionPolicy) {
        self.memory_compaction_policy = policy;
    }
}

fn acquire_full_state_writer_lock(paths: &FullStatePaths) -> io::Result<File> {
    // ponytail: one persistent lock file serializes one manifest; split only for independent writers.
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(&paths.writer_lock)?;
    match lock.try_lock() {
        Ok(()) => Ok(lock),
        Err(TryLockError::WouldBlock) => Err(io::Error::new(
            ErrorKind::AlreadyExists,
            "full-state writer is busy: lock is held by another process",
        )),
        Err(TryLockError::Error(error)) => Err(error),
    }
}

#[cfg(test)]
fn wait_at_full_state_writer_lock_test_barrier() -> io::Result<()> {
    let Some(ready_path) = std::env::var_os("RUST_NORION_FULL_STATE_LOCK_READY") else {
        return Ok(());
    };
    let release_path = std::env::var_os("RUST_NORION_FULL_STATE_LOCK_RELEASE")
        .map(PathBuf::from)
        .ok_or_else(|| {
            io::Error::new(
                ErrorKind::InvalidInput,
                "full-state writer lock test release path is missing",
            )
        })?;
    File::create(PathBuf::from(ready_path))?.sync_all()?;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    while !release_path.is_file() {
        if std::time::Instant::now() >= deadline {
            return Err(io::Error::new(
                ErrorKind::TimedOut,
                "full-state writer lock test release timed out",
            ));
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    Ok(())
}

fn save_full_state_generation(
    engine: &NoironEngine,
    paths: &FullStatePaths,
    failure_stage: Option<FullStateSaveStage>,
) -> io::Result<u64> {
    let committed_generation = resolve_committed_generation_for_save(paths)?;
    let bound_generation = match &engine.full_state_binding {
        Some(binding) if binding.matches(paths)? => Some(binding.generation),
        _ => None,
    };
    if committed_generation != LEGACY_FULL_STATE_GENERATION
        && bound_generation != Some(committed_generation)
    {
        return Err(io::Error::new(
            ErrorKind::AlreadyExists,
            format!(
                "full-state generation conflict: engine={} current={committed_generation}; load the current full state before saving",
                bound_generation
                    .map(|generation| generation.to_string())
                    .unwrap_or_else(|| "unbound".to_owned())
            ),
        ));
    }
    let next_generation = committed_generation.checked_add(1).ok_or_else(|| {
        io::Error::new(
            ErrorKind::InvalidData,
            "full-state snapshot generation overflow",
        )
    })?;
    let staged = paths.generation(next_generation)?;
    let transaction = (|| {
        remove_generation_artifacts(&staged)?;
        remove_file_if_exists(&paths.manifest_next)?;

        engine.cache.save_to_disk_kv(&staged.memory)?;
        sync_file(&staged.memory)?;
        sync_parent_directories(&[staged.memory.as_path()])?;
        fail_after(failure_stage, FullStateSaveStage::MemoryStaged)?;

        engine.experience.save_to_disk_kv(&staged.experience)?;
        sync_file(&staged.experience)?;
        sync_parent_directories(&[staged.experience.as_path()])?;
        fail_after(failure_stage, FullStateSaveStage::ExperienceStaged)?;

        engine.adaptive_state().save_to_disk_kv(&staged.adaptive)?;
        sync_file(&staged.adaptive)?;
        sync_parent_directories(&[staged.adaptive.as_path()])?;
        fail_after(failure_stage, FullStateSaveStage::AdaptiveStaged)?;

        write_manifest(&paths.manifest_next, next_generation)?;
        fail_after(failure_stage, FullStateSaveStage::ManifestStaged)?;

        seed_legacy_manifest_backup(paths)?;
        publish_manifest(paths, failure_stage)
    })();
    if let Err(error) = transaction {
        let manifest_points_to_staged = matches!(
            read_manifest(&paths.manifest),
            Ok(Some(generation)) if generation == next_generation
        );
        if !manifest_points_to_staged {
            let _ = remove_generation_artifacts(&staged);
            let _ = sync_parent_directories(&[
                staged.memory.as_path(),
                staged.experience.as_path(),
                staged.adaptive.as_path(),
            ]);
        }
        let _ = remove_file_if_exists(&paths.manifest_next);
        let _ = remove_file_if_exists(&append_path_suffix(&paths.manifest_backup, ".next"));
        let _ = sync_parent_directories(&[
            paths.manifest_next.as_path(),
            paths.manifest_backup.as_path(),
        ]);
        return Err(error);
    }

    remove_obsolete_generations(paths, next_generation.saturating_sub(1));
    Ok(next_generation)
}

fn load_committed_full_state(paths: &FullStatePaths) -> io::Result<NoironEngine> {
    let generation = select_committed_generation(paths)?;
    match load_full_state_generation(paths, generation) {
        Ok(engine) => bind_full_state(engine, paths, generation),
        Err(primary_error) => recover_full_state_from_backup(paths, primary_error),
    }
}

fn bind_full_state(
    mut engine: NoironEngine,
    paths: &FullStatePaths,
    generation: u64,
) -> io::Result<NoironEngine> {
    engine.full_state_binding = Some(FullStateBinding::new(paths, generation)?);
    Ok(engine)
}

fn load_full_state_generation(paths: &FullStatePaths, generation: u64) -> io::Result<NoironEngine> {
    let generation_paths = paths.generation(generation)?;
    if generation == LEGACY_FULL_STATE_GENERATION {
        let mut engine =
            NoironEngine::load_state(&generation_paths.memory, &generation_paths.experience)?;
        if let Some(state) = AdaptiveState::load_from_disk_kv(&generation_paths.adaptive)? {
            engine.restore_adaptive_state(state);
        }
        return Ok(engine);
    }

    require_generation_files(&generation_paths)?;
    let cache = KvFusionCache::load_from_disk_kv(&generation_paths.memory)?;
    let experience = ExperienceStore::load_from_disk_kv(&generation_paths.experience)?;
    let adaptive =
        AdaptiveState::load_from_disk_kv(&generation_paths.adaptive)?.ok_or_else(|| {
            io::Error::new(
                ErrorKind::InvalidData,
                format!("full-state generation {generation} has no adaptive snapshot"),
            )
        })?;
    let mut engine = NoironEngine::with_cache(cache);
    engine.experience = experience;
    engine.restore_adaptive_state(adaptive);
    Ok(engine)
}

fn resolve_committed_generation_for_save(paths: &FullStatePaths) -> io::Result<u64> {
    restore_manifest_rotation(paths)?;
    match read_manifest(&paths.manifest) {
        Ok(Some(generation)) if generation_is_complete(paths, generation)? => Ok(generation),
        Ok(Some(_)) => recover_generation_from_backup(
            paths,
            io::Error::new(
                ErrorKind::InvalidData,
                "current full-state manifest references an incomplete generation",
            ),
        ),
        Ok(None) => Ok(LEGACY_FULL_STATE_GENERATION),
        Err(error) => recover_generation_from_backup(paths, error),
    }
}

fn select_committed_generation(paths: &FullStatePaths) -> io::Result<u64> {
    match complete_manifest_generation(paths, &paths.manifest) {
        Ok(Some(generation)) => Ok(generation),
        Ok(None) => select_previous_generation(paths, None)
            .map(|generation| generation.unwrap_or(LEGACY_FULL_STATE_GENERATION)),
        Err(error) => select_previous_generation(paths, Some(error))?.ok_or_else(|| {
            io::Error::new(
                ErrorKind::InvalidData,
                "no complete full-state manifest generation is readable",
            )
        }),
    }
}

fn select_previous_generation(
    paths: &FullStatePaths,
    mut primary_error: Option<io::Error>,
) -> io::Result<Option<u64>> {
    for manifest in [&paths.manifest_backup, &paths.manifest_retired] {
        match complete_manifest_generation(paths, manifest) {
            Ok(Some(generation)) => return Ok(Some(generation)),
            Ok(None) => {}
            Err(error) if primary_error.is_none() => primary_error = Some(error),
            Err(_) => {}
        }
    }
    match primary_error {
        Some(error) => Err(error),
        None => Ok(None),
    }
}

fn complete_manifest_generation(
    paths: &FullStatePaths,
    manifest: &Path,
) -> io::Result<Option<u64>> {
    let Some(generation) = read_manifest(manifest)? else {
        return Ok(None);
    };
    if generation_is_complete(paths, generation)? {
        Ok(Some(generation))
    } else {
        Err(io::Error::new(
            ErrorKind::InvalidData,
            format!(
                "full-state manifest references an incomplete generation: {}",
                manifest.display()
            ),
        ))
    }
}

fn recover_generation_from_backup(
    paths: &FullStatePaths,
    primary_error: io::Error,
) -> io::Result<u64> {
    let backup_generation = match read_manifest(&paths.manifest_backup) {
        Ok(Some(generation)) => generation,
        Ok(None) => return Err(primary_error),
        Err(backup_error) => {
            return Err(recovery_error(
                primary_error,
                "backup manifest could not be read",
                backup_error,
            ));
        }
    };
    if !generation_is_complete(paths, backup_generation)? {
        return Err(io::Error::new(
            primary_error.kind(),
            format!("{primary_error}; backup manifest references an incomplete generation"),
        ));
    }
    repair_manifest_from_backup(paths).map_err(|repair_error| {
        recovery_error(
            primary_error,
            "backup generation is complete but its manifest could not be restored",
            repair_error,
        )
    })?;
    Ok(backup_generation)
}

fn recover_full_state_from_backup(
    paths: &FullStatePaths,
    primary_error: io::Error,
) -> io::Result<NoironEngine> {
    let backup_generation = match read_manifest(&paths.manifest_backup) {
        Ok(Some(generation)) => generation,
        Ok(None) => return Err(primary_error),
        Err(backup_error) => {
            return Err(recovery_error(
                primary_error,
                "backup manifest could not be read",
                backup_error,
            ));
        }
    };
    let engine = load_full_state_generation(paths, backup_generation).map_err(|backup_error| {
        recovery_error(
            primary_error,
            "backup full-state generation could not be loaded",
            backup_error,
        )
    })?;
    bind_full_state(engine, paths, backup_generation)
}

fn require_generation_files(paths: &FullStateGenerationPaths) -> io::Result<()> {
    for (kind, path) in [
        ("memory", &paths.memory),
        ("experience", &paths.experience),
        ("adaptive", &paths.adaptive),
    ] {
        if !path.is_file() {
            return Err(io::Error::new(
                ErrorKind::InvalidData,
                format!(
                    "full-state {kind} generation file is missing: {}",
                    path.display()
                ),
            ));
        }
    }
    Ok(())
}

fn generation_is_complete(paths: &FullStatePaths, generation: u64) -> io::Result<bool> {
    if generation == LEGACY_FULL_STATE_GENERATION {
        return Ok(true);
    }
    let generation_paths = paths.generation(generation)?;
    Ok(generation_paths.memory.is_file()
        && generation_paths.experience.is_file()
        && generation_paths.adaptive.is_file())
}

fn seed_legacy_manifest_backup(paths: &FullStatePaths) -> io::Result<()> {
    if paths.manifest.exists() || paths.manifest_backup.exists() {
        return Ok(());
    }
    let staged_backup = append_path_suffix(&paths.manifest_backup, ".next");
    remove_file_if_exists(&staged_backup)?;
    write_manifest(&staged_backup, LEGACY_FULL_STATE_GENERATION)?;
    rename_durable(&staged_backup, &paths.manifest_backup)
}

fn publish_manifest(
    paths: &FullStatePaths,
    failure_stage: Option<FullStateSaveStage>,
) -> io::Result<()> {
    let publish = (|| {
        if paths.manifest.exists() {
            remove_file_if_exists(&paths.manifest_retired)?;
            if paths.manifest_backup.exists() {
                rename_durable(&paths.manifest_backup, &paths.manifest_retired)?;
            }
            rename_durable(&paths.manifest, &paths.manifest_backup)?;
        }
        fail_after(failure_stage, FullStateSaveStage::CurrentBackedUp)?;
        fs::rename(&paths.manifest_next, &paths.manifest)?;
        fail_after(failure_stage, FullStateSaveStage::ManifestPublished)?;
        sync_parent_directories(&[paths.manifest_next.as_path(), paths.manifest.as_path()])
    })();
    if let Err(publish_error) = publish {
        if let Err(restore_error) = restore_manifest_rotation(paths) {
            return Err(recovery_error(
                publish_error,
                "new manifest publication failed and the previous manifest rotation could not be restored",
                restore_error,
            ));
        }
        return Err(publish_error);
    }
    if remove_file_if_exists(&paths.manifest_retired).is_ok() {
        let _ = sync_parent_directories(&[paths.manifest_retired.as_path()]);
    }
    Ok(())
}

fn restore_manifest_rotation(paths: &FullStatePaths) -> io::Result<()> {
    if paths.manifest.exists() {
        if !paths.manifest_backup.exists() && paths.manifest_retired.exists() {
            rename_durable(&paths.manifest_retired, &paths.manifest_backup)?;
        } else if paths.manifest_backup.exists() {
            remove_file_if_exists(&paths.manifest_retired)?;
            sync_parent_directories(&[paths.manifest_retired.as_path()])?;
        }
        return Ok(());
    }
    if paths.manifest_backup.exists() {
        rename_durable(&paths.manifest_backup, &paths.manifest)?;
        if paths.manifest_retired.exists() {
            rename_durable(&paths.manifest_retired, &paths.manifest_backup)?;
        }
    } else if paths.manifest_retired.exists() {
        rename_durable(&paths.manifest_retired, &paths.manifest)?;
    }
    Ok(())
}

fn repair_manifest_from_backup(paths: &FullStatePaths) -> io::Result<()> {
    remove_file_if_exists(&paths.manifest)?;
    rename_durable(&paths.manifest_backup, &paths.manifest)
}

fn write_manifest(path: &Path, generation: u64) -> io::Result<()> {
    ensure_parent_directories_durable(&[path])?;
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(path)?;
    writeln!(file, "{FULL_STATE_MANIFEST_HEADER}")?;
    writeln!(file, "generation={generation}")?;
    file.sync_all()?;
    sync_parent_directories(&[path])
}

fn read_manifest(path: &Path) -> io::Result<Option<u64>> {
    let value = match fs::read_to_string(path) {
        Ok(value) => value,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    let mut lines = value.lines();
    if lines.next() != Some(FULL_STATE_MANIFEST_HEADER) {
        return Err(io::Error::new(
            ErrorKind::InvalidData,
            format!("invalid full-state manifest header: {}", path.display()),
        ));
    }
    let generation = lines
        .next()
        .and_then(|line| line.strip_prefix("generation="))
        .and_then(|value| value.parse::<u64>().ok())
        .ok_or_else(|| {
            io::Error::new(
                ErrorKind::InvalidData,
                format!("invalid full-state manifest generation: {}", path.display()),
            )
        })?;
    if lines.any(|line| !line.trim().is_empty()) {
        return Err(io::Error::new(
            ErrorKind::InvalidData,
            format!("unexpected full-state manifest data: {}", path.display()),
        ));
    }
    Ok(Some(generation))
}

fn sync_file(path: &Path) -> io::Result<()> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)?
        .sync_all()
}

fn rename_durable(from: &Path, to: &Path) -> io::Result<()> {
    fs::rename(from, to)?;
    sync_parent_directories(&[from, to])
}

fn ensure_parent_directories_durable(paths: &[&Path]) -> io::Result<()> {
    let mut ensured = Vec::new();
    for path in paths {
        let parent = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        let identity = path_identity(parent)?;
        if ensured.contains(&identity) {
            continue;
        }
        create_directory_all_durable(parent)?;
        ensured.push(path_identity(parent)?);
    }
    Ok(())
}

fn create_directory_all_durable(path: &Path) -> io::Result<()> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    if absolute.is_dir() {
        return if absolute.parent().is_some() {
            sync_parent_directories(&[absolute.as_path()])
        } else {
            Ok(())
        };
    }

    let mut missing = Vec::new();
    let mut current = absolute.as_path();
    while !current.is_dir() {
        missing.push(current.to_path_buf());
        current = current.parent().ok_or_else(|| {
            io::Error::new(
                ErrorKind::NotFound,
                format!(
                    "full-state directory has no existing ancestor: {}",
                    path.display()
                ),
            )
        })?;
    }
    for directory in missing.iter().rev() {
        match fs::create_dir(directory) {
            Ok(()) => {}
            Err(error) if error.kind() == ErrorKind::AlreadyExists && directory.is_dir() => {}
            Err(error) => return Err(error),
        }
        sync_parent_directories(&[directory.as_path()])?;
    }
    Ok(())
}

fn sync_parent_directories(paths: &[&Path]) -> io::Result<()> {
    let mut synced = Vec::new();
    for path in paths {
        let parent = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        let identity = path_identity(parent)?;
        if synced.contains(&identity) {
            continue;
        }
        sync_directory(parent)?;
        synced.push(identity);
    }
    Ok(())
}

#[cfg(not(windows))]
fn sync_directory(path: &Path) -> io::Result<()> {
    File::open(path)?.sync_all()
}

#[cfg(windows)]
fn sync_directory(path: &Path) -> io::Result<()> {
    const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;

    let directory = OpenOptions::new()
        .write(true)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
        .open(path)?;
    let mut io_status = WindowsIoStatusBlock {
        status_or_pointer: std::ptr::null_mut(),
        information: 0,
    };
    // SAFETY: the owned directory handle remains alive for the call; flags=0
    // requires no parameter buffer; io_status is a valid aligned out-parameter.
    let status = unsafe {
        nt_flush_buffers_file_ex(
            directory.as_raw_handle(),
            0,
            std::ptr::null_mut(),
            0,
            &mut io_status,
        )
    };
    if status >= 0 {
        return Ok(());
    }
    // SAFETY: RtlNtStatusToDosError accepts every NTSTATUS value by value.
    let error = unsafe { rtl_nt_status_to_dos_error(status) };
    Err(io::Error::from_raw_os_error(error as i32))
}

#[cfg(windows)]
#[repr(C)]
struct WindowsIoStatusBlock {
    status_or_pointer: *mut c_void,
    information: usize,
}

#[cfg(windows)]
#[link(name = "ntdll")]
unsafe extern "system" {
    #[link_name = "NtFlushBuffersFileEx"]
    fn nt_flush_buffers_file_ex(
        file_handle: std::os::windows::io::RawHandle,
        flags: u32,
        parameters: *mut c_void,
        parameters_size: u32,
        io_status_block: *mut WindowsIoStatusBlock,
    ) -> i32;

    #[link_name = "RtlNtStatusToDosError"]
    fn rtl_nt_status_to_dos_error(status: i32) -> u32;
}

fn remove_generation_artifacts(paths: &FullStateGenerationPaths) -> io::Result<()> {
    remove_disk_kv_artifacts(&paths.memory)?;
    remove_disk_kv_artifacts(&paths.experience)?;
    remove_disk_kv_artifacts(&paths.adaptive)?;

    for suffix in [".adaptive.next", ".adaptive.bak"] {
        let sidecar = append_path_suffix(&paths.adaptive, suffix);
        remove_disk_kv_artifacts(&sidecar)?;
    }
    Ok(())
}

fn remove_obsolete_generations(paths: &FullStatePaths, oldest_to_keep: u64) {
    if oldest_to_keep > LEGACY_FULL_STATE_GENERATION
        && let Ok(legacy) = paths.generation(LEGACY_FULL_STATE_GENERATION)
    {
        let _ = remove_generation_artifacts(&legacy);
    }

    let mut generations = Vec::new();
    for base in [&paths.memory, &paths.experience, &paths.adaptive] {
        let Some(parent) = base.parent() else {
            continue;
        };
        let Ok(entries) = fs::read_dir(parent) else {
            continue;
        };
        for entry in entries.flatten() {
            let Some(generation) = generation_from_file_name(base, &entry.file_name()) else {
                continue;
            };
            if generation < oldest_to_keep && !generations.contains(&generation) {
                generations.push(generation);
            }
        }
    }
    for generation in generations {
        if let Ok(obsolete) = paths.generation(generation) {
            let _ = remove_generation_artifacts(&obsolete);
        }
    }
    let _ = sync_parent_directories(&[
        paths.memory.as_path(),
        paths.experience.as_path(),
        paths.adaptive.as_path(),
    ]);
}

fn generation_from_file_name(base: &Path, file_name: &std::ffi::OsStr) -> Option<u64> {
    let stem = base.file_stem()?.to_string_lossy();
    let prefix = format!("{stem}.full-state-");
    file_name
        .to_string_lossy()
        .strip_prefix(&prefix)?
        .split('.')
        .next()?
        .parse()
        .ok()
}

fn remove_disk_kv_artifacts(path: &Path) -> io::Result<()> {
    remove_file_if_exists(path)?;
    remove_file_if_exists(&path.with_extension("compact"))?;
    remove_file_if_exists(&path.with_extension("compact.bak"))
}

fn remove_file_if_exists(path: &Path) -> io::Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn generation_path(path: &Path, generation: u64) -> io::Result<PathBuf> {
    let file_name = path.file_name().ok_or_else(|| {
        io::Error::new(
            ErrorKind::InvalidInput,
            format!("full-state path has no file name: {}", path.display()),
        )
    })?;
    let stem = path
        .file_stem()
        .filter(|stem| !stem.is_empty())
        .unwrap_or(file_name);
    let mut generation_name = OsString::from(stem);
    generation_name.push(format!(".full-state-{generation}"));
    if let Some(extension) = path.extension()
        && !extension.is_empty()
    {
        generation_name.push(".");
        generation_name.push(extension);
    } else {
        generation_name.push(".ndkv");
    }
    Ok(path.with_file_name(generation_name))
}

fn append_path_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut value = path.as_os_str().to_os_string();
    value.push(suffix);
    PathBuf::from(value)
}

fn fail_after(
    failure_stage: Option<FullStateSaveStage>,
    completed_stage: FullStateSaveStage,
) -> io::Result<()> {
    if failure_stage == Some(completed_stage) {
        return Err(io::Error::other(format!(
            "injected full-state save failure after {completed_stage:?}"
        )));
    }
    Ok(())
}

fn recovery_error(primary: io::Error, context: &str, recovery: io::Error) -> io::Error {
    io::Error::new(primary.kind(), format!("{primary}; {context}: {recovery}"))
}
