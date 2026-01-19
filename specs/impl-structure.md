# Hierarchical Implementation Plans Specification

**Status:** Planned
**Version:** 1.0
**Last Updated:** 2026-01-19

---

## 1. Overview

### Purpose

Hierarchical implementation plans reduce context consumption during the build loop by organizing tasks into feature-aligned files. Instead of reading a single monolithic `IMPLEMENTATION_PLAN.md` each iteration, the agent reads a small index (`impl/README.md`) to identify the current focus, then reads only the relevant feature file.

### Goals

- **Context efficiency** - Reduce per-iteration context from ~300 lines to ~50-80 lines
- **Spec alignment** - Mirror the `specs/` structure with `impl/{feature}.md` files
- **Clean archival** - Move completed features out of active context automatically
- **Backward compatibility** - Support gradual migration from single-file plans

### Non-Goals

- **Per-task files** - Too granular; overhead exceeds benefit
- **Automatic spec generation** - Planning mode creates plans, not specs
- **Cross-project plan sharing** - Plans are project-specific

---

## 2. Architecture

### Directory Structure

```
impl/
├── README.md              # Index: status table, current focus, cross-cutting tasks
├── {feature-a}.md         # Tasks for feature A (aligns with specs/feature-a.md)
├── {feature-b}.md         # Tasks for feature B
└── .archive/              # Completed feature files (gitignored optional)
    ├── {feature-c}.md
    └── {feature-d}.md
```

### Spec-to-Impl Mapping

```
specs/authentication.md    →    impl/authentication.md
specs/attachments.md       →    impl/attachments.md
specs/permissions.md       →    impl/permissions.md
```

### Data Flow

```
┌─────────────────────────────────────────────────────────────────┐
│                      Planning Mode                               │
├─────────────────────────────────────────────────────────────────┤
│  1. Read specs/*.md                                              │
│  2. Analyze codebase for gaps                                    │
│  3. Create impl/README.md with status table                      │
│  4. Create impl/{feature}.md for each feature with gaps          │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Building Mode                               │
├─────────────────────────────────────────────────────────────────┤
│  1. Read impl/README.md (~30 lines)                              │
│  2. Identify current focus from "Current Focus" section          │
│  3. Read impl/{current-feature}.md (~30-50 lines)                │
│  4. Implement highest priority incomplete task                   │
│  5. Update task status in feature file                           │
│  6. If feature complete → move to .archive/, update README       │
└─────────────────────────────────────────────────────────────────┘
```

---

## 3. Core Types

### 3.1 ImplIndex (impl/README.md)

The index file provides a high-level overview and directs the agent to the active feature.

```rust
pub struct ImplIndex {
    pub generated: DateTime<Utc>,
    pub based_on: String,           // "specs/*.md"
    pub project: String,
    pub features: Vec<FeatureStatus>,
    pub current_focus: Option<String>,  // Path to active feature file
    pub cross_cutting_tasks: Vec<Task>,
}

pub struct FeatureStatus {
    pub name: String,               // e.g., "authentication"
    pub file: String,               // e.g., "impl/authentication.md"
    pub status: FeatureState,
    pub total_tasks: u32,
    pub completed_tasks: u32,
    pub spec_ref: Option<String>,   // e.g., "specs/authentication.md"
}

pub enum FeatureState {
    Pending,      // No tasks started
    InProgress,   // Some tasks complete
    Complete,     // All tasks complete
    Archived,     // Moved to .archive/
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `generated` | DateTime | Yes | When the plan was created |
| `based_on` | String | Yes | Source specs reference |
| `project` | String | Yes | Project name |
| `features` | Vec | Yes | Status of each feature |
| `current_focus` | Option | No | Path to active feature file |
| `cross_cutting_tasks` | Vec | No | Tasks not tied to a specific feature |

### 3.2 FeatureFile (impl/{feature}.md)

Each feature file contains tasks for a single spec-aligned feature.

```rust
pub struct FeatureFile {
    pub name: String,
    pub spec_ref: String,
    pub status: FeatureState,
    pub last_updated: DateTime<Utc>,
    pub tasks: Vec<Task>,
    pub dependencies: Vec<String>,  // Other features this depends on
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | String | Yes | Feature name matching spec |
| `spec_ref` | String | Yes | Path to corresponding spec |
| `status` | FeatureState | Yes | Current feature status |
| `last_updated` | DateTime | Yes | Last modification time |
| `tasks` | Vec | Yes | Tasks for this feature |
| `dependencies` | Vec | No | Feature dependencies |

### 3.3 Task (unchanged from current)

```rust
pub struct Task {
    pub description: String,
    pub status: TaskStatus,
    pub spec_refs: Vec<String>,
    pub line_number: usize,
    pub priority: Option<u32>,
    pub dependencies: Vec<String>,
    pub complexity: Option<String>,
}

pub enum TaskStatus {
    Pending,      // [ ]
    Completed,    // [x] or [X]
    InProgress,   // [~]
}
```

---

## 4. File Formats

### 4.1 impl/README.md Format

```markdown
# Implementation Plan

**Generated:** 2026-01-19
**Based on:** specs/*.md
**Project:** {project_name}

---

## Status Overview

| Feature | Status | Progress | Spec |
|---------|--------|----------|------|
| [authentication](./authentication.md) | ✅ Complete | 5/5 | [spec](../specs/authentication.md) |
| [attachments](./attachments.md) | 🔄 In Progress | 2/4 | [spec](../specs/attachments.md) |
| [permissions](./permissions.md) | ⏳ Pending | 0/5 | [spec](../specs/permissions.md) |

---

## Current Focus

**Active:** [attachments.md](./attachments.md)

Next task: P2.3 - Implement `download_attachment` tool

---

## Cross-Cutting Tasks

Tasks not tied to a specific feature:

- [ ] P4.1 - Create top-level tools barrel export
- [ ] P4.2 - Update specs/README.md status

---

## Archived Features

Completed features moved to `.archive/`:

- [authentication](./.archive/authentication.md) - Completed 2026-01-18
```

### 4.2 impl/{feature}.md Format

```markdown
# {Feature Name} Implementation

**Spec:** [specs/{feature}.md](../specs/{feature}.md)
**Status:** In Progress
**Last Updated:** 2026-01-19

---

## Dependencies

- ✅ authentication (complete)
- ⏳ None blocking

---

## Tasks

### Priority 2: Core Features

#### P2.1: Add KeepClient Attachment Methods ✅

- [x] Extend KeepClient with attachment-related API methods
  - **File:** `src/clients/keep-client.ts`
  - **Complexity:** Medium

#### P2.2: Implement `list_attachments` Tool ✅

- [x] Create tool to enumerate attachments on a note
  - **File:** `src/tools/attachments/list.ts`
  - **Complexity:** Low

#### P2.3: Implement `download_attachment` Tool

- [ ] Create tool to download attachment content as base64
  - **Refs:** specs/attachments.md
  - **File:** `src/tools/attachments/download.ts`
  - **Dependencies:** P2.1
  - **Complexity:** Medium

#### P2.4: Create Attachments Barrel Export

- [ ] Create `src/tools/attachments/index.ts`
  - **Dependencies:** P2.2, P2.3
  - **Complexity:** Low
```

---

## 5. Behaviors

### 5.1 Planning Mode Changes

Planning mode creates hierarchical structure instead of single file:

```rust
// In src/commands/plan.rs

fn create_hierarchical_plan(specs: &[Spec], gaps: &[Gap]) -> Result<()> {
    // 1. Group gaps by feature (aligned with specs)
    let features = group_by_spec(gaps);

    // 2. Create impl/ directory
    fs::create_dir_all("impl")?;
    fs::create_dir_all("impl/.archive")?;

    // 3. Create feature files
    for (spec_name, tasks) in &features {
        let content = render_feature_file(spec_name, tasks);
        fs::write(format!("impl/{}.md", spec_name), content)?;
    }

    // 4. Create README.md index
    let readme = render_impl_readme(&features);
    fs::write("impl/README.md", readme)?;

    Ok(())
}
```

### 5.2 Building Mode Changes

Building mode reads hierarchically:

```rust
// In src/commands/build.rs

fn get_current_task() -> Result<(PathBuf, Task)> {
    // 1. Read impl/README.md
    let readme = fs::read_to_string("impl/README.md")?;

    // 2. Extract current focus
    let current_focus = parse_current_focus(&readme)?;

    // 3. Read the active feature file
    let feature_content = fs::read_to_string(&current_focus)?;

    // 4. Find first pending task
    let tasks = parse_feature_tasks(&feature_content)?;
    let next_task = tasks.iter().find(|t| t.status == TaskStatus::Pending);

    Ok((current_focus, next_task))
}
```

### 5.3 Task Completion Flow

When a task is completed:

```rust
fn complete_task(feature_path: &Path, task: &Task) -> Result<()> {
    // 1. Update task status in feature file
    update_task_status(feature_path, task, TaskStatus::Completed)?;

    // 2. Check if feature is complete
    let tasks = parse_feature_tasks(&fs::read_to_string(feature_path)?)?;
    let all_complete = tasks.iter().all(|t| t.status == TaskStatus::Completed);

    if all_complete {
        // 3. Archive the feature file
        let archive_path = feature_path
            .parent().unwrap()
            .join(".archive")
            .join(feature_path.file_name().unwrap());
        fs::rename(feature_path, archive_path)?;

        // 4. Update README.md status and current focus
        update_readme_status()?;
        select_next_focus()?;
    }

    Ok(())
}
```

### 5.4 Pending Task Detection

Updated `has_pending_tasks()` to check hierarchical structure:

```rust
pub fn has_pending_tasks(impl_dir: &Path) -> bool {
    let readme_path = impl_dir.join("README.md");

    // Check if impl/ structure exists
    if !readme_path.exists() {
        // Fall back to legacy single-file check
        return has_pending_tasks_legacy(Path::new("IMPLEMENTATION_PLAN.md"));
    }

    // Check each non-archived feature file for pending tasks
    for entry in fs::read_dir(impl_dir).unwrap_or_else(|_| panic!()) {
        let entry = entry.unwrap();
        let path = entry.path();

        // Skip README, .archive, and non-markdown files
        if path.file_name() == Some("README.md".as_ref())
           || path.file_name() == Some(".archive".as_ref())
           || path.extension() != Some("md".as_ref()) {
            continue;
        }

        let content = fs::read_to_string(&path).unwrap_or_default();
        let checkbox_re = Regex::new(r"^\s*-\s*\[\s\]").unwrap();
        if content.lines().any(|line| checkbox_re.is_match(line)) {
            return true;
        }
    }

    // Also check cross-cutting tasks in README
    let readme_content = fs::read_to_string(&readme_path).unwrap_or_default();
    let checkbox_re = Regex::new(r"^\s*-\s*\[\s\]").unwrap();
    readme_content.lines().any(|line| checkbox_re.is_match(line))
}
```

### 5.5 Current Focus Selection

Automatic selection of next feature to work on:

```rust
fn select_next_focus(impl_dir: &Path) -> Result<Option<PathBuf>> {
    // Priority order:
    // 1. Features with in-progress tasks
    // 2. Features with pending tasks (lowest task count first for quick wins)
    // 3. Cross-cutting tasks in README

    let mut candidates: Vec<(PathBuf, u32, u32)> = vec![]; // (path, pending, total)

    for entry in fs::read_dir(impl_dir)? {
        let path = entry?.path();
        if !is_feature_file(&path) { continue; }

        let content = fs::read_to_string(&path)?;
        let (pending, total) = count_tasks(&content);

        if pending > 0 {
            candidates.push((path, pending, total));
        }
    }

    // Sort by: has in-progress first, then by pending count ascending
    candidates.sort_by(|a, b| a.1.cmp(&b.1));

    Ok(candidates.first().map(|(p, _, _)| p.clone()))
}
```

---

## 6. Configuration

New configuration options in `.fresher/config.toml`:

| Variable | Type | Description | Default |
|----------|------|-------------|---------|
| `paths.impl_dir` | string | Implementation plan directory | `impl` |
| `fresher.archive_completed` | boolean | Auto-archive completed features | `true` |
| `fresher.single_file_threshold` | number | Task count below which single file is used | `8` |

```toml
[paths]
impl_dir = "impl"

[fresher]
archive_completed = true
single_file_threshold = 8
```

---

## 7. Migration

### 7.1 Automatic Migration

On first `fresher build` after upgrade, detect and migrate:

```rust
fn maybe_migrate_to_hierarchical() -> Result<()> {
    let legacy_path = Path::new("IMPLEMENTATION_PLAN.md");
    let impl_dir = Path::new("impl");

    // Skip if already hierarchical or no legacy plan
    if impl_dir.exists() || !legacy_path.exists() {
        return Ok(());
    }

    // Parse legacy plan
    let tasks = parse_plan(legacy_path)?;

    // Check threshold
    if tasks.len() < config.single_file_threshold {
        return Ok(()); // Keep single file for small plans
    }

    // Group tasks by spec reference
    let by_feature = group_tasks_by_spec(&tasks);

    // Create hierarchical structure
    create_hierarchical_plan(&by_feature)?;

    // Rename legacy file (don't delete)
    fs::rename(legacy_path, "IMPLEMENTATION_PLAN.md.backup")?;

    println!("Migrated to hierarchical plan structure: impl/");
    Ok(())
}
```

### 7.2 Manual Migration

Users can also run explicit migration:

```bash
fresher migrate-plan
```

### 7.3 Fallback

If `impl/` doesn't exist but `IMPLEMENTATION_PLAN.md` does, use legacy single-file mode.

---

## 8. Prompt Template Updates

### 8.1 Planning Mode Prompt

Update `templates.rs::PROMPT_PLANNING`:

```markdown
## Step 4: Create Hierarchical Plan

Create `impl/` directory structure:

1. **impl/README.md** - Index with:
   - Status table (feature, status, progress, spec link)
   - Current Focus section pointing to active feature
   - Cross-cutting tasks section

2. **impl/{feature}.md** for each feature with gaps:
   - Feature name matching spec name
   - Link to corresponding spec
   - Tasks organized by priority
   - Each task with: checkbox, description, file, complexity

Example structure:
```
impl/
├── README.md
├── authentication.md
├── attachments.md
└── permissions.md
```
```

### 8.2 Building Mode Prompt

Update `templates.rs::PROMPT_BUILDING`:

```markdown
## Your Task

1. **Read** `impl/README.md` to see status overview
2. **Identify** current focus from "Current Focus" section
3. **Read** the active feature file (e.g., `impl/attachments.md`)
4. **Select** the highest priority incomplete task
5. **Implement** the task completely
6. **Validate** with tests and builds
7. **Update** task status in feature file (change `- [ ]` to `- [x]`)
8. **Commit** changes

### Step 1: Read Plan Index

Open `impl/README.md` and note:
- Which feature is marked as "Current Focus"
- Overall progress across features

### Step 2: Read Active Feature

Open the current focus file (e.g., `impl/attachments.md`) and find the first `- [ ]` task.

### After Completion

When you complete a task:
1. Mark it `- [x]` in the feature file
2. If all tasks in feature are complete:
   - Update feature status in `impl/README.md` to ✅ Complete
   - The system will archive it automatically
```

---

## 9. Verification Updates

### 9.1 fresher verify

Update to handle hierarchical plans:

```rust
pub fn generate_report(impl_dir: &Path, spec_dir: &Path) -> Result<VerifyReport> {
    let readme_path = impl_dir.join("README.md");

    if !readme_path.exists() {
        // Fall back to legacy
        return generate_report_legacy(Path::new("IMPLEMENTATION_PLAN.md"), spec_dir);
    }

    // Aggregate tasks from all feature files
    let mut all_tasks = Vec::new();

    for entry in fs::read_dir(impl_dir)? {
        let path = entry?.path();
        if is_feature_file(&path) {
            let tasks = parse_feature_file(&path)?;
            all_tasks.extend(tasks);
        }
    }

    // Also include cross-cutting tasks from README
    let readme_tasks = parse_cross_cutting_tasks(&readme_path)?;
    all_tasks.extend(readme_tasks);

    // Generate report as before
    generate_report_from_tasks(&all_tasks, spec_dir)
}
```

### 9.2 Terminal Output

```
Implementation Plan Verification (Hierarchical)
================================================

Feature Summary
  authentication     [████████████████████] 100% (5/5) ✅
  attachments        [████████████░░░░░░░░] 50% (2/4) 🔄
  permissions        [░░░░░░░░░░░░░░░░░░░░] 0% (0/5) ⏳

Current Focus: attachments
  Next: P2.3 - Implement download_attachment tool

Task Summary
  Total tasks:     14
  Completed:       7 (50%)
  In Progress:     1
  Pending:         6

→ 6 tasks remaining across 2 features
```

---

## 10. Implementation Phases

| Phase | Description | Dependencies | Complexity |
|-------|-------------|--------------|------------|
| 1 | Update `has_pending_tasks()` to check impl/ | None | Low |
| 2 | Update planning mode to create hierarchical structure | None | Medium |
| 3 | Update building mode prompts and focus selection | Phase 2 | Medium |
| 4 | Implement auto-archival on feature completion | Phase 3 | Low |
| 5 | Update `fresher verify` for hierarchical plans | Phase 2 | Medium |
| 6 | Add migration command and auto-migration | Phase 2 | Medium |
| 7 | Update configuration options | None | Low |

---

## 11. Context Efficiency Analysis

### Before (Single File)

```
Agent reads: IMPLEMENTATION_PLAN.md (~350 lines)
├── Executive Summary (~30 lines)
├── Gap Analysis Table (~20 lines)
├── What's Implemented (~30 lines)
├── Priority 1 Tasks (~50 lines) - 83% complete
├── Priority 2 Tasks (~80 lines) - 100% complete ← noise
├── Priority 3 Tasks (~80 lines) - 100% complete ← noise
├── Priority 4 Tasks (~30 lines) - 50% complete
├── Implementation Order (~20 lines)
└── Dependency Graph (~30 lines)

Total context: ~350 lines
Useful context: ~80 lines (23%)
```

### After (Hierarchical)

```
Agent reads: impl/README.md (~40 lines)
├── Status Overview Table (~10 lines)
├── Current Focus (~5 lines)
├── Cross-cutting Tasks (~10 lines)
└── Archived Features (~5 lines)

Agent reads: impl/attachments.md (~40 lines)
├── Dependencies (~5 lines)
├── Completed Tasks (~10 lines) - collapsed
└── Pending Tasks (~25 lines)

Total context: ~80 lines
Useful context: ~60 lines (75%)
```

**Result:** ~77% reduction in context, ~3x improvement in signal-to-noise ratio.

---

## 12. Open Questions

- [ ] Should archived features be gitignored or committed for history?
- [ ] How to handle features that span multiple specs?
- [ ] Should there be a `fresher plan --single-file` escape hatch?
