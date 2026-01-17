#!/bin/bash
# Fresher configuration for fresher
# Generated: 2026-01-17T19:15:10Z
# Project type: nodejs

#──────────────────────────────────────────────────────────────────
# Mode Configuration
#──────────────────────────────────────────────────────────────────
export FRESHER_MODE="${FRESHER_MODE:-planning}"

#──────────────────────────────────────────────────────────────────
# Termination Settings
#──────────────────────────────────────────────────────────────────
export FRESHER_MAX_ITERATIONS="${FRESHER_MAX_ITERATIONS:-0}"
export FRESHER_SMART_TERMINATION="${FRESHER_SMART_TERMINATION:-true}"

#──────────────────────────────────────────────────────────────────
# Claude Code Settings
#──────────────────────────────────────────────────────────────────
export FRESHER_DANGEROUS_PERMISSIONS="${FRESHER_DANGEROUS_PERMISSIONS:-true}"
export FRESHER_MAX_TURNS="${FRESHER_MAX_TURNS:-50}"
export FRESHER_MODEL="${FRESHER_MODEL:-sonnet}"

#──────────────────────────────────────────────────────────────────
# Project Commands (detected: nodejs)
#──────────────────────────────────────────────────────────────────
export FRESHER_TEST_CMD="${FRESHER_TEST_CMD:-bun test}"
export FRESHER_BUILD_CMD="${FRESHER_BUILD_CMD:-bun run build}"
export FRESHER_LINT_CMD="${FRESHER_LINT_CMD:-bun run lint}"

#──────────────────────────────────────────────────────────────────
# Paths
#──────────────────────────────────────────────────────────────────
export FRESHER_LOG_DIR="${FRESHER_LOG_DIR:-.fresher/logs}"
export FRESHER_SPEC_DIR="${FRESHER_SPEC_DIR:-specs}"
export FRESHER_SRC_DIR="${FRESHER_SRC_DIR:-src}"

#──────────────────────────────────────────────────────────────────
# Hook Settings
#──────────────────────────────────────────────────────────────────
export FRESHER_HOOK_TIMEOUT="${FRESHER_HOOK_TIMEOUT:-30}"
export FRESHER_HOOKS_ENABLED="${FRESHER_HOOKS_ENABLED:-true}"

#──────────────────────────────────────────────────────────────────
# Docker (optional)
#──────────────────────────────────────────────────────────────────
export FRESHER_USE_DOCKER="${FRESHER_USE_DOCKER:-false}"
export FRESHER_DOCKER_IMAGE="${FRESHER_DOCKER_IMAGE:-fresher:local}"
