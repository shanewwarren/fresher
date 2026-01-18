# Project Integration Specification

**Status:** Planned
**Version:** 1.0
**Last Updated:** 2026-01-17

---

## 1. Overview

### Purpose

Project integration handles how Fresher coexists cleanly with any project. This includes automatic .gitignore configuration, project type detection, and sensible defaults that keep Fresher's runtime artifacts out of version control while allowing the installation itself to remain flexible.

### Goals

- **Clean git hygiene** - Runtime files (.state, logs/) are always gitignored
- **Zero manual setup** - .gitignore is configured automatically on install
- **Project detection** - Auto-detect test/build commands based on project type
- **Non-intrusive** - Fresher presence doesn't pollute project structure

### Non-Goals

- **Gitignore the .fresher/ directory itself** - That's a user choice, not a default
- **Modify existing project structure** - Only add .fresher/ and update .gitignore
- **Force conventions** - Detected values are defaults, always overridable

---

## 2. Architecture

### Gitignore Strategy

The installer automatically adds these entries to .gitignore:

```gitignore
# Fresher runtime artifacts
.fresher/logs/
.fresher/.state
```

**Note:** The `.fresher/` directory itself is NOT gitignored by default. This allows teams to choose whether to:
- Commit `.fresher/` for shared team configuration
- Add `.fresher/` to .gitignore manually for personal tooling

### Project Type Detection

```
┌─────────────────┐
│  Check for      │
│  package.json   │──▶ Node.js
└─────────────────┘
         │ no
         ▼
┌─────────────────┐
│  Check for      │
│  Cargo.toml     │──▶ Rust
└─────────────────┘
         │ no
         ▼
┌─────────────────┐
│  Check for      │
│  go.mod         │──▶ Go
└─────────────────┘
         │ no
         ▼
┌─────────────────┐
│  Check for      │
│  pyproject.toml │──▶ Python
└─────────────────┘
         │ no
         ▼
┌─────────────────┐
│  Check for      │
│  Makefile       │──▶ Make
└─────────────────┘
         │ no
         ▼
      Generic
```

---

## 3. Core Types

### 3.1 Project Type Mappings

| Marker File | Type | Test Command | Build Command | Src Dir |
|-------------|------|--------------|---------------|---------|
| package.json | nodejs | `npm test` | `npm run build` | `src` |
| Cargo.toml | rust | `cargo test` | `cargo build` | `src` |
| go.mod | go | `go test ./...` | `go build ./...` | `.` |
| pyproject.toml | python | `pytest` | `python -m build` | `src` |
| requirements.txt | python | `pytest` | `:` (no-op) | `.` |
| Makefile | make | `make test` | `make` | `.` |
| *.csproj | dotnet | `dotnet test` | `dotnet build` | `.` |
| pom.xml | maven | `mvn test` | `mvn package` | `src/main` |
| build.gradle | gradle | `./gradlew test` | `./gradlew build` | `src/main` |
| (none) | generic | `echo "no tests"` | `echo "no build"` | `.` |

### 3.2 Gitignore Entry Format

```gitignore
# Fresher - AI development loop runtime artifacts
# Added by fresher install - do not remove unless uninstalling
.fresher/logs/
.fresher/.state
```

---

## 4. API / Behaviors

### 4.1 Gitignore Update

**Purpose:** Add Fresher entries to .gitignore

**Behavior:**

1. Check if .gitignore exists
   - If no: create it with Fresher entries
   - If yes: check for existing Fresher entries
2. If entries missing: append Fresher block
3. If entries exist: skip (idempotent)

**Implementation:**

```bash
update_gitignore() {
    local gitignore=".gitignore"
    local marker="# Fresher - AI development loop runtime artifacts"

    # Create if doesn't exist
    touch "$gitignore"

    # Check if already configured
    if grep -q "\.fresher/logs/" "$gitignore" 2>/dev/null; then
        echo "Gitignore already configured"
        return 0
    fi

    # Append entries
    cat >> "$gitignore" << 'EOF'

# Fresher - AI development loop runtime artifacts
# Added by fresher install - do not remove unless uninstalling
.fresher/logs/
.fresher/.state
EOF

    echo "Updated .gitignore"
}
```

### 4.2 Project Type Detection

**Purpose:** Detect project type and suggest defaults

**Implementation:**

```bash
detect_project_type() {
    if [[ -f "package.json" ]]; then
        echo "nodejs"
    elif [[ -f "Cargo.toml" ]]; then
        echo "rust"
    elif [[ -f "go.mod" ]]; then
        echo "go"
    elif [[ -f "pyproject.toml" ]] || [[ -f "requirements.txt" ]]; then
        echo "python"
    elif [[ -f "Makefile" ]]; then
        echo "make"
    elif ls *.csproj 1>/dev/null 2>&1; then
        echo "dotnet"
    elif [[ -f "pom.xml" ]]; then
        echo "maven"
    elif [[ -f "build.gradle" ]] || [[ -f "build.gradle.kts" ]]; then
        echo "gradle"
    else
        echo "generic"
    fi
}

get_defaults_for_type() {
    local type="$1"
    case "$type" in
        nodejs)
            echo "FRESHER_TEST_CMD='npm test'"
            echo "FRESHER_BUILD_CMD='npm run build'"
            echo "FRESHER_SRC_DIR='src'"
            ;;
        rust)
            echo "FRESHER_TEST_CMD='cargo test'"
            echo "FRESHER_BUILD_CMD='cargo build'"
            echo "FRESHER_SRC_DIR='src'"
            ;;
        go)
            echo "FRESHER_TEST_CMD='go test ./...'"
            echo "FRESHER_BUILD_CMD='go build ./...'"
            echo "FRESHER_SRC_DIR='.'"
            ;;
        python)
            echo "FRESHER_TEST_CMD='pytest'"
            echo "FRESHER_BUILD_CMD='python -m build'"
            echo "FRESHER_SRC_DIR='src'"
            ;;
        *)
            echo "FRESHER_TEST_CMD='echo \"no tests configured\"'"
            echo "FRESHER_BUILD_CMD='echo \"no build configured\"'"
            echo "FRESHER_SRC_DIR='.'"
            ;;
    esac
}
```

### 4.3 CLAUDE.md Integration

**Purpose:** Add Specifications section to CLAUDE.md if missing

**Behavior:**

1. Check if CLAUDE.md exists
2. Check if `## Specifications` section exists
3. If missing: inject after first heading or at document start
4. Preserve all existing content

**Template to inject:**

```markdown
## Specifications

**IMPORTANT:** Before implementing any feature, consult `specs/README.md`.

- **Assume NOT implemented.** Specs describe intent; code describes reality.
- **Check the codebase first.** Search actual code before concluding.
- **Use specs as guidance.** Follow design patterns in relevant spec.
- **Spec index:** `specs/README.md` lists all specs by category.
```

---

## 5. Configuration

| Variable | Type | Description | Default |
|----------|------|-------------|---------|
| `FRESHER_GITIGNORE_ENTRIES` | array | Paths to add to .gitignore | `[".fresher/logs/", ".fresher/.state"]` |
| `FRESHER_SKIP_GITIGNORE` | boolean | Skip .gitignore modification | `false` |

---

## 6. Security Considerations

### File Permissions

- .gitignore modifications append-only (never delete existing entries)
- No modification of files outside .fresher/ except .gitignore
- Respect existing .gitignore permissions

### Validation

- Verify .gitignore is a regular file (not symlink to sensitive location)
- Validate project root before modifying any files

---

## 7. Implementation Phases

| Phase | Description | Dependencies | Complexity |
|-------|-------------|--------------|------------|
| 1 | Gitignore update function | None | Low |
| 2 | Project type detection | None | Low |
| 3 | CLAUDE.md integration | None | Low |
| 4 | Integrate all into install.sh | Phases 1-3 | Medium |

---

## 8. Open Questions

- [x] Should .fresher/ be gitignored by default? → No, only runtime artifacts
- [x] How to handle existing .gitignore entries? → Skip if already present (idempotent)
- [ ] Support .gitignore in subdirectories?
- [ ] Add uninstall command that cleans up .gitignore entries?
